use serde_json::Value;

use crate::helpers::spawn_app;

#[tokio::test]
async fn swagger_page_is_served() {
    let app = spawn_app().await;

    let response = app
        .client
        .get(app.url("/swagger/index.html"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert!(
        response.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    assert!(response.text().await.unwrap().contains("SwaggerUIBundle"));
}

#[tokio::test]
async fn swagger_root_redirects_to_the_page() {
    let app = spawn_app().await;

    // reqwest follows the redirect.
    let response = app.client.get(app.url("/swagger")).send().await.unwrap();

    assert_eq!(response.status(), 200);
    assert!(response.url().path().ends_with("/swagger/index.html"));
}

#[tokio::test]
async fn openapi_document_lists_the_example_route() {
    let app = spawn_app().await;

    let response = app
        .client
        .get(app.url("/swagger/openapi.json"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let doc: Value = response.json().await.unwrap();
    assert_eq!(doc["info"]["title"], "Lux API");
    assert!(doc["paths"]["/example"]["post"].is_object(), "{doc}");
}
