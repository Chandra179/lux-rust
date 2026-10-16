//! Request ID propagation: reuse a valid inbound `X-Request-ID` or generate
//! one, expose it to later layers, and echo it in the response.

use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

pub const HEADER: HeaderName = HeaderName::from_static("x-request-id");

const MAX_LEN: usize = 128;

/// The request's ID, stored in the request extensions.
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

pub async fn request_id(mut request: Request, next: Next) -> Response {
    let id = request
        .headers()
        .get(&HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| is_valid(value))
        .map_or_else(|| Uuid::new_v4().simple().to_string(), str::to_owned);

    request.extensions_mut().insert(RequestId(id.clone()));

    let mut response = next.run(request).await;
    if let Ok(value) = HeaderValue::from_str(&id) {
        response.headers_mut().insert(HEADER, value);
    }
    response
}

/// IDs are 1..=128 characters of `[A-Za-z0-9._:-]`.
fn is_valid(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_LEN
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_allowed_characters() {
        for id in ["abc", "ABC-123", "a.b_c:d", &"x".repeat(MAX_LEN)] {
            assert!(is_valid(id), "{id}");
        }
    }

    #[test]
    fn rejects_empty_long_and_unsafe_ids() {
        for id in [
            "",
            "has space",
            "new\nline",
            "semi;colon",
            "ünï",
            &"x".repeat(MAX_LEN + 1),
        ] {
            assert!(!is_valid(id), "{id:?}");
        }
    }
}
