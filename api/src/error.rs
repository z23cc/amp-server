use anyhow::Result;
use reqwest::StatusCode;
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{warn, error};

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
        }
    }
}

#[derive(Debug)]
pub enum ProxyError {
    InvalidRequest(String),
    UpstreamError(StatusCode, String),
    NetworkError(String),
    TimeoutError,
    ConfigurationError(String),
    ConversionError(String),
}

impl std::fmt::Display for ProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProxyError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            ProxyError::UpstreamError(status, msg) => write!(f, "Upstream error ({}): {}", status, msg),
            ProxyError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            ProxyError::TimeoutError => write!(f, "Request timeout"),
            ProxyError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            ProxyError::ConversionError(msg) => write!(f, "Conversion error: {}", msg),
        }
    }
}

impl std::error::Error for ProxyError {}

impl From<ProxyError> for (StatusCode, String) {
    fn from(error: ProxyError) -> Self {
        match error {
            ProxyError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ProxyError::UpstreamError(status, msg) => (status, msg),
            ProxyError::NetworkError(msg) => (StatusCode::BAD_GATEWAY, msg),
            ProxyError::TimeoutError => (StatusCode::GATEWAY_TIMEOUT, "Request timeout".to_string()),
            ProxyError::ConfigurationError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ProxyError::ConversionError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        }
    }
}

pub async fn retry_with_backoff<F, T, Fut>(
    config: &RetryConfig,
    operation: F,
) -> Result<T, ProxyError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, ProxyError>>,
{
    let mut delay = config.base_delay;
    
    for attempt in 1..=config.max_attempts {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                // Don't retry on client errors (4xx) except for 429 (Too Many Requests)
                if let ProxyError::UpstreamError(status, _) = &error {
                    if status.is_client_error() && *status != StatusCode::TOO_MANY_REQUESTS {
                        return Err(error);
                    }
                }
                
                if attempt < config.max_attempts {
                    warn!("Attempt {}/{} failed: {}. Retrying in {:?}...", 
                          attempt, config.max_attempts, error, delay);
                    
                    sleep(delay).await;
                    delay = Duration::from_millis(
                        std::cmp::min(
                            (delay.as_millis() as f64 * config.backoff_multiplier) as u64,
                            config.max_delay.as_millis() as u64,
                        )
                    );
                } else {
                    error!("All {} attempts failed. Last error: {}", config.max_attempts, error);
                    return Err(error);
                }
            }
        }
    }
    
    unreachable!()
}

pub fn is_retriable_error(error: &ProxyError) -> bool {
    match error {
        ProxyError::NetworkError(_) => true,
        ProxyError::TimeoutError => true,
        ProxyError::UpstreamError(status, _) => {
            // Retry on server errors (5xx) and 429 Too Many Requests
            status.is_server_error() || *status == StatusCode::TOO_MANY_REQUESTS
        }
        _ => false,
    }
}

pub fn create_error_response(error: &ProxyError) -> Value {
    serde_json::json!({
        "error": {
            "type": match error {
                ProxyError::InvalidRequest(_) => "invalid_request_error",
                ProxyError::UpstreamError(_, _) => "api_error",
                ProxyError::NetworkError(_) => "connection_error",
                ProxyError::TimeoutError => "timeout_error",
                ProxyError::ConfigurationError(_) => "server_error",
                ProxyError::ConversionError(_) => "conversion_error",
            },
            "message": error.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }
    })
}
