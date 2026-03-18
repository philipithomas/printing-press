mod common;

use serde_json::json;

#[tokio::test]
async fn subscriber_count_returns_zero_with_no_subscribers() {
    let app = common::TestApp::new().await;
    let response = app.server.get("/api/v1/stats/subscribers/count").await;
    response.assert_status_ok();
    response.assert_json_contains(&json!({ "count": 0 }));
}

#[tokio::test]
async fn subscriber_count_excludes_unconfirmed() {
    let app = common::TestApp::new().await;

    // Create an unconfirmed subscriber
    app.server
        .post("/api/v1/subscribers")
        .add_header("x-api-key", &app.api_key)
        .json(&json!({ "email": "unconfirmed@example.com" }))
        .await;

    let response = app.server.get("/api/v1/stats/subscribers/count").await;
    response.assert_status_ok();
    response.assert_json_contains(&json!({ "count": 0 }));
}

#[tokio::test]
async fn subscriber_count_excludes_fully_unsubscribed() {
    let app = common::TestApp::new().await;

    // Create a confirmed subscriber (via google_verified), then unsubscribe all
    let create_resp = app
        .server
        .post("/api/v1/subscribers")
        .add_header("x-api-key", &app.api_key)
        .json(&json!({
            "email": "unsub@example.com",
            "google_verified": true
        }))
        .await;
    let created: serde_json::Value = create_resp.json();
    let uuid = created["uuid"].as_str().unwrap();

    app.server
        .post(&format!("/api/v1/subscribers/{}/unsubscribe", uuid))
        .add_header("x-api-key", &app.api_key)
        .await;

    let response = app.server.get("/api/v1/stats/subscribers/count").await;
    response.assert_status_ok();
    response.assert_json_contains(&json!({ "count": 0 }));
}

#[tokio::test]
async fn subscriber_count_includes_confirmed_and_subscribed() {
    let app = common::TestApp::new().await;

    // Create a confirmed subscriber via google_verified
    app.server
        .post("/api/v1/subscribers")
        .add_header("x-api-key", &app.api_key)
        .json(&json!({
            "email": "active@example.com",
            "google_verified": true
        }))
        .await;

    let response = app.server.get("/api/v1/stats/subscribers/count").await;
    response.assert_status_ok();
    response.assert_json_contains(&json!({ "count": 1 }));
}

#[tokio::test]
async fn subscriber_count_does_not_require_auth() {
    let app = common::TestApp::new().await;
    // No x-api-key header — should still work
    let response = app.server.get("/api/v1/stats/subscribers/count").await;
    response.assert_status_ok();
}
