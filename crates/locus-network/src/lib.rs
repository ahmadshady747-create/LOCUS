mod discovery;
mod load_balancer;
mod protocol;
pub mod types;

pub use discovery::{create_local_device, Discovery};
pub use load_balancer::{LoadBalancer, LocalFallback};
pub use protocol::{NetworkProtocol, ProtocolMessage, connect_to_peer, start_websocket_server};
pub use types::{
    DeviceAnnouncement, DeviceCapabilities, DeviceId, DeviceStatus, DeviceType,
    LocalDevice, ModelInfo, NetworkMessage, NetworkTask, Specialization,
    TaskPriority, TaskResult, TaskType,
};

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::info;

pub struct NetworkOrchestrator {
    local_device: Arc<RwLock<LocalDevice>>,
    discovery: Arc<RwLock<Option<Discovery>>>,
    load_balancer: Arc<RwLock<LoadBalancer>>,
    protocol: Arc<RwLock<Option<NetworkProtocol>>>,
    running: Arc<RwLock<bool>>,
}

impl NetworkOrchestrator {
    pub async fn new(
        name: Option<String>,
        device_type: DeviceType,
        capabilities: DeviceCapabilities,
        port: u16,
    ) -> Result<Self> {
        let local_device = create_local_device(name, device_type, capabilities, port).await;
        let device_id = local_device.id.clone();
        
        let orchestrator = Self {
            local_device: Arc::new(RwLock::new(local_device)),
            discovery: Arc::new(RwLock::new(None)),
            load_balancer: Arc::new(RwLock::new(LoadBalancer::new())),
            protocol: Arc::new(RwLock::new(Some(NetworkProtocol::new(device_id)))),
            running: Arc::new(RwLock::new(false)),
        };
        
        Ok(orchestrator)
    }

    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Ok(());
        }
        *running = true;
        drop(running);

        let device = self.local_device.read().await.clone();
        let discovery = Discovery::new(device.clone()).await?;
        
        discovery.start_advertising().await?;
        discovery.start_discovery().await?;
        
        let event_rx = discovery.event_receiver();
        let load_balancer = self.load_balancer.clone();
        let local_device = self.local_device.clone();
        
        tokio::spawn(async move {
            let mut rx = event_rx.lock().await;
            while let Some(msg) = rx.recv().await {
                match msg {
                    crate::types::NetworkMessage::Announce(announcement) => {
                        let mut lb = load_balancer.write().await;
                        lb.update_devices(vec![announcement.device]);
                    }
                    crate::types::NetworkMessage::Goodbye { device_id } => {
                        let mut lb = load_balancer.write().await;
                        lb.remove_device(&device_id);
                    }
                    _ => {}
                }
            }
        });

        let discovery_ref = self.discovery.clone();
        let local_device_clone = self.local_device.clone();
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(30));
            loop {
                ticker.tick().await;
                
                let mut device = local_device_clone.write().await;
                device.last_seen = chrono::Utc::now();
                device.status = DeviceStatus::Online;
                drop(device);

                let disc_guard = discovery_ref.read().await;
                if let Some(ref d) = *disc_guard {
                    let _ = d.start_advertising().await;
                }
            }
        });

        *self.discovery.write().await = Some(discovery);
        
        info!("Network orchestrator started for device: {}", device.name);
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }
        *running = false;
        drop(running);

        if let Some(discovery) = self.discovery.write().await.take() {
            discovery.stop_advertising().await?;
        }
        
        info!("Network orchestrator stopped");
        Ok(())
    }

    pub async fn discover_devices(&self) -> Vec<LocalDevice> {
        if let Some(discovery) = self.discovery.read().await.as_ref() {
            discovery.get_discovered_devices().await
        } else {
            vec![]
        }
    }

    pub async fn get_local_device(&self) -> LocalDevice {
        self.local_device.read().await.clone()
    }

    pub async fn get_device_capabilities(&self) -> DeviceCapabilities {
        self.local_device.read().await.capabilities.clone()
    }

    pub async fn assign_task(&self, task: NetworkTask) -> Result<TaskResult> {
        let device_id = {
            let mut lb = self.load_balancer.write().await;
            lb.update_devices(self.discover_devices().await);
            
            let selected_id = lb.select_device(&task).cloned();
            if let Some(device_id) = selected_id {
                lb.assign_task(task.clone(), &device_id)?;
                device_id
            } else {
                return self.execute_locally(task).await;
            }
        };
        
        let proto_guard = self.protocol.read().await;
        if let Some(ref proto) = *proto_guard {
            proto.send_task_request(task, &device_id.0.to_string()).await
        } else {
            self.execute_locally(task).await
        }
    }

    async fn execute_locally(&self, task: NetworkTask) -> Result<TaskResult> {
        let device = self.get_local_device().await;
        let fallback = LocalFallback::new(device);
        fallback.execute_locally(task).await
    }

    pub async fn update_capabilities(&self, capabilities: DeviceCapabilities) {
        let mut device = self.local_device.write().await;
        device.capabilities = capabilities;
        
        if let Some(discovery) = self.discovery.read().await.as_ref() {
            let _ = discovery.restart_advertising().await;
        }
    }

    pub async fn get_load_balancer_state(&self) -> Vec<(DeviceId, f32)> {
        let lb = self.load_balancer.read().await;
        lb.get_device_ids().into_iter().map(|id| {
            let load = lb.get_device_load(&id);
            (id, load)
        }).collect()
    }

    pub fn is_running(&self) -> bool {
        // This is a simplified check - in reality would need async
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use crate::types::{NetworkTask, TaskType, TaskPriority, DeviceCapabilities, DeviceType};

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let orchestrator = NetworkOrchestrator::new(
            Some("test-node".to_string()),
            DeviceType::Main,
            DeviceCapabilities::default(),
            8080,
        ).await;
        
        assert!(orchestrator.is_ok());
    }

    #[tokio::test]
    async fn test_load_balancer_selection() {
        let mut lb = LoadBalancer::new();
        
        let device = LocalDevice {
            id: DeviceId::new(),
            name: "test-worker".to_string(),
            hostname: "localhost".to_string(),
            ip_address: "127.0.0.1".to_string(),
            port: 8080,
            capabilities: DeviceCapabilities {
                specializations: vec![Specialization::CodeGeneration],
                performance_score: 1.5,
                ..Default::default()
            },
            last_seen: chrono::Utc::now(),
            status: DeviceStatus::Online,
            device_type: DeviceType::Worker,
        };
        
        lb.update_devices(vec![device]);
        
        let task = NetworkTask {
            id: Uuid::new_v4(),
            task_type: TaskType::GenerateCode,
            payload: serde_json::json!({}),
            priority: TaskPriority::Normal,
            required_capabilities: DeviceCapabilities {
                specializations: vec![Specialization::CodeGeneration],
                ..Default::default()
            },
            created_at: chrono::Utc::now(),
            timeout_seconds: 60,
        };
        
        let selected = lb.select_device(&task);
        assert!(selected.is_some());
    }
}