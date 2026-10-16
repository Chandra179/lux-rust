//! Route table and middleware stack.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    middleware::{from_fn, from_fn_with_state},
    routing::get,
};
use serde_json::{Value, json};
use sqlx::SqlitePool;
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::CatchPanicLayer, limit::RequestBodyLimitLayer, timeout::TimeoutLayer,
};

use crate::{
    config::Config,
    middleware::{recovery, request_id, request_log},
    modules::example::{self, SqliteExampleStore},
    state::AppState,
    store::kv::KvStore,
    swagger,
};

/// Wires the modules to the shared stores and applies the middleware stack.
/// Performs no I/O, so tests can build the app against temporary stores.
pub fn build_router(db: SqlitePool, kv: KvStore, config: &Config) -> Router {
    let example = example::Service::new(Arc::new(SqliteExampleStore::new(db.clone())));
    let state = AppState { db, kv };

    // The first layer listed is the outermost: request IDs wrap everything,
    // the request log sees recovered panics as 500s, and the limits apply
    // closest to the handlers.
    let middleware = ServiceBuilder::new()
        .layer(from_fn(request_id::request_id))
        .layer(from_fn_with_state(
            Arc::new(config.middleware.request_log.clone()),
            request_log::request_log,
        ))
        .layer(CatchPanicLayer::custom(recovery::panic_response))
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(
            config.http.max_body_size_in_bytes,
        ))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            config.http.request_timeout(),
        ));

    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .with_state(state)
        .merge(swagger::routes())
        .merge(example::routes(example))
        .layer(middleware)
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn ready(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let db_ready = sqlx::query("SELECT 1").execute(&state.db).await.is_ok();
    let kv_ready = state.kv.is_ready().await;

    if db_ready && kv_ready {
        (StatusCode::OK, Json(json!({ "status": "ready" })))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "not_ready" })),
        )
    }
}
