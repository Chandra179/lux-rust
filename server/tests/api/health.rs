use serde_json::{Value, json};

use crate::helpers::spawn_app;

#[tokio::test]
async fn health_reports_ok() {
    let app = spawn_app().await;

    let response = app.client.get(app.url("/health")).send().await.unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(
        response.json::<Value>().await.unwrap(),
        json!({ "status": "ok" })
    );
}

#[tokio::test]
async fn ready_reports_ready_when_stores_are_up() {
    let app = spawn_app().await;

    let response = app.client.get(app.url("/ready")).send().await.unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(
        response.json::<Value>().await.unwrap(),
        json!({ "status": "ready" })
    );
}

#[tokio::test]
async fn unknown_routes_return_404() {
    let app = spawn_app().await;

    let response = app.client.get(app.url("/nope")).send().await.unwrap();

    assert_eq!(response.status(), 404);
}
