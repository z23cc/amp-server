use std::collections::HashMap;

use axum::{
    Json, Router,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

mod internal;
use internal::InternalRequest;
use tracing::debug;

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
        .route("/api/internal", post(internal))
}

async fn get_user_info() -> Json<serde_json::Value> {
    Json(json!(
        {
            "id": ulid::Ulid::new().to_string(),
            "username": "USER_001",
            "email": "user_001@any.com",
            "firstName": "Any",
            "lastName": "User",
            "emailVerified": true,
            "profilePictureUrl": "https://picsum.photos/200",
            "lastSignInAt": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "createdAt": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "updatedAt": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "siteAdmin": true
        }
    ))
}

async fn get_connections() -> Json<serde_json::Value> {
    Json(json!([]))
}

async fn sync_thread(Json(request): Json<SyncThreadRequest>) -> Json<serde_json::Value> {
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

async fn internal(Json(request): Json<InternalRequest>) -> Json<serde_json::Value> {
    match request.method.as_str() {
        "uploadThread" => {
            let thread_data = &request.params.thread;
            debug!("Received thread upload request: ID={}, Title={}, Message count={}", thread_data.id, thread_data.title, thread_data.messages.len());
            
            Json(json!({"ok": true}))
        }
        _ => {
            Json(json!({"ok": true}))
        }
    }
}