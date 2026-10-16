//! OpenAPI document and Swagger UI page.
//!
//! The page loads the Swagger UI assets from a CDN, so viewing it needs
//! internet access. (The `utoipa-swagger-ui` crate would embed them, but every
//! release currently depends on a yanked `zip` version and cannot resolve.)

use axum::{
    Json, Router,
    response::{Html, Redirect},
    routing::get,
};
use utoipa::OpenApi;

use crate::modules::example;

const SWAGGER_UI_VERSION: &str = "5.33.3";

#[derive(OpenApi)]
#[openapi(info(title = "Lux API", description = "Lux API server."))]
struct ApiDoc;

/// The full OpenAPI document: shared info plus every module's fragment.
pub fn openapi() -> utoipa::openapi::OpenApi {
    let mut doc = ApiDoc::openapi();
    doc.info.version = env!("CARGO_PKG_VERSION").to_owned();
    // utoipa fills in an empty license object by default; drop it.
    doc.info.license = None;
    doc.merge(example::openapi());
    doc
}

/// `/swagger` (redirect), `/swagger/index.html`, `/swagger/openapi.json`.
pub fn routes() -> Router {
    Router::new()
        .route(
            "/swagger",
            get(|| async { Redirect::temporary("/swagger/index.html") }),
        )
        .route(
            "/swagger/",
            get(|| async { Redirect::temporary("/swagger/index.html") }),
        )
        .route("/swagger/index.html", get(|| async { Html(index_html()) }))
        .route("/swagger/openapi.json", get(|| async { Json(openapi()) }))
}

fn index_html() -> String {
    format!(
        r##"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Lux API</title>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/swagger-ui-dist@{v}/swagger-ui.css" />
  </head>
  <body>
    <div id="swagger-ui"></div>
    <script src="https://cdn.jsdelivr.net/npm/swagger-ui-dist@{v}/swagger-ui-bundle.js" crossorigin></script>
    <script>
      window.onload = () => {{
        window.ui = SwaggerUIBundle({{ url: "openapi.json", dom_id: "#swagger-ui" }});
      }};
    </script>
  </body>
</html>
"##,
        v = SWAGGER_UI_VERSION
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_describes_the_example_route() {
        let doc = openapi();
        assert_eq!(doc.info.title, "Lux API");
        assert!(doc.info.license.is_none());
        assert!(doc.paths.paths.contains_key("/example"));
    }

    #[test]
    fn index_page_pins_the_ui_version() {
        let html = index_html();
        assert!(html.contains(&format!(
            "swagger-ui-dist@{SWAGGER_UI_VERSION}/swagger-ui-bundle.js"
        )));
        assert!(html.contains(r#"url: "openapi.json""#));
    }
}
