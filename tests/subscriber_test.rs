mod common;

use serde_json::json;

#[tokio::test]
async fn create_subscriber_requires_auth() {
    let app = common::TestApp::new().await;
    let response = app
        .server
        .post("/api/v1/subscribers")
        .json(&json!({ "email": "test@example.com" }))
        .await;
    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_subscriber_with_valid_key() {
    let app = common::TestApp::new().await;
    let response = app
        .server
        .post("/api/v1/subscribers")
        .add_header("x-api-key".parse().unwrap(), app.api_key.parse().unwrap())
        .json(&json!({ "email": "test@example.com" }))
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["email"], "test@example.com");
    assert!(body["subscribed_postcard"].as_bool().unwrap());
    assert!(body["subscribed_contraption"].as_bool().unwrap());
    assert!(body["subscribed_workshop"].as_bool().unwrap());
}

#[tokio::test]
async fn create_subscriber_normalizes_email() {
    let app = common::TestApp::new().await;
    let response = app
        .server
        .post("/api/v1/subscribers")
        .add_header("x-api-key".parse().unwrap(), app.api_key.parse().unwrap())
        .json(&json!({ "email": "TEST@Example.COM" }))
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["email"], "test@example.com");
}

#[tokio::test]
async fn create_subscriber_duplicate_returns_existing() {
    let app = common::TestApp::new().await;

    // First creation
    app.server
        .post("/api/v1/subscribers")
        .add_header("x-api-key".parse().unwrap(), app.api_key.parse().unwrap())
        .json(&json!({ "email": "dupe@example.com" }))
        .await;

    // Second creation returns same
    let response = app
        .server
        .post("/api/v1/subscribers")
        .add_header("x-api-key".parse().unwrap(), app.api_key.parse().unwrap())
        .json(&json!({ "email": "dupe@example.com" }))
        .await;
    response.assert_status_ok();
}

#[tokio::test]
async fn create_subscriber_invalid_email() {
    let app = common::TestApp::new().await;
    let response = app
        .server
        .post("/api/v1/subscribers")
        .add_header("x-api-key".parse().unwrap(), app.api_key.parse().unwrap())
        .json(&json!({ "email": "not-an-email" }))
        .await;
    response.assert_status(axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_subscriber_by_uuid() {
    let app = common::TestApp::new().await;
    let create_resp = app
        .server
        .post("/api/v1/subscribers")
        .add_header("x-api-key".parse().unwrap(), app.api_key.parse().unwrap())
        .json(&json!({ "email": "get@example.com" }))
        .await;
    let created: serde_json::Value = create_resp.json();
    let uuid = created["uuid"].as_str().unwrap();

    let response = app
        .server
        .get(&format!("/api/v1/subscribers/{}", uuid))
        .add_header("x-api-key".parse().unwrap(), app.api_key.parse().unwrap())
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["email"], "get@example.com");
}

#[tokio::test]
async fn update_subscriber_preferences() {
    let app = common::TestApp::new().await;
    let create_resp = app
        .server
        .post("/api/v1/subscribers")
        .add_header("x-api-key".parse().unwrap(), app.api_key.parse().unwrap())
        .json(&json!({ "email": "update@example.com" }))
        .await;
    let created: serde_json::Value = create_resp.json();
    let uuid = created["uuid"].as_str().unwrap();

    let response = app
        .server
        .patch(&format!("/api/v1/subscribers/{}", uuid))
        .add_header("x-api-key".parse().unwrap(), app.api_key.parse().unwrap())
        .json(&json!({
            "subscribed_workshop": false,
            "name": "Philip"
        }))
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(!body["subscribed_workshop"].as_bool().unwrap());
    assert_eq!(body["name"], "Philip");
}

#[tokio::test]
async fn google_verified_subscriber_is_confirmed() {
    let app = common::TestApp::new().await;
    let response = app
        .server
        .post("/api/v1/subscribers")
        .add_header("x-api-key".parse().unwrap(), app.api_key.parse().unwrap())
        .json(&json!({
            "email": "google@example.com",
            "name": "Google User",
            "google_verified": true
        }))
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body["confirmed_at"].as_str().is_some());
    assert_eq!(body["name"], "Google User");
}

#[tokio::test]
async fn unsubscribe_subscriber() {
    let app = common::TestApp::new().await;
    let create_resp = app
        .server
        .post("/api/v1/subscribers")
        .add_header("x-api-key".parse().unwrap(), app.api_key.parse().unwrap())
        .json(&json!({ "email": "unsub@example.com" }))
        .await;
    let created: serde_json::Value = create_resp.json();
    let uuid = created["uuid"].as_str().unwrap();

    let response = app
        .server
        .post(&format!("/api/v1/subscribers/{}/unsubscribe", uuid))
        .add_header("x-api-key".parse().unwrap(), app.api_key.parse().unwrap())
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(!body["subscribed_postcard"].as_bool().unwrap());
    assert!(!body["subscribed_contraption"].as_bool().unwrap());
    assert!(!body["subscribed_workshop"].as_bool().unwrap());
}
