//! HTTP-facing error type. Domain errors stay in their modules; this is the
//! one place that maps them to status codes and response bodies.

use std::{error::Error, sync::Arc};

use axum::{
    Json,
    extract::{FromRequest, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::modules::example::ExampleError;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("request body too large")]
    PayloadTooLarge,
    #[error("internal error: {0}")]
    Internal(String),
    #[error(transparent)]
    Example(#[from] ExampleError),
}

/// Body of every error response.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ErrorBody {
    pub error: String,
}

/// Full error detail, attached to the response extensions so the request log
/// can record it. It is never sent to the client.
#[derive(Debug, Clone)]
pub struct ErrorDetail(pub Arc<str>);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Self::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "invalid request"),
            Self::PayloadTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "request body too large"),
            Self::Internal(_) | Self::Example(ExampleError::Store(_)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error")
            }
            Self::Example(ExampleError::ReservedName(_)) => {
                (StatusCode::CONFLICT, "name is reserved")
            }
        };

        let body = ErrorBody {
            error: message.to_owned(),
        };
        let mut response = (status, Json(body)).into_response();
        response
            .extensions_mut()
            .insert(ErrorDetail(error_chain(&self).into()));
        response
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

/// Renders an error and its sources as `outer: inner: root`.
fn error_chain(err: &(dyn Error + 'static)) -> String {
    let mut message = err.to_string();
    let mut source = err.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    message
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
            (
                AppError::Internal("x".to_owned()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            (
                AppError::Example(ExampleError::ReservedName("admin".to_owned())),
                StatusCode::CONFLICT,
            ),
            (
                AppError::Example(ExampleError::Store(sqlx::Error::PoolTimedOut)),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ];

        for (err, expected) in cases {
            let response = err.into_response();
            assert_eq!(response.status(), expected);
            assert!(response.extensions().get::<ErrorDetail>().is_some());
        }
    }

    #[test]
    fn error_chain_includes_sources() {
        let err = AppError::Example(ExampleError::Store(sqlx::Error::PoolTimedOut));
        let chain = error_chain(&err);
        assert!(chain.starts_with("create example"), "{chain}");
        assert!(chain.contains("pool timed out"), "{chain}");
    }
}
