//! HTTP-facing error type. Handlers return `Result<_, AppError>`; this is the
//! one place that maps failures to status codes and response bodies.

use axum::{
    Json,
    extract::{FromRequest, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use example::ExampleError;
use serde::Serialize;

/// Everything a handler can fail with.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The request body or parameters were not acceptable (400).
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    /// The request body exceeded the configured limit (413).
    #[error("request body too large")]
    PayloadTooLarge,
    /// The request conflicts with existing state (409). The text is shown to
    /// the client, so keep it fixed and free of user input.
    #[error("conflict: {0}")]
    Conflict(&'static str),
    /// A database query failed (500).
    #[error("database error")]
    Database(#[from] sqlx::Error),
}

/// Body of every error response.
#[derive(Debug, Serialize)]
struct ErrorBody {
    /// Short, fixed description of what went wrong.
    error: &'static str,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            Self::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "invalid request"),
            Self::PayloadTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "request body too large"),
            Self::Conflict(message) => (StatusCode::CONFLICT, message),
            Self::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal error"),
        };
        (status, Json(ErrorBody { error })).into_response()
    }
}

impl From<ExampleError> for AppError {
    fn from(err: ExampleError) -> Self {
        match err {
            ExampleError::EmptyName => Self::InvalidRequest("name is required".to_owned()),
            ExampleError::ReservedName(_) => Self::Conflict("name is reserved"),
            ExampleError::Database(err) => Self::Database(err),
        }
    }
}

/// `Json` extractor that reports failures as an [`AppError`].
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(AppError))]
pub struct AppJson<T>(pub T);

impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        match rejection.status() {
            StatusCode::PAYLOAD_TOO_LARGE => Self::PayloadTooLarge,
            _ => Self::InvalidRequest(rejection.body_text()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_errors_to_status_codes() {
        let cases = [
            (
                AppError::InvalidRequest("x".to_owned()),
                StatusCode::BAD_REQUEST,
            ),
            (AppError::PayloadTooLarge, StatusCode::PAYLOAD_TOO_LARGE),
            (AppError::Conflict("taken"), StatusCode::CONFLICT),
            (
                AppError::Database(sqlx::Error::PoolTimedOut),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ];

        for (err, expected) in cases {
            assert_eq!(err.into_response().status(), expected);
        }
    }

    #[test]
    fn example_errors_map_to_http_categories() {
        let cases = [
            (ExampleError::EmptyName, StatusCode::BAD_REQUEST),
            (
                ExampleError::ReservedName("admin".to_owned()),
                StatusCode::CONFLICT,
            ),
            (
                ExampleError::Database(sqlx::Error::PoolTimedOut),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ];

        for (err, expected) in cases {
            assert_eq!(AppError::from(err).into_response().status(), expected);
        }
    }

    #[tokio::test]
    async fn client_sees_only_the_fixed_message() {
        let response = AppError::Database(sqlx::Error::PoolTimedOut).into_response();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();

        assert_eq!(&body[..], br#"{"error":"internal error"}"#);
    }
}
