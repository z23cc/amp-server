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
use serde_json::{Value, json};


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

        // First, handle OpenAI Responses -> Chat Completions conversion for o3 models
        let (config_after_o3, body_after_o3, is_o3_conversion, original_request_json_o3) =
            Self::handle_o3_model_conversion(config, &body_bytes)?;

        // Then, handle Google Responses -> Gemini (generateContent/streamGenerateContent)
        let (final_config, final_body_bytes, is_google_conversion, original_request_json_google) =
            Self::handle_google_responses_conversion(config_after_o3, &body_after_o3)?;

        // Build request
        let method = Method::from_bytes(final_config.method.as_bytes())
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Invalid HTTP method".to_string()))?;

        let mut req_builder = client
            .request(method, &final_config.target_url)
            .body(final_body_bytes);

        // Add forwarded request headers
        for header_name in &final_config.forward_request_headers {
            if let Some(header_value) = parts.headers.get(header_name) {
                req_builder = req_builder.header(header_name, header_value);
            }
        }

        // Add custom request headers
        for (name, value) in &final_config.custom_headers {
            req_builder = req_builder.header(name, value);
        }

        // Special handling: add auth header for LLM proxy
        if final_config.path.contains("llm-proxy") {
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

        info!("=== Response from {} ===", final_config.target_url);
        info!("Status: {}", response.status());
        info!("Response Headers: {:?}", response.headers());

        if !response.status().is_success() {
            error!("Upstream server returned error status: {}", response.status());
            return Err((StatusCode::BAD_GATEWAY, "Upstream server error".to_string()));
        }

        // Handle response, with potential conversion back to Responses API format
        let mut result = match final_config.response_type {
            ResponseType::Sse => Self::handle_sse_response(response, &final_config).await,
            ResponseType::Stream => Self::handle_stream_response(response, &final_config).await,
            ResponseType::Json => Self::handle_json_response(response, &final_config).await,
            ResponseType::Html => Self::handle_html_response(response, &final_config).await,
        }?;

        // Convert back to Responses API format if needed
        if is_o3_conversion {
            let is_streaming = original_request_json_o3
                .as_ref()
                .and_then(|v| v.get("stream").and_then(|v| v.as_bool()))
                .unwrap_or(false);
            info!("O3 conversion: is_streaming = {}", is_streaming);
            result = Self::convert_chat_completions_to_responses_format(result, is_streaming).await?;
        } else if is_google_conversion {
            let is_streaming = original_request_json_google
                .as_ref()
                .and_then(|v| v.get("stream").and_then(|v| v.as_bool()))
                .unwrap_or(false);
            info!("Google Gemini conversion: is_streaming = {}", is_streaming);
            result = Self::convert_gemini_to_responses_format(result, is_streaming).await?;
        }

        Ok(result)
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

        // Read raw bytes so we can decide whether it's JSON or plain text (e.g., error bodies)
        let body_bytes = response.bytes().await.map_err(|e| {
            error!("Failed to read response body: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read response".to_string())
        })?;

        // Try to parse JSON first
        match serde_json::from_slice::<Value>(&body_bytes) {
            Ok(json_data) => {
                info!(
                    "Response Body (JSON): {}",
                    serde_json::to_string_pretty(&json_data).unwrap_or_else(|_| "Invalid JSON".to_string())
                );
                let mut json_response = Json(json_data).into_response();
                *json_response.status_mut() = status;
                json_response.headers_mut().extend(response_headers);
                Ok(json_response)
            }
            Err(err) => {
                // Fallback to returning plain text with original status and forwarded headers
                error!("Failed to parse JSON response, returning text: {}", err);
                let mut builder = Response::builder().status(status);
                // Default to text/plain; upstream content-type is already forwarded in response_headers if configured
                builder = builder.header("content-type", "text/plain; charset=utf-8");

                let mut resp = builder
                    .body(Body::from(body_bytes))
                    .map_err(|e| {
                        error!("Failed to build text response: {}", e);
                        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response".to_string())
                    })?;
                resp.headers_mut().extend(response_headers);
                Ok(resp)
            }
        }
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

    /// Check if this is a Responses API request for o3 models and convert to Chat Completions if needed
    fn handle_o3_model_conversion(
        config: EndpointConfig,
        body_bytes: &[u8],
    ) -> Result<(EndpointConfig, Vec<u8>, bool, Option<Value>), (StatusCode, String)> {
        // Only process Responses API requests
        if !config.path.contains("/v1/responses") {
            return Ok((config, body_bytes.to_vec(), false, None));
        }

        // Try to parse the request body as JSON
        let request_json: Value = match serde_json::from_slice(body_bytes) {
            Ok(json) => json,
            Err(_) => return Ok((config, body_bytes.to_vec(), false, None)), // Not JSON, pass through
        };

        // Check if the model is o3 or o3-mini
        let model = request_json.get("model")
            .and_then(|m| m.as_str())
            .unwrap_or("");

        if !model.starts_with("o3") {
            return Ok((config, body_bytes.to_vec(), false, None));
        }

        info!("Converting Responses API request for o3 model '{}' to Chat Completions format", model);

        // Convert Responses API request to Chat Completions format
        let chat_request = Self::convert_responses_to_chat_completions(&request_json)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to convert request: {}", e)))?;

        // Create new config for Chat Completions endpoint
        let mut chat_config = config.clone();
        chat_config.target_url = chat_config.target_url.replace("/v1/responses", "/v1/chat/completions");
        chat_config.path = chat_config.path.replace("/v1/responses", "/v1/chat/completions");

        // Serialize the converted request
        let chat_body = serde_json::to_vec(&chat_request)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize request: {}", e)))?;

        Ok((chat_config, chat_body, true, Some(request_json)))
    }

    /// Check if this is a Google Responses API request and convert to Gemini generateContent if needed
    fn handle_google_responses_conversion(
        mut config: EndpointConfig,
        body_bytes: &[u8],
    ) -> Result<(EndpointConfig, Vec<u8>, bool, Option<Value>), (StatusCode, String)> {
        // Only process Google responses path
        let is_google_responses = config.path.contains("/api/provider/google/") && config.path.contains("/responses");
        if !is_google_responses {
            return Ok((config, body_bytes.to_vec(), false, None));
        }

        // Parse body as JSON
        let request_json: Value = match serde_json::from_slice(body_bytes) {
            Ok(json) => json,
            Err(_) => return Ok((config, body_bytes.to_vec(), false, None)),
        };

        // Extract model
        let model = request_json
            .get("model")
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .to_string();
        if model.is_empty() {
            // No model -> let it pass through (upstream likely to error, but do not hijack)
            return Ok((config, body_bytes.to_vec(), false, None));
        }

        // Determine streaming
        let is_stream = request_json
            .get("stream")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Convert Responses request to Gemini request
        let gemini_request = match Self::convert_responses_to_gemini_request(&request_json) {
            Ok(v) => v,
            Err(e) => return Err((StatusCode::BAD_REQUEST, format!("Failed to convert Google Responses request: {}", e))),
        };

        // Build target URL from base + /{model}:{op}
        // Expect config.target_url like "https://api-key.info/v1beta/models"
        let base = config.target_url.trim_end_matches('/');
        let op = if is_stream { "streamGenerateContent" } else { "generateContent" };
        let new_target = format!("{}/{}:{}", base, model, op);

        // Update config path (for logging) and target URL
        tracing::info!("Converting Google Responses request: model='{}', stream={}, target='{}'", model, is_stream, new_target);
        config.target_url = new_target;
        config.path = format!("/api/provider/google/v1beta/models/{}:{}", model, op);

        let body = serde_json::to_vec(&gemini_request)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize Gemini request: {}", e)))?;

        Ok((config, body, true, Some(request_json)))
    }

    /// Convert Responses API request format to Chat Completions format
    fn convert_responses_to_chat_completions(responses_request: &Value) -> Result<Value, String> {
        let mut chat_request = serde_json::json!({});

        // Copy basic fields
        if let Some(model) = responses_request.get("model") {
            chat_request["model"] = model.clone();
        }

        if let Some(stream) = responses_request.get("stream") {
            chat_request["stream"] = stream.clone();
        }

        if let Some(max_tokens) = responses_request.get("max_completion_tokens") {
            chat_request["max_tokens"] = max_tokens.clone();
        }

        if let Some(temperature) = responses_request.get("temperature") {
            chat_request["temperature"] = temperature.clone();
        }

        // Convert input array to messages array
        if let Some(input) = responses_request.get("input").and_then(|i| i.as_array()) {
            let mut messages = Vec::new();

            for item in input {
                if let Some(role) = item.get("role").and_then(|r| r.as_str()) {
                    if let Some(content) = item.get("content") {
                        messages.push(serde_json::json!({
                            "role": role,
                            "content": content
                        }));
                    }
                }
            }

            chat_request["messages"] = serde_json::json!(messages);
        }

        Ok(chat_request)
    }

    /// Convert Chat Completions streaming response back to Responses API format
    async fn convert_chat_completions_to_responses_format(
        response: Response,
        is_streaming: bool,
    ) -> Result<Response, (StatusCode, String)> {
        if !is_streaming {
            // For non-streaming responses, we need to convert the JSON structure
            let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read response body: {}", e)))?;

            let chat_response: Value = serde_json::from_slice(&body_bytes)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse response JSON: {}", e)))?;

            let responses_format = Self::convert_chat_completion_to_responses_json(&chat_response)?;

            let response_body = serde_json::to_vec(&responses_format)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize response: {}", e)))?;

            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(Body::from(response_body))
                .unwrap());
        }

        // For streaming responses, we'll use a simpler approach
        // Convert the response body to bytes and then process line by line
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read streaming response: {}", e)))?;

        let body_text = String::from_utf8_lossy(&body_bytes);
        let mut converted_lines = Vec::new();

        for line in body_text.lines() {
            if line.starts_with("data: ") {
                let data_part = &line[6..]; // Remove "data: " prefix
                if data_part == "[DONE]" {
                    converted_lines.push("data: [DONE]".to_string());
                    continue;
                }

                // Parse the Chat Completions chunk
                if let Ok(chunk) = serde_json::from_str::<Value>(data_part) {
                    if let Ok(responses_chunk) = Self::convert_chat_chunk_to_responses_chunk(&chunk) {
                        converted_lines.push(format!("data: {}", serde_json::to_string(&responses_chunk).unwrap_or_default()));
                    }
                }
            } else if line.is_empty() {
                converted_lines.push("".to_string());
            }
        }

        let converted_body = converted_lines.join("\n\n");

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-cache")
            .header("connection", "keep-alive")
            .body(Body::from(converted_body))
            .unwrap())
    }

    /// Convert Gemini (generateContent/streamGenerateContent) response to Responses API format
    async fn convert_gemini_to_responses_format(
        response: Response,
        is_streaming: bool,
    ) -> Result<Response, (StatusCode, String)> {
        if !is_streaming {
            // Non-streaming: pass through JSON (best-effort; clients may still handle)
            return Ok(response);
        }

        // Read the whole SSE body, then re-emit as Responses-style SSE
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read streaming response: {}", e)))?;
        let body_text = String::from_utf8_lossy(&body_bytes);

        let mut converted_lines = Vec::new();

        for line in body_text.lines() {
            if line.starts_with("data: ") {
                let data_part = &line[6..];
                if data_part == "[DONE]" { // Some implementations may send this sentinel
                    converted_lines.push("data: [DONE]".to_string());
                    continue;
                }

                // Parse Gemini chunk
                if let Ok(chunk) = serde_json::from_str::<Value>(data_part) {
                    // Emit response.created once if we see an id or first candidate
                    if let Some(created_evt) = Self::maybe_gemini_created_event(&chunk) {
                        converted_lines.push(format!("data: {}", serde_json::to_string(&created_evt).unwrap_or_default()));
                    }

                    // Emit delta text if present
                    if let Some(delta_text) = Self::extract_gemini_text_delta(&chunk) {
                        let responses_chunk = json!({
                            "type": "response.output_text.delta",
                            "delta": delta_text
                        });
                        converted_lines.push(format!("data: {}", serde_json::to_string(&responses_chunk).unwrap_or_default()));
                    }

                    // Emit completed when finishReason is present and not null
                    if Self::gemini_chunk_finished(&chunk) {
                        let usage = chunk.get("usageMetadata").cloned();
                        let responses_chunk = json!({
                            "type": "response.completed",
                            "response": {
                                "id": chunk.get("id").unwrap_or(&json!("response-unknown")),
                                "object": "response",
                                "created": chunk.get("created").unwrap_or(&json!(0)),
                                "model": chunk.get("model").unwrap_or(&json!("gemini")),
                                "usage": usage
                            }
                        });
                        converted_lines.push(format!("data: {}", serde_json::to_string(&responses_chunk).unwrap_or_default()));
                    }
                }
            } else if line.is_empty() {
                converted_lines.push(String::new());
            }
        }

        let converted_body = converted_lines.join("\n\n");

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-cache")
            .header("connection", "keep-alive")
            .body(Body::from(converted_body))
            .unwrap())
    }

    fn maybe_gemini_created_event(chunk: &Value) -> Option<Value> {
        // Heuristic: if candidates exist and we haven't signaled created yet
        if chunk.get("candidates").is_some() {
            return Some(json!({
                "type": "response.created",
                "response": {
                    "id": chunk.get("id").unwrap_or(&json!("response-unknown")),
                    "object": "response",
                    "created": chunk.get("created").unwrap_or(&json!(0)),
                    "model": chunk.get("model").unwrap_or(&json!("gemini"))
                }
            }));
        }
        None
    }

    fn gemini_chunk_finished(chunk: &Value) -> bool {
        // Look for candidates[0].finishReason
        chunk
            .get("candidates")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("finishReason"))
            .map(|fr| !fr.is_null())
            .unwrap_or(false)
    }

    fn extract_gemini_text_delta(chunk: &Value) -> Option<String> {
        // Try candidates[0].content.parts[*].text and concatenate
        let mut acc = String::new();
        if let Some(arr) = chunk.get("candidates").and_then(|c| c.as_array()) {
            if let Some(first) = arr.first() {
                if let Some(content) = first.get("content") {
                    if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                        for p in parts {
                            if let Some(text) = p.get("text").and_then(|t| t.as_str()) {
                                acc.push_str(text);
                            }
                        }
                    }
                }
            }
        }
        if acc.is_empty() { None } else { Some(acc) }
    }

    /// Convert OpenAI Responses-style request to Gemini generateContent request
    fn convert_responses_to_gemini_request(responses_request: &Value) -> Result<Value, String> {
        let mut contents: Vec<Value> = Vec::new();
        let mut system_texts: Vec<String> = Vec::new();

        if let Some(input) = responses_request.get("input").and_then(|i| i.as_array()) {
            for item in input {
                let role = item.get("role").and_then(|r| r.as_str()).unwrap_or("user");
                let content_val = item.get("content").cloned().unwrap_or(json!(""));

                // Gather system into systemInstruction; others into contents
                if role.eq_ignore_ascii_case("system") {
                    if let Some(txt) = Self::content_value_to_text(&content_val) {
                        system_texts.push(txt);
                    }
                    continue;
                }

                let gemini_role = match role {
                    "assistant" => "model",
                    _ => "user",
                };

                let text = Self::content_value_to_text(&content_val).unwrap_or_default();
                let content = json!({
                    "role": gemini_role,
                    "parts": [{ "text": text }]
                });
                contents.push(content);
            }
        }

        let mut req = json!({
            "contents": contents,
        });

        let mut gen_cfg = serde_json::Map::new();
        if let Some(t) = responses_request.get("temperature") {
            gen_cfg.insert("temperature".to_string(), t.clone());
        }
        if let Some(mt) = responses_request.get("max_completion_tokens") {
            gen_cfg.insert("maxOutputTokens".to_string(), mt.clone());
        }
        if let Some(tp) = responses_request.get("top_p") { gen_cfg.insert("topP".to_string(), tp.clone()); }
        if let Some(tk) = responses_request.get("top_k") { gen_cfg.insert("topK".to_string(), tk.clone()); }
        if !gen_cfg.is_empty() {
            req["generationConfig"] = Value::Object(gen_cfg);
        }

        if !system_texts.is_empty() {
            let joined = system_texts.join("\n\n");
            req["systemInstruction"] = json!({
                "parts": [{ "text": joined }]
            });
        }

        Ok(req)
    }

    fn content_value_to_text(content: &Value) -> Option<String> {
        // If it's a string, return directly
        if let Some(s) = content.as_str() {
            return Some(s.to_string());
        }
        // If it's an array of blocks, try to extract text-like fields
        if let Some(arr) = content.as_array() {
            let mut acc = String::new();
            for v in arr {
                if let Some(t) = v.get("text").and_then(|x| x.as_str()) {
                    acc.push_str(t);
                } else if let Some(t) = v.get("content").and_then(|x| x.as_str()) {
                    acc.push_str(t);
                }
            }
            if !acc.is_empty() { return Some(acc); }
        }
        // Fallback: stringify
        Some(content.to_string())
    }

    /// Convert Chat Completions JSON response to Responses API format
    fn convert_chat_completion_to_responses_json(chat_response: &Value) -> Result<Value, (StatusCode, String)> {
        info!("Converting Chat Completions response to Responses format: {}", serde_json::to_string_pretty(chat_response).unwrap_or_default());
        // For now, let's just pass through the Chat Completions response
        // The OpenAI SDK seems to handle this format correctly
        Ok(chat_response.clone())
    }

    /// Convert Chat Completions streaming chunk to Responses API chunk
    fn convert_chat_chunk_to_responses_chunk(chat_chunk: &Value) -> Result<Value, String> {
        // Handle different types of streaming events
        if let Some(choices) = chat_chunk.get("choices").and_then(|c| c.as_array()) {
            if let Some(first_choice) = choices.first() {
                if let Some(delta) = first_choice.get("delta") {
                    if let Some(content) = delta.get("content") {
                        // This is a content delta - convert to response.output_text.delta
                        return Ok(json!({
                            "type": "response.output_text.delta",
                            "delta": content
                        }));
                    }
                }

                if let Some(finish_reason) = first_choice.get("finish_reason") {
                    if !finish_reason.is_null() {
                        // This is the end of the response
                        return Ok(json!({
                            "type": "response.completed",
                            "response": {
                                "id": chat_chunk.get("id").unwrap_or(&json!("response-unknown")),
                                "object": "response",
                                "created": chat_chunk.get("created").unwrap_or(&json!(0)),
                                "model": chat_chunk.get("model").unwrap_or(&json!("o3")),
                                "usage": chat_chunk.get("usage")
                            }
                        }));
                    }
                }
            }
        }

        // If this is the first chunk, send response.created
        if chat_chunk.get("id").is_some() && chat_chunk.get("choices").is_some() {
            return Ok(json!({
                "type": "response.created",
                "response": {
                    "id": chat_chunk.get("id").unwrap_or(&json!("response-unknown")),
                    "object": "response",
                    "created": chat_chunk.get("created").unwrap_or(&json!(0)),
                    "model": chat_chunk.get("model").unwrap_or(&json!("o3"))
                }
            }));
        }

        Err("Unknown chunk format".to_string())
    }
}
