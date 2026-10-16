//! Shared infrastructure handed to handlers that need it.

use sqlx::SqlitePool;

use crate::store::kv::KvStore;

#[derive(Debug, Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub kv: KvStore,
}
