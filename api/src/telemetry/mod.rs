use std::collections::HashMap;

use axum::{
    Json, Router,
    routing::post,
};
use serde_json::json;

type TelemetryEvent = Vec<HashMap<String, serde_json::Value>>;

pub fn router() -> Router {
    Router::new()
        .route("/api/telemetry", post(telemetry))
}

async fn telemetry(Json(request): Json<TelemetryEvent>) -> Json<serde_json::Value> {
    Json(json!({ "message": "ok", "published": request.len() }))
}   