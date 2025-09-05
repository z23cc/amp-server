use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub endpoints: Vec<EndpointConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointConfig {
    /// Local route path
    pub path: String,
    /// Target forwarding URL
    pub target_url: String,
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    pub method: String,
    /// Response type (json, sse, stream, html)
    pub response_type: ResponseType,
    /// Custom request headers
    pub custom_headers: HashMap<String, String>,
    /// List of request headers to forward
    pub forward_request_headers: Vec<String>,
    /// List of response headers to forward
    pub forward_response_headers: Vec<String>,
    /// Whether this endpoint is enabled
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResponseType {
    Json,
    Sse,
    Stream,
    Html,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            endpoints: vec![
                // OpenAI compatible endpoint
                EndpointConfig {
                    path: "/api/provider/openai/v1/chat/completions".to_string(),
                    target_url: "https://api-key.info/v1/chat/completions".to_string(),
                    method: "POST".to_string(),
                    response_type: ResponseType::Stream,
                    custom_headers: HashMap::new(),
                    forward_request_headers: vec![
                        "authorization".to_string(),
                        "content-type".to_string(),
                        "user-agent".to_string(),
                        "accept".to_string(),
                        "accept-encoding".to_string(),
                    ],
                    forward_response_headers: vec![
                        "content-type".to_string(),
                        "cache-control".to_string(),
                    ],
                    enabled: true,
                },
                // Anthropic compatible endpoint
                EndpointConfig {
                    path: "/api/provider/anthropic/v1/messages".to_string(),
                    target_url: "https://api-key.info/v1/messages".to_string(),
                    method: "POST".to_string(),
                    response_type: ResponseType::Stream,
                    custom_headers: HashMap::new(),
                    forward_request_headers: vec![
                        "authorization".to_string(),
                        "content-type".to_string(),
                        "user-agent".to_string(),
                        "accept".to_string(),
                        "accept-encoding".to_string(),
                        "anthropic-version".to_string(),
                    ],
                    forward_response_headers: vec![
                        "content-type".to_string(),
                        "cache-control".to_string(),
                    ],
                    enabled: true,
                },
                // LLM proxy endpoint
                EndpointConfig {
                    path: "/api/tab/llm-proxy".to_string(),
                    target_url: "https://ampcode.com/api/tab/llm-proxy".to_string(),
                    method: "POST".to_string(),
                    response_type: ResponseType::Sse,
                    custom_headers: HashMap::new(),
                    forward_request_headers: vec![
                        "authorization".to_string(),
                        "user-agent".to_string(),
                        "x-amp-feature".to_string(),
                        "accept-language".to_string(),
                        "sec-fetch-mode".to_string(),
                    ],
                    forward_response_headers: vec![
                        "alt-svc".to_string(),
                        "content-security-policy".to_string(),
                        "fireworks-backend-host".to_string(),
                        "fireworks-cached-prompt-tokens".to_string(),
                        "fireworks-deployment".to_string(),
                        "fireworks-generation-queue-duration".to_string(),
                        "fireworks-num-concurrent-requests".to_string(),
                        "fireworks-prefill-duration".to_string(),
                        "fireworks-prefill-queue-duration".to_string(),
                        "fireworks-prompt-tokens".to_string(),
                        "fireworks-sampling-options".to_string(),
                        "fireworks-server-time-to-first-token".to_string(),
                        "fireworks-speculation-matched-tokens".to_string(),
                        "fireworks-speculation-prompt-tokens".to_string(),
                        "fireworks-tokenizer-duration".to_string(),
                        "fireworks-tokenizer-queue-duration".to_string(),
                    ],
                    enabled: true,
                },
                // Google Gemini streaming content generation
                EndpointConfig {
                    path: "/api/provider/google/v1beta/models/gemini-pro:streamGenerateContent".to_string(),
                    target_url: "https://api-key.info/v1beta/models/gemini-pro:streamGenerateContent".to_string(),
                    method: "POST".to_string(),
                    response_type: ResponseType::Sse,
                    custom_headers: HashMap::new(),
                    forward_request_headers: vec![
                        "authorization".to_string(),
                        "content-type".to_string(),
                        "user-agent".to_string(),
                        "accept".to_string(),
                        "accept-encoding".to_string(),
                    ],
                    forward_response_headers: vec![
                        "content-type".to_string(),
                        "cache-control".to_string(),
                    ],
                    enabled: true,
                },
                // Google Gemini non-streaming content generation
                EndpointConfig {
                    path: "/api/provider/google/v1beta/models/gemini-pro:generateContent".to_string(),
                    target_url: "https://api-key.info/v1beta/models/gemini-pro:generateContent".to_string(),
                    method: "POST".to_string(),
                    response_type: ResponseType::Json,
                    custom_headers: HashMap::new(),
                    forward_request_headers: vec![
                        "authorization".to_string(),
                        "content-type".to_string(),
                        "user-agent".to_string(),
                        "accept".to_string(),
                        "accept-encoding".to_string(),
                    ],
                    forward_response_headers: vec![
                        "content-type".to_string(),
                        "cache-control".to_string(),
                    ],
                    enabled: true,
                },
                // Google Gemini models list
                EndpointConfig {
                    path: "/api/provider/google/v1beta/models".to_string(),
                    target_url: "https://api-key.info/v1beta/models".to_string(),
                    method: "GET".to_string(),
                    response_type: ResponseType::Json,
                    custom_headers: HashMap::new(),
                    forward_request_headers: vec![
                        "authorization".to_string(),
                        "user-agent".to_string(),
                        "accept".to_string(),
                        "accept-encoding".to_string(),
                    ],
                    forward_response_headers: vec![
                        "content-type".to_string(),
                        "cache-control".to_string(),
                    ],
                    enabled: true,
                },
                // Google Gemini text embedding
                EndpointConfig {
                    path: "/api/provider/google/v1beta/models/embedding-001:embedContent".to_string(),
                    target_url: "https://api-key.info/v1beta/models/embedding-001:embedContent".to_string(),
                    method: "POST".to_string(),
                    response_type: ResponseType::Json,
                    custom_headers: HashMap::new(),
                    forward_request_headers: vec![
                        "authorization".to_string(),
                        "content-type".to_string(),
                        "user-agent".to_string(),
                        "accept".to_string(),
                        "accept-encoding".to_string(),
                    ],
                    forward_response_headers: vec![
                        "content-type".to_string(),
                        "cache-control".to_string(),
                    ],
                    enabled: true,
                },
                // Google Gemini 2.5 Flash streaming
                EndpointConfig {
                    path: "/api/provider/google/v1beta/models/gemini-2.5-flash-preview-05-20:streamGenerateContent".to_string(),
                    target_url: "https://api-key.info/v1beta/models/gemini-2.5-flash-preview-05-20:streamGenerateContent".to_string(),
                    method: "POST".to_string(),
                    response_type: ResponseType::Sse,
                    custom_headers: HashMap::new(),
                    forward_request_headers: vec![
                        "authorization".to_string(),
                        "content-type".to_string(),
                        "user-agent".to_string(),
                        "accept".to_string(),
                        "accept-encoding".to_string(),
                    ],
                    forward_response_headers: vec![
                        "content-type".to_string(),
                        "cache-control".to_string(),
                    ],
                    enabled: true,
                },
                // Google Gemini 2.5 Flash non-streaming
                EndpointConfig {
                    path: "/api/provider/google/v1beta/models/gemini-2.5-flash-preview-05-20:generateContent".to_string(),
                    target_url: "https://api-key.info/v1beta/models/gemini-2.5-flash-preview-05-20:generateContent".to_string(),
                    method: "POST".to_string(),
                    response_type: ResponseType::Json,
                    custom_headers: HashMap::new(),
                    forward_request_headers: vec![
                        "authorization".to_string(),
                        "content-type".to_string(),
                        "user-agent".to_string(),
                        "accept".to_string(),
                        "accept-encoding".to_string(),
                    ],
                    forward_response_headers: vec![
                        "content-type".to_string(),
                        "cache-control".to_string(),
                    ],
                    enabled: true,
                },
                // Cerebras OpenAI-compatible endpoint
                EndpointConfig {
                    path: "/api/provider/cerebras/v1/chat/completions".to_string(),
                    target_url: "https://api-key.info/v1/chat/completions".to_string(),
                    method: "POST".to_string(),
                    response_type: ResponseType::Stream,
                    custom_headers: HashMap::new(),
                    forward_request_headers: vec![
                        "authorization".to_string(),
                        "content-type".to_string(),
                        "user-agent".to_string(),
                        "accept".to_string(),
                        "accept-encoding".to_string(),
                    ],
                    forward_response_headers: vec![
                        "content-type".to_string(),
                        "cache-control".to_string(),
                    ],
                    enabled: true,
                },
            ],

        }
    }
}

impl ProxyConfig {
    /// Load configuration from YAML file
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: ProxyConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Get enabled endpoint configurations
    pub fn enabled_endpoints(&self) -> Vec<&EndpointConfig> {
        self.endpoints.iter().filter(|e| e.enabled).collect()
    }
}