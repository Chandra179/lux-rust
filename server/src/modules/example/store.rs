//! Persistence port of the example module and its SQLite implementation.

use async_trait::async_trait;
use sqlx::SqlitePool;
use uuid::Uuid;

use super::types::Example;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ExampleStore: Send + Sync {
    async fn create(&self, name: &str) -> Result<Example, sqlx::Error>;
}

#[derive(Debug, Clone)]
pub struct SqliteExampleStore {
    pool: SqlitePool,
}

impl SqliteExampleStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ExampleStore for SqliteExampleStore {
    async fn create(&self, name: &str) -> Result<Example, sqlx::Error> {
        sqlx::query_as::<_, Example>(
            "INSERT INTO examples (id, name) VALUES (?, ?) RETURNING id, name, created_at",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(name)
        .fetch_one(&self.pool)
        .await
    }
}
