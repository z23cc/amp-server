use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use crate::error::RetryConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub endpoints: Vec<EndpointConfig>,
    #[serde(default)]
    pub global_timeout: Option<u64>, // seconds
    #[serde(default)]
    pub global_retry: Option<RetrySettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrySettings {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetrySettings {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 10000,
            backoff_multiplier: 2.0,
        }
    }
}

impl From<&RetrySettings> for RetryConfig {
    fn from(settings: &RetrySettings) -> Self {
        Self {
            max_attempts: settings.max_attempts,
            base_delay: Duration::from_millis(settings.base_delay_ms),
            max_delay: Duration::from_millis(settings.max_delay_ms),
            backoff_multiplier: settings.backoff_multiplier,
        }
    }
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
    /// Request timeout in seconds (overrides global)
    #[serde(default)]
    pub timeout: Option<u64>,
    /// Retry configuration (overrides global)
    #[serde(default)]
    pub retry: Option<RetrySettings>,
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
            global_timeout: Some(30), // 30 seconds default timeout
            global_retry: Some(RetrySettings::default()),
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
                    timeout: None,
                    retry: None,
                },
                // OpenAI Responses API (streaming or chunked JSON)
                EndpointConfig {
                    path: "/api/provider/openai/v1/responses".to_string(),
                    target_url: "https://api-key.info/v1/responses".to_string(),
                    method: "POST".to_string(),
                    response_type: ResponseType::Stream,
                    custom_headers: HashMap::new(),
                    forward_request_headers: vec![
                        "authorization".to_string(),
                        "content-type".to_string(),
                        "user-agent".to_string(),
                        "accept".to_string(),
                        "accept-encoding".to_string(),
                        "openai-beta".to_string(),
                    ],
                    forward_response_headers: vec![
                        "content-type".to_string(),
                        "cache-control".to_string(),
                    ],
                    enabled: true,
                    timeout: None,
                    retry: None,
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
                    timeout: None,
                    retry: None,
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
                    timeout: None,
                    retry: None,
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
                    timeout: None,
                    retry: None,
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
                    timeout: None,
                    retry: None,
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
                    timeout: None,
                    retry: None,
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
                    timeout: None,
                    retry: None,
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
                    timeout: None,
                    retry: None,
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
                    timeout: None,
                    retry: None,
                },
                // OpenAI models list
                EndpointConfig {
                    path: "/api/provider/openai/v1/models".to_string(),
                    target_url: "https://api-key.info/v1/models".to_string(),
                    method: "GET".to_string(),
                    response_type: ResponseType::Json,
                    custom_headers: HashMap::new(),
                    forward_request_headers: vec![
                        "authorization".to_string(),
                        "user-agent".to_string(),
                        "accept".to_string(),
                        "accept-encoding".to_string(),
                        "openai-beta".to_string(),
                    ],
                    forward_response_headers: vec![
                        "content-type".to_string(),
                        "cache-control".to_string(),
                    ],
                    enabled: true,
                    timeout: None,
                    retry: None,
                },
                // OpenAI embeddings
                EndpointConfig {
                    path: "/api/provider/openai/v1/embeddings".to_string(),
                    target_url: "https://api-key.info/v1/embeddings".to_string(),
                    method: "POST".to_string(),
                    response_type: ResponseType::Json,
                    custom_headers: HashMap::new(),
                    forward_request_headers: vec![
                        "authorization".to_string(),
                        "content-type".to_string(),
                        "user-agent".to_string(),
                        "accept".to_string(),
                        "accept-encoding".to_string(),
                        "openai-beta".to_string(),
                    ],
                    forward_response_headers: vec![
                        "content-type".to_string(),
                        "cache-control".to_string(),
                    ],
                    enabled: true,
                    timeout: None,
                    retry: None,
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
                    timeout: None,
                    retry: None,
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
        
        // Validate the configuration
        config.validate()?;
        
        Ok(config)
    }
    
    pub fn validate(&self) -> Result<(), String> {
        if self.endpoints.is_empty() {
            return Err("No endpoints configured".to_string());
        }
        
        let mut seen_paths = std::collections::HashSet::new();
        
        for endpoint in &self.endpoints {
            endpoint.validate()?;
            
            // Check for duplicate paths
            if !seen_paths.insert(&endpoint.path) {
                return Err(format!("Duplicate endpoint path: {}", endpoint.path));
            }
        }
        
        // Validate global settings
        if let Some(timeout) = self.global_timeout {
            if timeout == 0 || timeout > 3600 {
                return Err("Global timeout must be between 1 and 3600 seconds".to_string());
            }
        }
        
        if let Some(retry) = &self.global_retry {
            retry.validate()?;
        }
        
        Ok(())
    }

    /// Get enabled endpoint configurations
    pub fn enabled_endpoints(&self) -> Vec<&EndpointConfig> {
        self.endpoints.iter().filter(|e| e.enabled).collect()
    }
}

