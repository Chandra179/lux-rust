//! Turns handler panics into 500 responses.

use std::any::Any;

use axum::response::{IntoResponse, Response};

use crate::error::AppError;

/// Response factory for `CatchPanicLayer::custom`. Logs the panic message and
/// answers with the standard internal-error body; the request log records the
/// detail alongside method, route, and request ID.
pub fn panic_response(panic: Box<dyn Any + Send + 'static>) -> Response {
    let message = panic
        .downcast_ref::<&str>()
        .map(|message| (*message).to_owned())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic payload".to_owned());

    tracing::error!(panic = %message, "panic recovered");
    AppError::Internal(format!("panic: {message}")).into_response()
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::*;
    use crate::error::ErrorDetail;

    #[test]
    fn str_and_string_payloads_become_500_with_detail() {
        let payloads: [Box<dyn Any + Send>; 2] = [Box::new("boom"), Box::new("bang".to_owned())];

        for (payload, expected) in payloads.into_iter().zip(["boom", "bang"]) {
            let response = panic_response(payload);

            assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
            let detail = response.extensions().get::<ErrorDetail>().unwrap();
            assert!(detail.0.contains(expected), "{}", detail.0);
        }
    }

    #[test]
    fn unknown_payloads_still_become_500() {
        let response = panic_response(Box::new(42_u8));

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
