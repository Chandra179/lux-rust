use crate::helpers::spawn_app;

fn request_id(response: &reqwest::Response) -> String {
    response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned()
}

#[tokio::test]
async fn generates_a_request_id_when_none_is_sent() {
    let app = spawn_app().await;

    let response = app.client.get(app.url("/health")).send().await.unwrap();

    let id = request_id(&response);
    assert_eq!(id.len(), 32, "{id}");
    assert!(id.bytes().all(|b| b.is_ascii_hexdigit()), "{id}");
}

#[tokio::test]
async fn echoes_a_valid_request_id() {
    let app = spawn_app().await;

    let response = app
        .client
        .get(app.url("/health"))
        .header("x-request-id", "trace-123.a:b_c")
        .send()
        .await
        .unwrap();

    assert_eq!(request_id(&response), "trace-123.a:b_c");
}

#[tokio::test]
async fn replaces_an_invalid_request_id() {
    let app = spawn_app().await;

    let response = app
        .client
        .get(app.url("/health"))
        .header("x-request-id", "not valid!")
        .send()
        .await
        .unwrap();

    let id = request_id(&response);
    assert_ne!(id, "not valid!");
    assert_eq!(id.len(), 32);
}

#[tokio::test]
async fn request_id_is_set_on_unmatched_routes_too() {
    let app = spawn_app().await;

    let response = app.client.get(app.url("/nope")).send().await.unwrap();

    assert_eq!(response.status(), 404);
    assert!(response.headers().contains_key("x-request-id"));
}
