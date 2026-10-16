//! Embedded key-value store (redb).

use std::{fmt, path::PathBuf, sync::Arc};

use redb::{Database, ReadableDatabase};

/// Failure to open the key-value database.
#[derive(Debug, thiserror::Error)]
pub enum KvError {
    /// redb could not open or create the database file.
    #[error("open key-value database")]
    Open(#[from] redb::DatabaseError),
    /// The blocking task that opens the file panicked or was cancelled.
    #[error("key-value task failed")]
    Task(#[from] tokio::task::JoinError),
}

/// Cheaply cloneable handle to the embedded key-value database.
#[derive(Clone)]
pub struct KvStore {
    db: Arc<Database>,
}

impl fmt::Debug for KvStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KvStore").finish_non_exhaustive()
    }
}

impl KvStore {
    /// Opens (creating if missing) the database file at `path`.
    pub async fn open(path: impl Into<PathBuf>) -> Result<Self, KvError> {
        let path = path.into();
        let db = tokio::task::spawn_blocking(move || Database::create(path)).await??;
        Ok(Self { db: Arc::new(db) })
    }

    /// Whether a read transaction can be started.
    pub async fn is_ready(&self) -> bool {
        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || db.begin_read().is_ok())
            .await
            .unwrap_or(false)
    }
}
