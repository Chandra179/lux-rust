//! Liveness (`/health`) and readiness (`/ready`) probes.

use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde_json::{Value, json};
use sqlx::SqlitePool;

/// Routes of this file.
pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
}

/// The process is up.
async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

/// The process can reach the database.
async fn ready(State(db): State<SqlitePool>) -> (StatusCode, Json<Value>) {
    if sqlx::query("SELECT 1").execute(&db).await.is_ok() {
        (StatusCode::OK, Json(json!({ "status": "ready" })))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "not_ready" })),
        )
    }
}
