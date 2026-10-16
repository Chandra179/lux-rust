use axum::{Json, extract::State, http::StatusCode};

use super::{CreateExampleRequest, Example, Service};
use crate::error::{AppError, AppJson};

/// Creates and persists an example record.
#[utoipa::path(
    post,
    path = "/example",
    tag = "example",
    request_body = CreateExampleRequest,
    responses(
        (status = 201, description = "Example created", body = Example),
        (status = 400, description = "Invalid request", body = crate::error::ErrorBody),
        (status = 409, description = "Name is reserved", body = crate::error::ErrorBody),
        (status = 500, description = "Internal error", body = crate::error::ErrorBody),
    )
)]
pub(super) async fn create_example(
    State(service): State<Service>,
    AppJson(request): AppJson<CreateExampleRequest>,
) -> Result<(StatusCode, Json<Example>), AppError> {
    if request.name.is_empty() {
        return Err(AppError::InvalidRequest("name is required".to_owned()));
    }

    let example = service.create(&request.name).await?;
    Ok((StatusCode::CREATED, Json(example)))
}
