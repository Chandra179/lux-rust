use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// A persisted example record.
#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Example {
    pub id: String,
    pub name: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Request body of `POST /example`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateExampleRequest {
    pub name: String,
}
