use serde_json::{Value, json};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::helpers::{TestApp, spawn_app};

async fn post_example(app: &TestApp, body: &Value) -> reqwest::Response {
    app.client
        .post(app.url("/example"))
        .json(body)
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn create_returns_the_persisted_example() {
    let app = spawn_app().await;

    let response = post_example(&app, &json!({ "name": "alpha" })).await;

    assert_eq!(response.status(), 201);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["name"], "alpha");
    assert_eq!(
        body["id"].as_str().unwrap().len(),
        36,
        "id should be a UUID: {body}"
    );
    let created_at = body["created_at"].as_str().unwrap();
    OffsetDateTime::parse(created_at, &Rfc3339).unwrap();
}

#[tokio::test]
async fn create_generates_distinct_ids() {
    let app = spawn_app().await;

    let first: Value = post_example(&app, &json!({ "name": "a" }))
        .await
        .json()
        .await
        .unwrap();
    let second: Value = post_example(&app, &json!({ "name": "b" }))
        .await
        .json()
        .await
        .unwrap();

    assert_ne!(first["id"], second["id"]);
}

#[tokio::test]
async fn reserved_name_is_a_conflict() {
    let app = spawn_app().await;

    let response = post_example(&app, &json!({ "name": "admin" })).await;

    assert_eq!(response.status(), 409);
    assert_eq!(
        response.json::<Value>().await.unwrap(),
        json!({ "error": "name is reserved" })
    );
}

#[tokio::test]
async fn invalid_bodies_are_bad_requests() {
    let app = spawn_app().await;

    for body in [
        json!({}),
        json!({ "name": "" }),
        json!({ "name": 7 }),
        json!([]),
    ] {
        let response = post_example(&app, &body).await;
        assert_eq!(response.status(), 400, "body: {body}");
        assert_eq!(
            response.json::<Value>().await.unwrap(),
            json!({ "error": "invalid request" }),
            "body: {body}"
        );
    }
}

#[tokio::test]
async fn malformed_json_is_a_bad_request() {
    let app = spawn_app().await;

    let response = app
        .client
        .post(app.url("/example"))
        .header("content-type", "application/json")
        .body("{not json")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn oversized_bodies_are_rejected() {
    let app = spawn_app().await;
    // The dev config limits bodies to 1 MiB.
    let body = json!({ "name": "x".repeat(2 * 1024 * 1024) });

    let response = post_example(&app, &body).await;

    assert_eq!(response.status(), 413);
}
