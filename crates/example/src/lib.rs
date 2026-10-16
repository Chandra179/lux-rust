//! The example feature: its data type, rules, and SQL.
//!
//! This crate knows nothing about HTTP. The server does not use it yet.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::{error::Error, fmt};

use serde::Serialize;
use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

/// Names that cannot be used for an example.
const RESERVED_NAME: &str = "admin";

/// A persisted example record.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Example {
    /// Unique ID (UUID v4).
    pub id: String,
    /// Name given at creation.
    pub name: String,
    /// When the record was created (RFC 3339, UTC).
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Why an example could not be created.
#[derive(Debug)]
pub enum ExampleError {
    /// The name is empty.
    EmptyName,
    /// The name is on the reserved list.
    ReservedName(String),
    /// The database query failed.
    Database(sqlx::Error),
}

impl fmt::Display for ExampleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => f.write_str("name is required"),
            Self::ReservedName(name) => write!(f, "name {name:?} is reserved"),
            Self::Database(_) => f.write_str("create example"),
        }
    }
}

impl Error for ExampleError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Database(err) => Some(err),
            _ => None,
        }
    }
}

impl From<sqlx::Error> for ExampleError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(err)
    }
}

/// Validates `name` and stores a new example.
///
/// # Errors
///
/// [`ExampleError::EmptyName`] or [`ExampleError::ReservedName`] for a bad
/// name, [`ExampleError::Database`] if the insert fails.
pub async fn create(db: &SqlitePool, name: &str) -> Result<Example, ExampleError> {
    validate_name(name)?;
    Ok(insert(db, name).await?)
}

fn validate_name(name: &str) -> Result<(), ExampleError> {
    if name.is_empty() {
        return Err(ExampleError::EmptyName);
    }
    if name == RESERVED_NAME {
        return Err(ExampleError::ReservedName(name.to_owned()));
    }
    Ok(())
}

async fn insert(db: &SqlitePool, name: &str) -> Result<Example, sqlx::Error> {
    sqlx::query_as::<_, Example>(
        "INSERT INTO examples (id, name) VALUES (?, ?) RETURNING id, name, created_at",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(name)
    .fetch_one(db)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_an_empty_name() {
        assert!(matches!(validate_name(""), Err(ExampleError::EmptyName)));
    }

    #[test]
    fn rejects_the_reserved_name() {
        assert!(matches!(
            validate_name("admin"),
            Err(ExampleError::ReservedName(name)) if name == "admin"
        ));
    }

    #[test]
    fn accepts_other_names() {
        assert!(validate_name("alpha").is_ok());
        assert!(validate_name("Admin").is_ok());
    }

    // `#[sqlx::test]` gives each test its own SQLite database with the
    // server's migrations already applied.
    #[sqlx::test(migrations = "../../server/migrations")]
    async fn create_persists_a_row(db: SqlitePool) {
        let created = create(&db, "alpha").await.unwrap();

        assert_eq!(created.name, "alpha");
        assert_eq!(created.id.len(), 36, "id should be a UUID: {}", created.id);

        let (stored,): (String,) = sqlx::query_as("SELECT name FROM examples WHERE id = ?")
            .bind(&created.id)
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(stored, "alpha");
    }

    #[sqlx::test(migrations = "../../server/migrations")]
    async fn create_generates_distinct_ids(db: SqlitePool) {
        let first = create(&db, "a").await.unwrap();
        let second = create(&db, "b").await.unwrap();

        assert_ne!(first.id, second.id);
    }

    #[sqlx::test(migrations = "../../server/migrations")]
    async fn a_rejected_name_stores_nothing(db: SqlitePool) {
        assert!(create(&db, "admin").await.is_err());

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM examples")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }
}
