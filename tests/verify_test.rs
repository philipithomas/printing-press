mod common;

use chrono::{Duration, Utc};
use printing_press::models::login::Login;
use printing_press::models::subscriber::Subscriber;
use serde_json::json;

#[tokio::test]
async fn verify_valid_code() {
    let app = common::TestApp::new().await;

    // Create subscriber directly
    let sub = Subscriber::create(&app.pool, "verify@example.com", None, None)
        .await
        .unwrap();

    // Create a login token
    let expiry = Utc::now() + Duration::minutes(15);
    let _login = Login::create(&app.pool, sub.id, "123456", "code", expiry)
        .await
        .unwrap();

    let response = app
        .server
        .post("/api/v1/subscribers/verify")
        .add_header("x-api-key".parse().unwrap(), app.api_key.parse().unwrap())
        .json(&json!({ "token": "123456" }))
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body["confirmed_at"].as_str().is_some());
}

#[tokio::test]
async fn verify_expired_token() {
    let app = common::TestApp::new().await;

    let sub = Subscriber::create(&app.pool, "expired@example.com", None, None)
        .await
        .unwrap();

    // Create an already-expired token
    let expiry = Utc::now() - Duration::minutes(1);
    Login::create(&app.pool, sub.id, "expired123", "code", expiry)
        .await
        .unwrap();

    let response = app
        .server
        .post("/api/v1/subscribers/verify")
        .add_header("x-api-key".parse().unwrap(), app.api_key.parse().unwrap())
        .json(&json!({ "token": "expired123" }))
        .await;
    response.assert_status(axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn verify_invalid_token() {
    let app = common::TestApp::new().await;

    let response = app
        .server
        .post("/api/v1/subscribers/verify")
        .add_header("x-api-key".parse().unwrap(), app.api_key.parse().unwrap())
        .json(&json!({ "token": "nonexistent" }))
        .await;
    response.assert_status(axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn verify_already_used_token() {
    let app = common::TestApp::new().await;

    let sub = Subscriber::create(&app.pool, "used@example.com", None, None)
        .await
        .unwrap();

    let expiry = Utc::now() + Duration::minutes(15);
    Login::create(&app.pool, sub.id, "usedtoken", "code", expiry)
        .await
        .unwrap();

    // First verification
    app.server
        .post("/api/v1/subscribers/verify")
        .add_header("x-api-key".parse().unwrap(), app.api_key.parse().unwrap())
        .json(&json!({ "token": "usedtoken" }))
        .await;

    // Second verification should fail
    let response = app
        .server
        .post("/api/v1/subscribers/verify")
        .add_header("x-api-key".parse().unwrap(), app.api_key.parse().unwrap())
        .json(&json!({ "token": "usedtoken" }))
        .await;
    response.assert_status(axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn verify_magic_link_token() {
    let app = common::TestApp::new().await;

    let sub = Subscriber::create(&app.pool, "magic@example.com", None, None)
        .await
        .unwrap();

    let expiry = Utc::now() + Duration::minutes(15);
    Login::create(
        &app.pool,
        sub.id,
        "magic-link-uuid-token",
        "magic_link",
        expiry,
    )
    .await
    .unwrap();

    let response = app
        .server
        .post("/api/v1/subscribers/verify")
        .add_header("x-api-key".parse().unwrap(), app.api_key.parse().unwrap())
        .json(&json!({ "token": "magic-link-uuid-token" }))
        .await;
    response.assert_status_ok();
}
