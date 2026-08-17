use crate::types::{
    BackendType, ChatRequest, ChatResponse, GenerationOptions, GenerationRequest, GenerationResponse,
    LocalModel, Message, ModelDetails,
};
use anyhow::Result;
use futures::{StreamExt, TryStreamExt};
use reqwest::Client;
use std::time::Duration;
use tracing::{debug, info, warn};
use url::Url;

const DEFAULT_LLAMACPP_URL: &str = "http://localhost:8080";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone)]
pub struct LlamaCppClient {
    client: Client,
    base_url: Url,
}

impl LlamaCppClient {
    pub fn new(base_url: Option<String>) -> Result<Self> {
        let url = base_url.unwrap_or_else(|| DEFAULT_LLAMACPP_URL.to_string());
        let base_url = Url::parse(&url)?;
        
        let client = Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .build()?;
        
        Ok(Self { client, base_url })
    }

    pub async fn is_available(&self) -> bool {
        self.client
            .get(self.base_url.join("/health").unwrap())
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub async fn get_models(&self) -> Result<Vec<LocalModel>> {
        let url = self.base_url.join("/v1/models")?;
        let response: serde_json::Value = self.client.get(url).send().await?.json().await?;
        
        let models = response["data"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|m| self.parse_model(m)).collect())
            .unwrap_or_default();
        
        Ok(models)
    }

    fn parse_model(&self, value: &serde_json::Value) -> Option<LocalModel> {
        let name = value["id"].as_str()?.to_string();
        let owned_by = value["owned_by"].as_str().unwrap_or("llamacpp").to_string();
        
        let details = ModelDetails {
            format: "gguf".to_string(),
            family: owned_by.clone(),
            families: Some(vec![owned_by]),
            parameter_size: "".to_string(),
            quantization_level: "".to_string(),
            parent_model: None,
        };
        
        Some(LocalModel {
            name: name.clone(),
            size: "Unknown".to_string(),
            digest: "".to_string(),
            details,
            modified_at: chrono::Utc::now(),
            backend: BackendType::LlamaCpp,
        })
    }

    pub async fn generate(&self, request: GenerationRequest) -> Result<GenerationResponse> {
        let url = self.base_url.join("/v1/completions")?;
        
        let payload = serde_json::json!({
            "model": request.model,
            "prompt": request.prompt,
            "stream": request.stream,
            "temperature": request.options.as_ref().and_then(|o| o.temperature).unwrap_or(0.7),
            "top_p": request.options.as_ref().and_then(|o| o.top_p).unwrap_or(0.9),
            "max_tokens": request.options.as_ref().and_then(|o| o.num_predict).unwrap_or(-1),
            "stop": request.options.as_ref().and_then(|o| o.stop.clone()),
        });
        
        let response = self.client.post(url).json(&payload).send().await?;
        let result: serde_json::Value = response.json().await?;
        
        let text = result["choices"][0]["text"].as_str().unwrap_or("").to_string();
        
        Ok(GenerationResponse {
            model: request.model,
            created_at: chrono::Utc::now(),
            response: text,
            done: true,
            context: None,
            total_duration: None,
            load_duration: None,
            prompt_eval_count: None,
            prompt_eval_duration: None,
            eval_count: None,
            eval_duration: None,
        })
    }

    pub async fn generate_stream(
        &self,
        request: GenerationRequest,
    ) -> Result<impl futures::Stream<Item = Result<GenerationResponse>>> {
        let url = self.base_url.join("/v1/completions")?;
        let mut req = request;
        req.stream = true;
        
        let payload = serde_json::json!({
            "model": req.model,
            "prompt": req.prompt,
            "stream": true,
            "temperature": req.options.as_ref().and_then(|o| o.temperature).unwrap_or(0.7),
            "top_p": req.options.as_ref().and_then(|o| o.top_p).unwrap_or(0.9),
            "max_tokens": req.options.as_ref().and_then(|o| o.num_predict).unwrap_or(-1),
            "stop": req.options.as_ref().and_then(|o| o.stop.clone()),
        });
        
        let response = self.client.post(url).json(&payload).send().await?;
        let stream = response.bytes_stream();
        
        let stream = stream
            .map_err(anyhow::Error::from)
            .and_then(move |chunk| {
                let model_name = req.model.clone();
                async move {
                    let text = std::str::from_utf8(&chunk)?;
                    let mut results = Vec::new();
                    for line in text.lines() {
                        if line.trim().is_empty() { continue; }
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if data == "[DONE]" { continue; }
                            if let Ok(resp) = serde_json::from_str::<serde_json::Value>(data) {
                                if let Some(text) = resp["choices"][0]["text"].as_str() {
                                    results.push(GenerationResponse {
                                        model: model_name.clone(),
                                        created_at: chrono::Utc::now(),
                                        response: text.to_string(),
                                        done: !resp["choices"][0]["finish_reason"].is_null(),
                                        context: None,
                                        total_duration: None,
                                        load_duration: None,
                                        prompt_eval_count: None,
                                        prompt_eval_duration: None,
                                        eval_count: None,
                                        eval_duration: None,
                                    });
                                }
                            }
                        }
                    }
                    Ok(futures::stream::iter(results.into_iter().map(Ok)))
                }
            })
            .try_flatten();
        
