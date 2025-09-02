use async_stream::stream;
use axum::{
    Json, Router,
    body::Body,
    extract::Request,
    http::{HeaderMap, HeaderName, StatusCode, Method},
    response::{
        IntoResponse, Response,
        sse::{Event, Sse},
    },
    routing::{get, post, put, delete},
};
use reqwest::Client;
use std::convert::Infallible;
use tracing::{error, info, warn};
use serde_json::Value;

use crate::get_amp_api_key;
use super::config::{ProxyConfig, EndpointConfig, ResponseType};

pub struct ProxyService {
    config: ProxyConfig,
}

impl ProxyService {
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            config,
        }
    }

    pub fn create_router(&self) -> Router {
        let mut router = Router::new();

        for endpoint in self.config.enabled_endpoints() {
            let endpoint_clone = endpoint.clone();
            let path = endpoint.path.clone();

            match endpoint.method.to_uppercase().as_str() {
                "GET" => {
                    router = router.route(&path, get(move |req| {
                        Self::handle_proxy_request(endpoint_clone, req)
                    }));
                }
                "POST" => {
                    router = router.route(&path, post(move |req| {
                        Self::handle_proxy_request(endpoint_clone, req)
                    }));
                }
                "PUT" => {
                    router = router.route(&path, put(move |req| {
                        Self::handle_proxy_request(endpoint_clone, req)
                    }));
                }
                "DELETE" => {
                    router = router.route(&path, delete(move |req| {
                        Self::handle_proxy_request(endpoint_clone, req)
                    }));
                }
                _ => {
                    warn!("Unsupported HTTP method: {} for path: {}", endpoint.method, endpoint.path);
                }
            }
        }

        router
    }

    async fn handle_proxy_request(
        config: EndpointConfig,
        req: Request,
    ) -> Result<Response, (StatusCode, String)> {
        let client = Client::new();
        let (parts, body) = req.into_parts();
        
        info!("=== Incoming Request ===");
        info!("Method: {}", parts.method);
        info!("Path: {} -> {}", config.path, config.target_url);
        info!("Headers: {:?}", parts.headers);

        // Read request body
        let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to read request body: {}", e);
                return Err((StatusCode::BAD_REQUEST, "Unable to read request body".to_string()));
            }
        };

        // Print request body if it's JSON
        if let Ok(body_str) = String::from_utf8(body_bytes.clone().to_vec()) {
            if !body_str.is_empty() {
                if let Ok(json_value) = serde_json::from_str::<Value>(&body_str) {
                    info!("Request Body (JSON): {}", serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| body_str));
                } else {
                    info!("Request Body (Text): {}", body_str);
                }
            }
        }

        // Build request
        let method = Method::from_bytes(config.method.as_bytes())
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Invalid HTTP method".to_string()))?;

        let mut req_builder = client
            .request(method, &config.target_url)
            .body(body_bytes);

        // Add forwarded request headers
        for header_name in &config.forward_request_headers {
            if let Some(header_value) = parts.headers.get(header_name) {
                req_builder = req_builder.header(header_name, header_value);
            }
        }

        // Add custom request headers
        for (name, value) in &config.custom_headers {
            req_builder = req_builder.header(name, value);
        }

        // Special handling: add auth header for LLM proxy
        if config.path.contains("llm-proxy") {
            req_builder = req_builder.header("authorization", format!("Bearer {}", get_amp_api_key()));
        }

        // Send request
        let response = match req_builder.send().await {
            Ok(resp) => resp,
            Err(e) => {
                error!("Failed to forward request: {}", e);
                return Err((StatusCode::BAD_GATEWAY, format!("Forward failed: {e}")));
            }
        };

        info!("=== Response from {} ===", config.target_url);
        info!("Status: {}", response.status());
        info!("Response Headers: {:?}", response.headers());

        if !response.status().is_success() {
            error!("Upstream server returned error status: {}", response.status());
            return Err((StatusCode::BAD_GATEWAY, "Upstream server error".to_string()));
        }

        // Handle based on response type
        match config.response_type {
            ResponseType::Sse => Self::handle_sse_response(response, &config).await,
            ResponseType::Stream => Self::handle_stream_response(response, &config).await,
            ResponseType::Json => Self::handle_json_response(response, &config).await,
            ResponseType::Html => Self::handle_html_response(response, &config).await,
        }
    }

    async fn handle_sse_response(
        response: reqwest::Response,
        config: &EndpointConfig,
    ) -> Result<Response, (StatusCode, String)> {
        info!("Starting SSE stream processing for endpoint: {}", config.path);
        let mut response_headers = HeaderMap::new();
        
        // Forward response headers
        for header_name in &config.forward_response_headers {
            if let Some(header_value) = response.headers().get(header_name) {
                if let Ok(name) = HeaderName::from_bytes(header_name.as_bytes()) {
                    response_headers.insert(name, header_value.clone());
                }
            }
        }

        let stream = stream! {
            let mut bytes_stream = response.bytes_stream();
            let mut buffer = Vec::new();

            while let Some(chunk) = futures_util::StreamExt::next(&mut bytes_stream).await {
                match chunk {
                    Ok(bytes) => {
                        buffer.extend_from_slice(&bytes);

                        let text = String::from_utf8_lossy(&buffer);
                        let lines_vec: Vec<&str> = text.lines().collect();

                        if lines_vec.len() > 1 {
                            for line in &lines_vec[..lines_vec.len()-1] {
                                if let Some(data) = Self::parse_sse_line(line) {
                                    yield Ok::<Event, Infallible>(Event::default().data(data));
                                }
                            }

                            buffer = lines_vec.last().unwrap().as_bytes().to_vec();
                        }
                    }
                    Err(e) => {
                        error!("Failed to read SSE response stream: {}", e);
                        break;
                    }
                }
            }

            if !buffer.is_empty() {
                let text = String::from_utf8_lossy(&buffer);
                for line in text.lines() {
                    if let Some(data) = Self::parse_sse_line(line) {
                        yield Ok::<Event, Infallible>(Event::default().data(data));
                    }
                }
            }
        };

        let sse_response = Sse::new(stream);
        let mut final_response = sse_response.into_response();
        final_response.headers_mut().extend(response_headers);

        Ok(final_response)
    }

    async fn handle_stream_response(
        response: reqwest::Response,
        config: &EndpointConfig,
    ) -> Result<Response, (StatusCode, String)> {
        let status = response.status();
        let headers = response.headers().clone();

        let mut response_builder = Response::builder().status(status);

        // Forward response headers
        for header_name in &config.forward_response_headers {
            if let Some(header_value) = headers.get(header_name) {
                let name_str = header_name.as_str();
                if !name_str.starts_with("connection") && !name_str.starts_with("transfer-encoding") {
                    response_builder = response_builder.header(header_name, header_value);
                }
            }
        }

        // Check if it's a streaming response
        let is_streaming = headers
            .get("content-type")
            .and_then(|ct| ct.to_str().ok())
            .map(|ct| ct.contains("text/event-stream") || ct.contains("application/stream"))
            .unwrap_or(false);

        if is_streaming {
            let stream = futures_util::StreamExt::map(response.bytes_stream(), |result| {
                result.map_err(std::io::Error::other)
            });
            let body = Body::from_stream(stream);
            
            response_builder.body(body)
                .map_err(|e| {
                    error!("Failed to build streaming response: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Failed to build streaming response".to_string())
                })
        } else {
            let body_bytes = response.bytes().await
                .map_err(|e| {
                    error!("Failed to read response body: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read response".to_string())
                })?;

            response_builder.body(Body::from(body_bytes))
                .map_err(|e| {
                    error!("Failed to build response: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response".to_string())
                })
        }
    }

    async fn handle_json_response(
        response: reqwest::Response,
        config: &EndpointConfig,
    ) -> Result<Response, (StatusCode, String)> {
        let status = response.status();
        let mut response_headers = HeaderMap::new();

        // Forward response headers
        for header_name in &config.forward_response_headers {
            if let Some(header_value) = response.headers().get(header_name) {
                if let Ok(name) = HeaderName::from_bytes(header_name.as_bytes()) {
                    response_headers.insert(name, header_value.clone());
                }
            }
        }

        let json_data: Value = response.json().await
            .map_err(|e| {
                error!("Failed to parse JSON response: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to parse response".to_string())
            })?;
        
        info!("Response Body (JSON): {}", serde_json::to_string_pretty(&json_data).unwrap_or_else(|_| "Invalid JSON".to_string()));

        let mut json_response = Json(json_data).into_response();
        *json_response.status_mut() = status;
        json_response.headers_mut().extend(response_headers);

        Ok(json_response)
    }

    async fn handle_html_response(
        response: reqwest::Response,
        config: &EndpointConfig,
    ) -> Result<Response, (StatusCode, String)> {
        let status = response.status();
        let mut response_headers = HeaderMap::new();

        // Forward response headers
        for header_name in &config.forward_response_headers {
            if let Some(header_value) = response.headers().get(header_name) {
                if let Ok(name) = HeaderName::from_bytes(header_name.as_bytes()) {
                    response_headers.insert(name, header_value.clone());
                }
            }
        }

        let html_text = response.text().await
            .map_err(|e| {
                error!("Failed to read HTML response: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read response".to_string())
            })?;
            
        info!("Response Body (HTML): {}", if html_text.len() > 1000 { 
            format!("{}... (truncated, length: {})", &html_text[..1000], html_text.len()) 
        } else { 
            html_text.clone() 
        });

        let mut html_response = Response::builder()
            .status(status)
            .header("content-type", "text/html")
            .body(Body::from(html_text))
            .map_err(|e| {
                error!("Failed to build HTML response: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response".to_string())
            })?;

        html_response.headers_mut().extend(response_headers);

        Ok(html_response)
    }

    fn parse_sse_line(line: &str) -> Option<String> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        if let Some(data_content) = line.strip_prefix("data: ") {
            if data_content == "[DONE]" {
                Some("[DONE]".to_string())
            } else {
                Some(data_content.to_string())
            }
        } else if let Some(stripped) = line.strip_prefix("data:") {
            Some(stripped.to_string())
        } else {
            Some(line.to_string())
        }
    }
}