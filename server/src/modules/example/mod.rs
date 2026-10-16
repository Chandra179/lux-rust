//! Example module: creates and persists example records.

mod error;
mod handler;
mod store;
mod types;

use std::{fmt, sync::Arc};

use axum::{Router, routing::post};
use utoipa::OpenApi;

use crate::error::ErrorBody;

pub use error::ExampleError;
pub use store::{ExampleStore, SqliteExampleStore};
pub use types::{CreateExampleRequest, Example};

/// Names that cannot be used for an example.
const RESERVED_NAME: &str = "admin";

/// The module's wired dependencies and business logic. Handlers receive it as
/// axum state.
#[derive(Clone)]
pub struct Service {
    store: Arc<dyn ExampleStore>,
}

impl fmt::Debug for Service {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Service").finish_non_exhaustive()
    }
}

impl Service {
    pub fn new(store: Arc<dyn ExampleStore>) -> Self {
        Self { store }
    }

    /// Creates an example, rejecting reserved names.
    pub async fn create(&self, name: &str) -> Result<Example, ExampleError> {
        if name == RESERVED_NAME {
            return Err(ExampleError::ReservedName(name.to_owned()));
        }
        Ok(self.store.create(name).await?)
    }
}

/// Routes owned by this module.
pub fn routes(service: Service) -> Router {
    Router::new()
        .route("/example", post(handler::create_example))
        .with_state(service)
}

#[derive(OpenApi)]
#[openapi(
    paths(handler::create_example),
    components(schemas(Example, CreateExampleRequest, ErrorBody))
)]
struct Api;

/// OpenAPI fragment describing this module's routes and schemas.
pub fn openapi() -> utoipa::openapi::OpenApi {
    Api::openapi()
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;

    use super::*;
    use store::MockExampleStore;

    #[tokio::test]
    async fn rejects_reserved_name_without_touching_the_store() {
        // A mock with no expectations panics if the store is called.
        let service = Service::new(Arc::new(MockExampleStore::new()));

        let err = service.create("admin").await.unwrap_err();

        assert!(matches!(err, ExampleError::ReservedName(name) if name == "admin"));
    }

    #[tokio::test]
    async fn returns_the_stored_example() {
        let mut store = MockExampleStore::new();
        store
            .expect_create()
            .withf(|name| name == "alpha")
            .times(1)
            .returning(|name| {
                Ok(Example {
                    id: "id-1".to_owned(),
                    name: name.to_owned(),
                    created_at: OffsetDateTime::UNIX_EPOCH,
                })
            });
        let service = Service::new(Arc::new(store));

        let example = service.create("alpha").await.unwrap();

        assert_eq!(example.id, "id-1");
        assert_eq!(example.name, "alpha");
    }

    #[tokio::test]
    async fn wraps_store_failures() {
        let mut store = MockExampleStore::new();
        store
            .expect_create()
            .returning(|_| Err(sqlx::Error::PoolTimedOut));
        let service = Service::new(Arc::new(store));

        let err = service.create("alpha").await.unwrap_err();

        assert!(matches!(err, ExampleError::Store(_)));
    }
}
