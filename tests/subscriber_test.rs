mod common;

use serde_json::json;
use sqlx::Row;

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
        .add_header("x-api-key", &app.api_key)
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
        .add_header("x-api-key", &app.api_key)
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
        .add_header("x-api-key", &app.api_key)
        .json(&json!({ "email": "dupe@example.com" }))
        .await;

    // Second creation returns same
    let response = app
        .server
        .post("/api/v1/subscribers")
        .add_header("x-api-key", &app.api_key)
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
        .add_header("x-api-key", &app.api_key)
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
        .add_header("x-api-key", &app.api_key)
        .json(&json!({ "email": "get@example.com" }))
        .await;
    let created: serde_json::Value = create_resp.json();
    let uuid = created["uuid"].as_str().unwrap();

    let response = app
        .server
        .get(&format!("/api/v1/subscribers/{}", uuid))
        .add_header("x-api-key", &app.api_key)
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
        .add_header("x-api-key", &app.api_key)
        .json(&json!({ "email": "update@example.com" }))
        .await;
    let created: serde_json::Value = create_resp.json();
    let uuid = created["uuid"].as_str().unwrap();

    let response = app
        .server
        .patch(&format!("/api/v1/subscribers/{}", uuid))
        .add_header("x-api-key", &app.api_key)
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
        .add_header("x-api-key", &app.api_key)
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
async fn resubmit_unconfirmed_subscriber_sends_new_login() {
    let app = common::TestApp::new().await;

    // First creation — generates login tokens
    app.server
        .post("/api/v1/subscribers")
        .add_header("x-api-key", &app.api_key)
        .json(&json!({ "email": "retry@example.com" }))
        .await;

    let count_after_first: i64 = sqlx::query(
        "SELECT COUNT(*) as count FROM logins l
         JOIN subscribers s ON l.subscriber_id = s.id
         WHERE s.email = 'retry@example.com'",
    )
    .fetch_one(&app.pool)
    .await
    .unwrap()
    .get("count");

    // Should have 2 logins (code + magic_link)
    assert_eq!(count_after_first, 2);

    // Second creation — subscriber is unconfirmed, should generate new logins
    let response = app
        .server
        .post("/api/v1/subscribers")
        .add_header("x-api-key", &app.api_key)
        .json(&json!({ "email": "retry@example.com" }))
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body["confirmed_at"].is_null());

    let count_after_second: i64 = sqlx::query(
        "SELECT COUNT(*) as count FROM logins l
         JOIN subscribers s ON l.subscriber_id = s.id
         WHERE s.email = 'retry@example.com'",
    )
    .fetch_one(&app.pool)
    .await
    .unwrap()
    .get("count");

    // Should now have 4 logins (2 from first + 2 from resend)
    assert_eq!(count_after_second, 4);
}

#[tokio::test]
async fn resubmit_confirmed_subscriber_does_not_resend() {
    let app = common::TestApp::new().await;

    // Create and confirm via google_verified
    app.server
        .post("/api/v1/subscribers")
        .add_header("x-api-key", &app.api_key)
        .json(&json!({
            "email": "confirmed@example.com",
            "google_verified": true
        }))
        .await;

    let count_before: i64 = sqlx::query(
        "SELECT COUNT(*) as count FROM logins l
         JOIN subscribers s ON l.subscriber_id = s.id
         WHERE s.email = 'confirmed@example.com'",
    )
    .fetch_one(&app.pool)
    .await
    .unwrap()
    .get("count");

    // Resubmit — already confirmed, should NOT create new logins
    app.server
        .post("/api/v1/subscribers")
        .add_header("x-api-key", &app.api_key)
        .json(&json!({ "email": "confirmed@example.com" }))
        .await;

    let count_after: i64 = sqlx::query(
        "SELECT COUNT(*) as count FROM logins l
         JOIN subscribers s ON l.subscriber_id = s.id
         WHERE s.email = 'confirmed@example.com'",
    )
    .fetch_one(&app.pool)
    .await
    .unwrap()
    .get("count");

    assert_eq!(count_before, count_after);
}

#[tokio::test]
async fn unsubscribe_subscriber() {
    let app = common::TestApp::new().await;
    let create_resp = app
        .server
        .post("/api/v1/subscribers")
        .add_header("x-api-key", &app.api_key)
        .json(&json!({ "email": "unsub@example.com" }))
        .await;
    let created: serde_json::Value = create_resp.json();
    let uuid = created["uuid"].as_str().unwrap();

    let response = app
        .server
        .post(&format!("/api/v1/subscribers/{}/unsubscribe", uuid))
        .add_header("x-api-key", &app.api_key)
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(!body["subscribed_postcard"].as_bool().unwrap());
    assert!(!body["subscribed_contraption"].as_bool().unwrap());
    assert!(!body["subscribed_workshop"].as_bool().unwrap());
}