impl EndpointConfig {
    pub fn validate(&self) -> Result<(), String> {
        // Validate path
        if self.path.is_empty() {
            return Err("Endpoint path cannot be empty".to_string());
        }
        
        if !self.path.starts_with('/') {
            return Err(format!("Endpoint path '{}' must start with '/'", self.path));
        }
        
        // Validate target URL
        if self.target_url.is_empty() {
            return Err("Target URL cannot be empty".to_string());
        }
        
        match url::Url::parse(&self.target_url) {
            Ok(url) => {
                if url.scheme() != "http" && url.scheme() != "https" {
                    return Err(format!("Invalid URL scheme in '{}'. Only http and https are supported", self.target_url));
                }
            }
            Err(e) => {
                return Err(format!("Invalid target URL '{}': {}", self.target_url, e));
            }
        }
        
        // Validate HTTP method
        let valid_methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];
        if !valid_methods.contains(&self.method.to_uppercase().as_str()) {
            return Err(format!("Invalid HTTP method '{}'. Supported methods: {:?}", self.method, valid_methods));
        }
        
        // Validate timeout if specified
        if let Some(timeout) = self.timeout {
            if timeout == 0 || timeout > 3600 {
                return Err(format!("Endpoint timeout must be between 1 and 3600 seconds, got {}", timeout));
            }
        }
        
        // Validate retry configuration if specified
        if let Some(retry) = &self.retry {
            retry.validate()?;
        }
        
        // Validate headers
        for (key, _) in &self.custom_headers {
            if key.trim().is_empty() {
                return Err("Custom header name cannot be empty".to_string());
            }
        }
        
        for header in &self.forward_request_headers {
            if header.trim().is_empty() {
                return Err("Forward request header name cannot be empty".to_string());
            }
        }
        
        for header in &self.forward_response_headers {
            if header.trim().is_empty() {
                return Err("Forward response header name cannot be empty".to_string());
            }
        }
        
        Ok(())
    }
    
    pub fn get_timeout(&self, global_timeout: Option<u64>) -> Duration {
        let timeout_secs = self.timeout
            .or(global_timeout)
            .unwrap_or(30); // Default 30 seconds
        Duration::from_secs(timeout_secs)
    }
    
    pub fn get_retry_config(&self, global_retry: &Option<RetrySettings>) -> RetryConfig {
        if let Some(endpoint_retry) = &self.retry {
            return endpoint_retry.into();
        }
        
        if let Some(global_retry) = global_retry {
            return global_retry.into();
        }
        
        RetryConfig::default()
    }
}

impl RetrySettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.max_attempts == 0 || self.max_attempts > 10 {
            return Err(format!("Max retry attempts must be between 1 and 10, got {}", self.max_attempts));
        }
        
        if self.base_delay_ms == 0 || self.base_delay_ms > 60000 {
            return Err(format!("Base delay must be between 1 and 60000 ms, got {}", self.base_delay_ms));
        }
        
        if self.max_delay_ms < self.base_delay_ms || self.max_delay_ms > 600000 {
            return Err(format!("Max delay must be between base_delay_ms and 600000 ms, got {}", self.max_delay_ms));
        }
        
        if self.backoff_multiplier < 1.0 || self.backoff_multiplier > 10.0 {
            return Err(format!("Backoff multiplier must be between 1.0 and 10.0, got {}", self.backoff_multiplier));
        }
        
        Ok(())
    }
}