use crate::types::{NetworkMessage, NetworkTask, TaskResult, DeviceId};
use anyhow::Result;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::{accept_async, connect_async, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, info, warn, error};
use uuid::Uuid;

const PROTOCOL_VERSION: u8 = 1;
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const MESSAGE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMessage {
    pub version: u8,
    pub message_id: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub payload: NetworkMessage,
}

impl ProtocolMessage {
    pub fn new(payload: NetworkMessage) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            message_id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            payload,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(Into::into)
    }

    pub fn decode(data: &[u8]) -> Result<Self> {
        serde_json::from_slice(data).map_err(Into::into)
    }
}

pub struct NetworkProtocol {
    local_device_id: DeviceId,
    outbound_tx: mpsc::UnboundedSender<ProtocolMessage>,
    inbound_rx: mpsc::UnboundedReceiver<ProtocolMessage>,
    pending_requests: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<Uuid, tokio::sync::oneshot::Sender<TaskResult>>>>,
}

impl NetworkProtocol {
    pub fn new(local_device_id: DeviceId) -> Self {
        let (outbound_tx, _outbound_rx) = mpsc::unbounded_channel();
        let (_inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        
        Self {
            local_device_id,
            outbound_tx,
            inbound_rx,
            pending_requests: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub fn get_sender(&self) -> mpsc::UnboundedSender<ProtocolMessage> {
        self.outbound_tx.clone()
    }

    pub fn get_receiver(&mut self) -> mpsc::UnboundedReceiver<ProtocolMessage> {
        std::mem::replace(&mut self.inbound_rx, mpsc::unbounded_channel().1)
    }

    pub async fn send_task_request(&self, task: NetworkTask, target: &str) -> Result<TaskResult> {
        let message = ProtocolMessage::new(NetworkMessage::TaskRequest(task.clone()));
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        
        self.pending_requests.write().await.insert(message.message_id, response_tx);
        
        if let Err(e) = self.outbound_tx.send(message) {
            error!("Failed to send task request: {}", e);
            return Err(anyhow::anyhow!("Failed to send task request"));
        }
        
        match tokio::time::timeout(MESSAGE_TIMEOUT * 10, response_rx).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(_)) => Err(anyhow::anyhow!("Response channel closed")),
            Err(_) => Err(anyhow::anyhow!("Task request timed out")),
        }
    }

    pub async fn send_task_response(&self, task_id: Uuid, result: TaskResult) -> Result<()> {
        let message = ProtocolMessage::new(NetworkMessage::TaskResponse(result));
        self.outbound_tx.send(message)?;
        Ok(())
    }

    pub async fn send_heartbeat(&self, status: crate::types::DeviceStatus) -> Result<()> {
        let message = ProtocolMessage::new(NetworkMessage::Heartbeat {
            device_id: self.local_device_id.clone(),
            status,
        });
        self.outbound_tx.send(message)?;
        Ok(())
    }

    pub async fn handle_incoming(&self, message: ProtocolMessage) -> Result<Option<ProtocolMessage>> {
        match &message.payload {
            NetworkMessage::TaskRequest(task) => {
                info!("Received task request: {} ({:?})", task.id, task.task_type);
                Ok(None)
            }
            NetworkMessage::TaskResponse(result) => {
                debug!("Received task response for: {}", result.task_id);
                if let Some(sender) = self.pending_requests.write().await.remove(&message.message_id) {
                    let _ = sender.send(result.clone());
                }
                Ok(None)
            }
            NetworkMessage::Heartbeat { device_id, status } => {
                debug!("Heartbeat from {}: {:?}", device_id, status);
                Ok(None)
            }
            NetworkMessage::Announce(announcement) => {
                info!("Device announced: {}", announcement.device.name);
                Ok(None)
            }
            NetworkMessage::Goodbye { device_id } => {
                info!("Device left: {}", device_id);
                Ok(None)
            }
            NetworkMessage::DeviceListRequest => {
                let response = ProtocolMessage::new(NetworkMessage::DeviceListResponse(vec![]));
                Ok(Some(response))
            }
            NetworkMessage::DeviceListResponse(_) => {
                Ok(None)
            }
        }
    }
}

pub async fn start_websocket_server(
    addr: &str,
    handler: impl Fn(ProtocolMessage) -> Result<Option<ProtocolMessage>> + Send + Sync + 'static,
) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!("WebSocket server listening on {}", addr);
    
    let handler = std::sync::Arc::new(handler);
    
    while let Ok((stream, addr)) = listener.accept().await {
        let handler = handler.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, addr, handler).await {
                error!("Connection error from {}: {}", addr, e);
            }
        });
    }
    
    Ok(())
}

async fn handle_connection(
    stream: TcpStream,
    addr: std::net::SocketAddr,
    handler: std::sync::Arc<dyn Fn(ProtocolMessage) -> Result<Option<ProtocolMessage>> + Send + Sync>,
) -> Result<()> {
    let ws_stream = accept_async(stream).await?;
    let (mut write, mut read) = ws_stream.split();
    
    let (tx, mut rx) = mpsc::unbounded_channel::<ProtocolMessage>();
    
    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let data = msg.encode()?;
            write.send(Message::Binary(data.into())).await?;
        }
        Ok::<_, anyhow::Error>(())
    });
    
    let read_task = tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    if let Ok(protocol_msg) = ProtocolMessage::decode(&data) {
                        if let Ok(Some(response)) = handler(protocol_msg) {
                            if let Err(e) = tx.send(response) {
                                error!("Failed to send response: {}", e);
                                break;
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) => break,
                Ok(Message::Ping(data)) => {
                    if let Err(e) = tx.send(ProtocolMessage::new(crate::types::NetworkMessage::Heartbeat {
                        device_id: crate::types::DeviceId::new(),
                        status: crate::types::DeviceStatus::Online,
                    })) {
                        break;
                    }
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        Ok::<_, anyhow::Error>(())
    });
    
    tokio::select! {
        result = write_task => result??,
        result = read_task => result??,
    }
    
    Ok(())
}

pub async fn connect_to_peer(addr: &str) -> Result<WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>> {
    let (ws_stream, _) = connect_async(addr).await?;
    info!("Connected to peer: {}", addr);
    Ok(ws_stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{NetworkTask, TaskType, TaskPriority, DeviceCapabilities};

    #[test]
    fn test_protocol_message_encoding() {
        let task = NetworkTask {
            id: Uuid::new_v4(),
            task_type: TaskType::GenerateCode,
            payload: serde_json::json!({"prompt": "test"}),
            priority: TaskPriority::Normal,
            required_capabilities: DeviceCapabilities::default(),
            created_at: chrono::Utc::now(),
            timeout_seconds: 60,
        };
        
        let msg = ProtocolMessage::new(NetworkMessage::TaskRequest(task));
        let encoded = msg.encode().unwrap();
        let decoded = ProtocolMessage::decode(&encoded).unwrap();
        
        assert_eq!(msg.version, decoded.version);
        assert_eq!(msg.message_id, decoded.message_id);
    }
}