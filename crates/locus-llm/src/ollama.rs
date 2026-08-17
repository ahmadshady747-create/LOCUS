use crate::types::{
    BackendType, ChatRequest, ChatResponse, GenerationOptions, GenerationRequest, GenerationResponse, LocalModel, Message,
    ModelDetails, ModelInfo, Tool,
};
use anyhow::Result;
use futures::{StreamExt, TryStreamExt};
use reqwest::Client;
use std::time::Duration;
use tracing::{debug, info, warn};
use url::Url;

const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone)]
pub struct OllamaClient {
    client: Client,
    base_url: Url,
}

impl OllamaClient {
    pub fn new(base_url: Option<String>) -> Result<Self> {
        let url = base_url.unwrap_or_else(|| DEFAULT_OLLAMA_URL.to_string());
        let base_url = Url::parse(&url)?;
        
        let client = Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .build()?;
        
        Ok(Self { client, base_url })
    }

    pub async fn is_available(&self) -> bool {
        self.client
            .get(self.base_url.join("/api/version").unwrap())
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub async fn list_models(&self) -> Result<Vec<LocalModel>> {
        let url = self.base_url.join("/api/tags")?;
        let response: serde_json::Value = self.client.get(url).send().await?.json().await?;
        
        let models = response["models"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|m| self.parse_model(m)).collect())
            .unwrap_or_default();
        
        Ok(models)
    }

    fn parse_model(&self, value: &serde_json::Value) -> Option<LocalModel> {
        let name = value["name"].as_str()?.to_string();
        let size = value["size"].as_u64()?;
        let digest = value["digest"].as_str()?.to_string();
        let modified_at = value["modified_at"].as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);
        
        let details = ModelDetails {
            format: value["details"]["format"].as_str().unwrap_or("").to_string(),
            family: value["details"]["family"].as_str().unwrap_or("").to_string(),
            families: value["details"]["families"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()),
            parameter_size: value["details"]["parameter_size"].as_str().unwrap_or("").to_string(),
            quantization_level: value["details"]["quantization_level"].as_str().unwrap_or("").to_string(),
            parent_model: value["details"]["parent_model"].as_str().map(|s| s.to_string()),
        };
        
        Some(LocalModel {
            name: name.clone(),
            size: Self::format_size(size),
            digest,
            details,
            modified_at,
            backend: BackendType::Ollama,
        })
    }

    fn format_size(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_idx = 0;
        while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
            size /= 1024.0;
            unit_idx += 1;
        }
        format!("{:.1} {}", size, UNITS[unit_idx])
    }

    pub async fn pull_model(&self, name: &str, stream: bool) -> Result<()> {
        let url = self.base_url.join("/api/pull")?;
        let payload = serde_json::json!({
            "name": name,
            "stream": stream
        });
        
        let response = self.client.post(url).json(&payload).send().await?;
        
        if stream {
            let mut stream = response.bytes_stream();
            while let Some(chunk) = futures::StreamExt::next(&mut stream).await {
                let chunk = chunk?;
                if let Ok(text) = std::str::from_utf8(&chunk) {
                    for line in text.lines() {
                        if line.trim().is_empty() { continue; }
                        debug!("Pull progress: {}", line);
                    }
                }
            }
        }
        
        info!("Model {} pulled successfully", name);
        Ok(())
    }

    pub async fn delete_model(&self, name: &str) -> Result<()> {
        let url = self.base_url.join("/api/delete")?;
        let payload = serde_json::json!({ "name": name });
        self.client.delete(url).json(&payload).send().await?;
        info!("Model {} deleted", name);
        Ok(())
    }

    pub async fn generate(&self, request: GenerationRequest) -> Result<GenerationResponse> {
        let url = self.base_url.join("/api/generate")?;
        let response = self.client.post(url).json(&request).send().await?;
        let result = response.json().await?;
        Ok(result)
    }

    pub async fn generate_stream(
        &self,
        request: GenerationRequest,
    ) -> Result<impl futures::Stream<Item = Result<GenerationResponse>>> {
        let url = self.base_url.join("/api/generate")?;
        let mut req = request;
        req.stream = true;
        
        let response = self.client.post(url).json(&req).send().await?;
        let stream = response.bytes_stream();
        
        let stream = stream
            .map_err(anyhow::Error::from)
            .and_then(|chunk| async move {
                let text = std::str::from_utf8(&chunk)?;
                let mut results = Vec::new();
                for line in text.lines() {
                    if line.trim().is_empty() { continue; }
                    let resp: GenerationResponse = serde_json::from_str(line)?;
                    results.push(resp);
                }
                Ok(futures::stream::iter(results.into_iter().map(Ok)))
            })
            .try_flatten();
        
        Ok(stream)
    }

    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = self.base_url.join("/api/chat")?;
        let response = self.client.post(url).json(&request).send().await?;
        let result = response.json().await?;
        Ok(result)
    }

    pub async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<impl futures::Stream<Item = Result<ChatResponse>>> {
        let url = self.base_url.join("/api/chat")?;
        let mut req = request;
        req.stream = true;
        
        let response = self.client.post(url).json(&req).send().await?;
        let stream = response.bytes_stream();
        
        let stream = stream
            .map_err(anyhow::Error::from)
            .and_then(|chunk| async move {
                let text = std::str::from_utf8(&chunk)?;
                let mut results = Vec::new();
                for line in text.lines() {
                    if line.trim().is_empty() { continue; }
                    let resp: ChatResponse = serde_json::from_str(line)?;
                    results.push(resp);
                }
                Ok(futures::stream::iter(results.into_iter().map(Ok)))
            })
            .try_flatten();
        
        Ok(stream)
    }

    pub async fn get_model_info(&self, name: &str) -> Result<ModelInfo> {
        let url = self.base_url.join("/api/show")?;
        let payload = serde_json::json!({ "name": name });
        let response = self.client.post(url).json(&payload).send().await?;
        let result = response.json().await?;
        Ok(result)
    }

    pub async fn create_model(&self, name: &str, modelfile: &str) -> Result<()> {
        let url = self.base_url.join("/api/create")?;
        let payload = serde_json::json!({
            "name": name,
            "modelfile": modelfile
        });
        self.client.post(url).json(&payload).send().await?;
        info!("Model {} created", name);
        Ok(())
    }

    pub async fn copy_model(&self, source: &str, destination: &str) -> Result<()> {
        let url = self.base_url.join("/api/copy")?;
        let payload = serde_json::json!({
            "source": source,
            "destination": destination
        });
        self.client.post(url).json(&payload).send().await?;
        info!("Model {} copied to {}", source, destination);
        Ok(())
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.client = Client::builder().timeout(timeout).build().unwrap();
    }
}
