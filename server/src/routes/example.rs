//! `POST /example`: HTTP front for the `example` crate.

use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::error::{AppError, AppJson};

/// Routes of this file.
pub fn router() -> Router<SqlitePool> {
    Router::new().route("/example", post(create_example))
}

/// Request body of `POST /example`.
#[derive(Debug, Deserialize)]
struct CreateExampleRequest {
    name: String,
}

/// Creates and persists an example record.
async fn create_example(
    State(db): State<SqlitePool>,
    AppJson(request): AppJson<CreateExampleRequest>,
) -> Result<(StatusCode, Json<::example::Example>), AppError> {
    let example = ::example::create(&db, &request.name).await?;
    Ok((StatusCode::CREATED, Json(example)))
}
