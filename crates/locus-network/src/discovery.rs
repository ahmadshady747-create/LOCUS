use crate::types::{DeviceAnnouncement, DeviceId, DeviceStatus, LocalDevice, NetworkMessage};
use anyhow::Result;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use tracing::{debug, info, warn, error};
use uuid::Uuid;

const SERVICE_TYPE: &str = "_locus-llm._tcp.local.";
const SERVICE_NAME_PREFIX: &str = "locus";

pub struct Discovery {
    daemon: ServiceDaemon,
    local_device: Arc<RwLock<LocalDevice>>,
    discovered_devices: Arc<RwLock<HashMap<DeviceId, LocalDevice>>>,
    event_tx: mpsc::UnboundedSender<NetworkMessage>,
    event_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<NetworkMessage>>>,
}

impl Discovery {
    pub async fn new(local_device: LocalDevice) -> Result<Self> {
        let daemon = ServiceDaemon::new()?;
        
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        let discovery = Self {
            daemon,
            local_device: Arc::new(RwLock::new(local_device)),
            discovered_devices: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx: Arc::new(tokio::sync::Mutex::new(event_rx)),
        };

        Ok(discovery)
    }

    pub fn event_receiver(&self) -> Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<NetworkMessage>>> {
        self.event_rx.clone()
    }

    pub async fn start_advertising(&self) -> Result<()> {
        let device = self.local_device.read().await.clone();
        
        let service_info = self.create_service_info(&device)?;
        
        self.daemon.register(service_info)?;
        
        info!("Started advertising service: {}", device.name);
        Ok(())
    }

    pub async fn stop_advertising(&self) -> Result<()> {
        let device = self.local_device.read().await.clone();
        let full_name = format!("{}._locus-llm._tcp.local.", device.name);
        self.daemon.unregister(&full_name)?;
        info!("Stopped advertising service: {}", device.name);
        Ok(())
    }

