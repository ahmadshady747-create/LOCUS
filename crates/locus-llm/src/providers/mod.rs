pub mod gemini;
pub mod groq;
pub mod local_ollama;
pub mod openrouter;

pub use gemini::{GeminiProvider, DEFAULT_GEMINI_MODEL, GEMINI_15_FLASH};
pub use groq::{GroqProvider, DEFAULT_GROQ_MODEL, GROQ_DEEPSEEK_R1};
pub use local_ollama::{LocalOllamaProvider, DEFAULT_OLLAMA_MODEL, DEFAULT_OLLAMA_URL};
pub use openrouter::{
    OpenRouterProvider, DEFAULT_OPENROUTER_FREE_MODEL, OPENROUTER_DEEPSEEK_FREE,
    OPENROUTER_GEMINI_FREE,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{
        CompletionResponse, LatencyMetric, LlmError, LlmProvider, ProviderType, TokenUsage,
    };
    use serde_json::json;

    #[test]
    fn test_gemini_payload_structure() {
        let provider = GeminiProvider::new("test_key".to_string(), Some("gemini-2.0-flash".to_string()));
        assert_eq!(provider.model(), "gemini-2.0-flash");
        assert_eq!(provider.provider_type(), ProviderType::GeminiFlash);
    }

    #[test]
    fn test_groq_payload_structure() {
        let provider = GroqProvider::new("test_key".to_string(), Some("llama-3.3-70b-versatile".to_string()));
        assert_eq!(provider.model(), "llama-3.3-70b-versatile");
        assert_eq!(provider.provider_type(), ProviderType::Groq);
    }

    #[test]
    fn test_openrouter_payload_structure() {
        let provider = OpenRouterProvider::new(
            "test_key".to_string(),
            Some("meta-llama/llama-3.3-70b-instruct:free".to_string()),
        );
        assert_eq!(provider.model(), "meta-llama/llama-3.3-70b-instruct:free");
        assert_eq!(provider.provider_type(), ProviderType::OpenRouter);
    }

    #[test]
    fn test_local_ollama_provider_defaults() {
        let provider = LocalOllamaProvider::new(None, None);
        assert_eq!(provider.model(), "qwen2.5-coder:7b");
        assert_eq!(provider.base_url(), "http://localhost:11434");
        assert_eq!(provider.provider_type(), ProviderType::LocalOllama);
    }

    #[test]
    fn test_gemini_response_deserialization() {
        let mock_gemini_json = json!({
            "candidates": [{
                "content": {
                    "parts": [{"text": "fn add(a: i32, b: i32) -> i32 { a + b }"}]
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 24,
                "candidatesTokenCount": 16,
                "totalTokenCount": 40
            }
        });

        let text = mock_gemini_json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap();
        let prompt_tokens = mock_gemini_json["usageMetadata"]["promptTokenCount"]
            .as_u64()
            .unwrap() as usize;
        let comp_tokens = mock_gemini_json["usageMetadata"]["candidatesTokenCount"]
            .as_u64()
            .unwrap() as usize;

        let completion = CompletionResponse {
            content: text.to_string(),
            model_used: "gemini-2.0-flash".to_string(),
            provider_stamp: ProviderType::GeminiFlash,
            token_usage: TokenUsage::new(prompt_tokens, comp_tokens),
            latency_ms: 120,
            finish_reason: Some("STOP".to_string()),
        };

        assert_eq!(completion.content, "fn add(a: i32, b: i32) -> i32 { a + b }");
        assert_eq!(completion.token_usage.total_tokens, 40);
        assert_eq!(completion.provider_stamp, ProviderType::GeminiFlash);
    }

    #[test]
    fn test_groq_and_openrouter_openai_format_deserialization() {
        let mock_openai_json = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "const calculateSum = (a, b) => a + b;"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 30,
                "completion_tokens": 12,
                "total_tokens": 42
            }
        });

        let text = mock_openai_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap();
        let prompt_tokens = mock_openai_json["usage"]["prompt_tokens"].as_u64().unwrap() as usize;
        let comp_tokens = mock_openai_json["usage"]["completion_tokens"]
            .as_u64()
            .unwrap() as usize;

        let completion = CompletionResponse {
            content: text.to_string(),
            model_used: "llama-3.3-70b-versatile".to_string(),
            provider_stamp: ProviderType::Groq,
            token_usage: TokenUsage::new(prompt_tokens, comp_tokens),
            latency_ms: 85,
            finish_reason: Some("stop".to_string()),
        };

        assert_eq!(completion.content, "const calculateSum = (a, b) => a + b;");
        assert_eq!(completion.token_usage.prompt_tokens, 30);
        assert_eq!(completion.token_usage.completion_tokens, 12);
        assert_eq!(completion.token_usage.total_tokens, 42);
        assert_eq!(completion.provider_stamp, ProviderType::Groq);
    }

    #[test]
    fn test_ollama_format_deserialization() {
        let mock_ollama_json = json!({
            "model": "qwen2.5-coder:7b",
            "response": "def multiply(x, y): return x * y",
            "done": true,
            "prompt_eval_count": 18,
            "eval_count": 9
        });

        let text = mock_ollama_json["response"].as_str().unwrap();
        let prompt_tokens = mock_ollama_json["prompt_eval_count"].as_u64().unwrap() as usize;
        let comp_tokens = mock_ollama_json["eval_count"].as_u64().unwrap() as usize;

        let completion = CompletionResponse {
            content: text.to_string(),
            model_used: "qwen2.5-coder:7b".to_string(),
            provider_stamp: ProviderType::LocalOllama,
            token_usage: TokenUsage::new(prompt_tokens, comp_tokens),
            latency_ms: 210,
            finish_reason: Some("stop".to_string()),
        };

        assert_eq!(completion.content, "def multiply(x, y): return x * y");
        assert_eq!(completion.token_usage.total_tokens, 27);
        assert_eq!(completion.provider_stamp, ProviderType::LocalOllama);
    }

    #[test]
    fn test_llm_error_categorization() {
        let err_rate = LlmError::RateLimited("429 Too Many Requests".to_string());
        let err_auth = LlmError::AuthFailed("401 Unauthorized".to_string());
        let err_timeout = LlmError::NetworkTimeout("deadline exceeded".to_string());
        let err_unavail = LlmError::ProviderUnavailable("connection refused".to_string());

        assert!(matches!(err_rate, LlmError::RateLimited(_)));
        assert!(matches!(err_auth, LlmError::AuthFailed(_)));
        assert!(matches!(err_timeout, LlmError::NetworkTimeout(_)));
        assert!(matches!(err_unavail, LlmError::ProviderUnavailable(_)));
    }
}
