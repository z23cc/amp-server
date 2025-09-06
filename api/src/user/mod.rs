use std::collections::HashMap;

use axum::{
    Json, Router,
    routing::{get, post},
    extract::Query,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

mod internal;
use internal::InternalRequest;
use tracing::{debug, warn};

// Error reporting structures
#[derive(Debug, Serialize, Deserialize)]
struct ErrorReport {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
    stack: Option<String>,
    #[serde(rename = "threadId")]
    thread_id: Option<String>,
    #[serde(rename = "timestamp")]
    timestamp: Option<String>,
    #[serde(rename = "userAgent")]
    user_agent: Option<String>,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ThreadMeta {
    #[serde(rename = "id")]
    thread_id: Option<String>,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SyncThreadRequest {
    #[serde(rename = "threadVersions")]
    thread_versions: Vec<String>,
    #[serde(rename = "threadMetas")]
    thread_metas: Vec<Option<ThreadMeta>>,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

pub fn router() -> Router {
    Router::new()
        .route("/api/user", get(get_user_info))
        .route("/api/connections", get(get_connections))
        .route("/api/threads/sync", post(sync_thread))
        .route("/api/internal", post(handle_internal))
        .route("/api/errors", post(handle_error_report))
}

async fn get_user_info() -> Json<serde_json::Value> {
    debug!("User info requested");
    Json(json!(
        {
            "id": ulid::Ulid::new().to_string(),
            "username": "USER_001",
            "email": "user_001@any.com",
            "firstName": "Any",
            "lastName": "User",
            "displayName": "Any User",
            "emailVerified": true,
            "profilePictureUrl": "https://picsum.photos/200",
            "lastSignInAt": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "createdAt": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "updatedAt": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "siteAdmin": true,
            "subscriptions": [],
            "plan": {
                "type": "free",
                "name": "Free Plan",
                "limits": {
                    "monthlyUsage": 0,
                    "monthlyLimit": 100
                }
            }
        }
    ))
}

async fn get_connections() -> Json<serde_json::Value> {
    debug!("Connections requested");
    Json(json!([
        {
            "id": ulid::Ulid::new().to_string(),
            "name": "GitHub",
            "type": "github",
            "status": "connected",
            "connectedAt": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "username": "USER_001"
        },
        {
            "id": ulid::Ulid::new().to_string(),
            "name": "GitLab",
            "type": "gitlab",
            "status": "disconnected",
            "connectedAt": null,
            "username": null
        }
    ]))
}

async fn sync_thread(Json(request): Json<SyncThreadRequest>) -> Json<serde_json::Value> {
    debug!("Sync thread request: thread_versions={:?}", request.thread_versions);
    
    let thread_id = if let Some(Some(thread_meta)) = request.thread_metas.first()
        && let Some(thread_id) = &thread_meta.thread_id
    {
        thread_id
    } else {
        return Json(json!(
            {
                "threadActions": [],
            }
        ));
    };
    Json(json!(
        {
            "threadActions": [
                {
                    "id": thread_id,
                    "action": "meta",
                    "meta": {
                        "private": false,
                        "public": false,
                    }
                }
            ],
        }
    ))
}

async fn handle_internal(
    Query(params): Query<HashMap<String, String>>,
    Json(request): Json<InternalRequest>
) -> Json<serde_json::Value> {
    let method = params.get("method").map(|s| s.as_str()).unwrap_or(&request.method);
    
    debug!("Internal API call: method={}", method);
    
    match method {
        "uploadThread" => {
            let thread_data = &request.params.thread;
            debug!("Received thread upload request: ID={}, Title={}, Message count={}", 
                thread_data.id, thread_data.title, thread_data.messages.len());
            
            Json(json!({"ok": true}))
        }
        "getUser" => {
            Json(json!({
                "id": ulid::Ulid::new().to_string(),
                "username": "USER_001",
                "email": "user_001@any.com",
                "firstName": "Any",
                "lastName": "User",
                "emailVerified": true
            }))
        }
        _ => {
            debug!("Unknown internal method: {}", method);
            Json(json!({"ok": true}))
        }
    }
}

async fn handle_error_report(Json(error_report): Json<ErrorReport>) -> Json<serde_json::Value> {
    warn!("Client error reported: type={}, message={}", 
        error_report.error_type, error_report.message);
    
    if let Some(stack) = &error_report.stack {
        warn!("Error stack trace:\n{}", stack);
    }
    
    if let Some(thread_id) = &error_report.thread_id {
        warn!("Error occurred in thread: {}", thread_id);
    }
    
    Json(json!({"status": "received"}))
}