    pub async fn start_discovery(&self) -> Result<()> {
        let receiver = self.daemon.browse(SERVICE_TYPE)?;
        
        let discovered = self.discovered_devices.clone();
        let local_device = self.local_device.clone();
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            while let Ok(event) = receiver.recv_async().await {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        if let Some(device) = Self::parse_service_info(info) {
                            let local = local_device.read().await;
                            if device.id != local.id {
                                let mut devices = discovered.write().await;
                                devices.insert(device.id.clone(), device.clone());
                                
                                let _ = event_tx.send(NetworkMessage::Announce(DeviceAnnouncement {
                                    device: device.clone(),
                                    timestamp: chrono::Utc::now(),
                                }));
                                
                                info!("Discovered device: {} ({})", device.name, device.ip_address);
                            }
                        }
                    }
                    ServiceEvent::ServiceRemoved(_, name) => {
                        let mut devices = discovered.write().await;
                        let to_remove: Vec<_> = devices
                            .iter()
                            .filter(|(_, d)| format!("{}._locus-llm._tcp.local.", d.name) == name)
                            .map(|(id, _)| id.clone())
                            .collect();
                        
                        for id in to_remove {
                            if let Some(device) = devices.remove(&id) {
                                info!("Device removed: {}", device.name);
                                let _ = event_tx.send(NetworkMessage::Goodbye { device_id: device.id });
                            }
                        }
                    }
                    _ => {}
                }
            }
        });

        info!("Started mDNS discovery");
        Ok(())
    }

    pub async fn get_discovered_devices(&self) -> Vec<LocalDevice> {
        self.discovered_devices.read().await.values().cloned().collect()
    }

    pub async fn get_local_device(&self) -> LocalDevice {
        self.local_device.read().await.clone()
    }

    pub async fn update_local_device<F>(&self, f: F) 
    where
        F: FnOnce(&mut LocalDevice),
    {
        let mut device = self.local_device.write().await;
        f(&mut device);
        
        if let Err(e) = self.restart_advertising().await {
            error!("Failed to restart advertising: {}", e);
        }
    }

    pub async fn restart_advertising(&self) -> Result<()> {
        self.stop_advertising().await?;
        self.start_advertising().await?;
        Ok(())
    }

    fn create_service_info(&self, device: &LocalDevice) -> Result<ServiceInfo> {
        let hostname = hostname::get()
            .ok()
            .and_then(|h| h.to_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "localhost".to_string());
        
        let ips: Vec<IpAddr> = if_addrs::get_if_addrs()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|iface| if !iface.is_loopback() && iface.addr.ip().is_ipv4() { Some(iface.addr.ip()) } else { None })
            .collect();
        
        let port = device.port;
        let full_name = format!("{}._locus-llm._tcp.local.", device.name);
        
        let mut txt_props = HashMap::new();
        txt_props.insert("id", device.id.0.to_string());
        txt_props.insert("type", format!("{:?}", device.device_type));
        txt_props.insert("models", device.capabilities.models.iter().map(|m| m.name.clone()).collect::<Vec<_>>().join(","));
        txt_props.insert("vram", device.capabilities.vram_gb.map(|v| v.to_string()).unwrap_or("0".to_string()));
        txt_props.insert("ctx", device.capabilities.max_context_tokens.to_string());
        txt_props.insert("specializations", device.capabilities.specializations.iter().map(|s| format!("{:?}", s)).collect::<Vec<_>>().join(","));
        txt_props.insert("score", device.capabilities.performance_score.to_string());

        ServiceInfo::new(
            SERVICE_TYPE,
            &device.name,
            &full_name,
            &hostname,
            port,
            txt_props.into_iter().map(|(k, v)| (k.to_string(), v)).collect::<std::collections::HashMap<String, String>>(),
            
        ).map_err(|e| anyhow::anyhow!("Failed to create service info: {}", e))
    }

    fn parse_service_info(info: ServiceInfo) -> Option<LocalDevice> {
        let properties = info.get_properties();
        
        let id_str = properties.get("id")?.val_str();
        let id = Uuid::parse_str(id_str).ok().map(DeviceId)?;
        
        let device_type_str = properties.get("type")?.val_str();
        let device_type = match device_type_str {
            "Main" => crate::types::DeviceType::Main,
            "Worker" => crate::types::DeviceType::Worker,
            _ => crate::types::DeviceType::Hybrid,
        };
        
        let models_str = properties.get("models")?.val_str();
        let models: Vec<crate::types::ModelInfo> = models_str
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|name| crate::types::ModelInfo {
                name: name.to_string(),
                quantization: "q4_k_m".to_string(),
                context_window: 4096,
                parameter_count: "7B".to_string(),
                size_gb: 4.0,
            })
            .collect();
        
        let vram_gb = properties.get("vram")?.val_str().parse::<f32>().ok();
        let max_context = properties.get("ctx")?.val_str().parse::<usize>().ok()?;
        let score = properties.get("score")?.val_str().parse::<f32>().ok().unwrap_or(1.0);
        
        let ip = info.get_addresses_v4().iter().next().cloned()?;
        let port = info.get_port();
        
        Some(LocalDevice {
            id,
            name: info.get_fullname().trim_end_matches("._locus-llm._tcp.local.").to_string(),
            hostname: info.get_hostname().trim_end_matches('.').to_string(),
            ip_address: ip.to_string(),
            port,
            capabilities: crate::types::DeviceCapabilities {
                models,
                max_context_tokens: max_context,
                vram_gb,
                quantization: vec!["q4_k_m".to_string()],
                cpu_cores: 8,
                memory_gb: 16.0,
                supports_gpu: vram_gb.is_some(),
                specializations: vec![crate::types::Specialization::CodeGeneration],
                performance_score: score,
            },
            last_seen: chrono::Utc::now(),
            status: DeviceStatus::Online,
            device_type,
        })
    }
}

pub async fn create_local_device(
    name: Option<String>,
    device_type: crate::types::DeviceType,
    capabilities: crate::types::DeviceCapabilities,
    port: u16,
) -> LocalDevice {
    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "localhost".to_string());
    
    let ips = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|iface| if !iface.is_loopback() && iface.addr.ip().is_ipv4() { Some(iface.addr.ip()) } else { None })
        .collect::<Vec<_>>();
    
    let ip = ips.first().cloned().unwrap_or(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
    
    LocalDevice {
        id: DeviceId::new(),
        name: name.unwrap_or_else(|| format!("{}-{}", SERVICE_NAME_PREFIX, Uuid::new_v4().to_string().split('-').next().unwrap())),
        hostname,
        ip_address: ip.to_string(),
        port,
        capabilities,
        last_seen: chrono::Utc::now(),
        status: DeviceStatus::Online,
        device_type,
    }
}






