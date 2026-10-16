//! HTTP routes, one file per resource. A resource that outgrows its file
//! becomes `routes/<name>.rs` plus a `routes/<name>/` folder of submodules.

mod example;
mod health;

use axum::Router;
use sqlx::SqlitePool;

/// All application routes. The database pool is the shared state; it is
/// attached once, in `startup::build_router`.
pub fn router() -> Router<SqlitePool> {
    Router::new()
        .merge(health::router())
        .merge(example::router())
}
