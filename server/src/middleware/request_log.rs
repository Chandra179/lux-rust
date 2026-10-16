//! One canonical log line per HTTP request.
//!
//! The level follows the response status (info for 2xx/3xx, warn for 4xx,
//! error for 5xx). Request and response bodies are never logged; the query
//! string is logged only for allow-listed keys, with sensitive values
//! redacted.

use std::{sync::Arc, time::Instant};

use axum::{
    extract::{MatchedPath, Request, State},
    middleware::Next,
    response::Response,
};

use super::request_id::RequestId;
use crate::{config::RequestLogConfig, error::ErrorDetail};

/// Substrings that mark a query key as sensitive.
const SENSITIVE_FRAGMENTS: [&str; 8] = [
    "auth",
    "code",
    "key",
    "password",
    "secret",
    "signature",
    "state",
    "token",
];

pub async fn request_log(
    State(config): State<Arc<RequestLogConfig>>,
    request: Request,
    next: Next,
) -> Response {
    if config
        .skip_paths
        .iter()
        .any(|skipped| skipped == request.uri().path())
    {
        return next.run(request).await;
    }

    let started = Instant::now();
    let method = request.method().clone();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or("<unmatched>", MatchedPath::as_str)
        .to_owned();
    let request_id = request
        .extensions()
        .get::<RequestId>()
        .map(|id| id.0.clone());
    let query = request
        .uri()
        .query()
        .filter(|_| config.log_query)
        .map(|query| query_for_log(query, &config.query_allowlist))
        .filter(|query| !query.is_empty());

    let response = next.run(request).await;

    let status = response.status();
    let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let error = response
        .extensions()
        .get::<ErrorDetail>()
        .map(|detail| Arc::clone(&detail.0));

    macro_rules! emit {
        ($level:ident) => {
            tracing::$level!(
                method = %method,
                path = %route,
                status = status.as_u16(),
                duration_ms,
                request_id = request_id.as_deref(),
                query = query.as_deref(),
                error = error.as_deref(),
                "request completed"
            )
        };
    }

    if status.is_server_error() {
        emit!(error);
    } else if status.is_client_error() {
        emit!(warn);
    } else {
        emit!(info);
    }

    response
}

/// Keeps only allow-listed keys (sorted by key), redacting sensitive values.
fn query_for_log(query: &str, allowlist: &[String]) -> String {
    let mut pairs: Vec<(String, String)> = form_urlencoded::parse(query.as_bytes())
        .filter(|(key, _)| allowlist.iter().any(|allowed| allowed == key.as_ref()))
        .map(|(key, value)| {
            let value = if is_sensitive(&key) {
                "[REDACTED]".into()
            } else {
                value
            };
            (key.into_owned(), value.into_owned())
        })
        .collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));

    form_urlencoded::Serializer::new(String::new())
        .extend_pairs(pairs)
        .finish()
}

fn is_sensitive(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    SENSITIVE_FRAGMENTS
        .iter()
        .any(|fragment| key.contains(fragment))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allow(keys: &[&str]) -> Vec<String> {
        keys.iter().map(|key| (*key).to_owned()).collect()
    }

    #[test]
    fn keeps_only_allow_listed_keys_sorted() {
        let query = "limit=10&secret_thing=x&page=2&other=1";
        let logged = query_for_log(query, &allow(&["page", "limit"]));
        assert_eq!(logged, "limit=10&page=2");
    }

    #[test]
    fn redacts_sensitive_keys_even_when_allow_listed() {
        let logged = query_for_log("access_token=abc&page=1", &allow(&["access_token", "page"]));
        assert_eq!(logged, "access_token=%5BREDACTED%5D&page=1");
    }

    #[test]
    fn keeps_repeated_keys() {
        let logged = query_for_log("page=1&page=2", &allow(&["page"]));
        assert_eq!(logged, "page=1&page=2");
    }

    #[test]
    fn empty_when_nothing_is_allowed() {
        assert_eq!(query_for_log("a=1&b=2", &allow(&["page"])), "");
    }

    #[test]
    fn sensitive_key_detection_is_case_insensitive() {
        assert!(is_sensitive("API_KEY"));
        assert!(is_sensitive("Authorization"));
        assert!(!is_sensitive("page"));
    }
}
