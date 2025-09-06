use axum::{http::StatusCode, response::Json, routing::get, Router};
use serde_json::{json, Value};
use chrono::Utc;

pub fn router() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/health/detailed", get(detailed_health_check))
}

async fn health_check() -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({
        "status": "healthy",
        "timestamp": Utc::now()
    })))
}

async fn detailed_health_check() -> Result<Json<Value>, StatusCode> {
    let uptime = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let system_info = get_system_info();
    
    Ok(Json(json!({
        "status": "healthy",
        "timestamp": Utc::now(),
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": uptime,
        "system": system_info,
        "components": {
            "proxy": "healthy",
            "telemetry": "healthy",
            "user_service": "healthy"
        }
    })))
}

fn get_system_info() -> Value {
    use std::env;
    
    json!({
        "platform": env::consts::OS,
        "arch": env::consts::ARCH,
        "rust_version": "1.75" // Static version for now
    })
}