        Ok(stream)
    }

    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = self.base_url.join("/v1/chat/completions")?;
        
        let messages: Vec<serde_json::Value> = request.messages.iter().map(|m| {
            serde_json::json!({
                "role": format!("{:?}", m.role).to_lowercase(),
                "content": m.content,
            })
        }).collect();
        
        let payload = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "stream": request.stream,
            "temperature": request.options.as_ref().and_then(|o| o.temperature).unwrap_or(0.7),
            "top_p": request.options.as_ref().and_then(|o| o.top_p).unwrap_or(0.9),
            "max_tokens": request.options.as_ref().and_then(|o| o.num_predict).unwrap_or(-1),
        });
        
        let response = self.client.post(url).json(&payload).send().await?;
        let result: serde_json::Value = response.json().await?;
        
        let content = result["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
        
        Ok(ChatResponse {
            model: request.model,
            created_at: chrono::Utc::now(),
            message: Message {
                role: crate::types::MessageRole::Assistant,
                content,
                images: None,
                tool_calls: None,
                tool_call_id: None,
            },
            done: true,
            total_duration: None,
            load_duration: None,
            prompt_eval_count: None,
            prompt_eval_duration: None,
            eval_count: None,
            eval_duration: None,
        })
    }

    pub async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<impl futures::Stream<Item = Result<ChatResponse>>> {
        let url = self.base_url.join("/v1/chat/completions")?;
        let mut req = request;
        req.stream = true;
        
        let messages: Vec<serde_json::Value> = req.messages.iter().map(|m| {
            serde_json::json!({
                "role": format!("{:?}", m.role).to_lowercase(),
                "content": m.content,
            })
        }).collect();
        
        let payload = serde_json::json!({
            "model": req.model,
            "messages": messages,
            "stream": true,
            "temperature": req.options.as_ref().and_then(|o| o.temperature).unwrap_or(0.7),
            "top_p": req.options.as_ref().and_then(|o| o.top_p).unwrap_or(0.9),
            "max_tokens": req.options.as_ref().and_then(|o| o.num_predict).unwrap_or(-1),
        });
        
        let response = self.client.post(url).json(&payload).send().await?;
        let stream = response.bytes_stream();
        
        let stream = stream
            .map_err(anyhow::Error::from)
            .and_then(move |chunk| {
                let model_name = req.model.clone();
                async move {
                    let text = std::str::from_utf8(&chunk)?;
                    let mut results = Vec::new();
                    for line in text.lines() {
                        if line.trim().is_empty() { continue; }
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if data == "[DONE]" { continue; }
                            if let Ok(resp) = serde_json::from_str::<serde_json::Value>(data) {
                                if let Some(content) = resp["choices"][0]["delta"]["content"].as_str() {
                                    results.push(ChatResponse {
                                        model: model_name.clone(),
                                        created_at: chrono::Utc::now(),
                                        message: Message {
                                            role: crate::types::MessageRole::Assistant,
                                            content: content.to_string(),
                                            images: None,
                                            tool_calls: None,
                                            tool_call_id: None,
                                        },
                                        done: !resp["choices"][0]["finish_reason"].is_null(),
                                        total_duration: None,
                                        load_duration: None,
                                        prompt_eval_count: None,
                                        prompt_eval_duration: None,
                                        eval_count: None,
                                        eval_duration: None,
                                    });
                                }
                            }
                        }
                    }
                    Ok(futures::stream::iter(results.into_iter().map(Ok)))
                }
            })
            .try_flatten();
        
        Ok(stream)
    }

    pub async fn get_embeddings(&self, model: &str, input: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let url = self.base_url.join("/v1/embeddings")?;
        
        let payload = serde_json::json!({
            "model": model,
            "input": input,
        });
        
        let response = self.client.post(url).json(&payload).send().await?;
        let result: serde_json::Value = response.json().await?;
        
        let embeddings = result["data"]
            .as_array()
            .map(|arr| {
                arr.iter().filter_map(|e| {
                    e["embedding"].as_array().map(|v| {
                        v.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect()
                    })
                }).collect()
            })
            .unwrap_or_default();
        
        Ok(embeddings)
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.client = Client::builder().timeout(timeout).build().unwrap();
    }
}