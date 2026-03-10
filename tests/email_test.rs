mod common;

use serde_json::json;

#[tokio::test]
async fn send_email_requires_auth() {
    let app = common::TestApp::new().await;
    let response = app
        .server
        .post("/api/v1/emails/send")
        .json(&json!({
            "subscriber_uuid": "00000000-0000-0000-0000-000000000000",
            "post_slug": "test",
            "subject": "Test",
            "html_content": "<p>Hello</p>"
        }))
        .await;
    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn send_email_subscriber_not_found() {
    let app = common::TestApp::new().await;
    let response = app
        .server
        .post("/api/v1/emails/send")
        .add_header("x-api-key".parse().unwrap(), app.api_key.parse().unwrap())
        .json(&json!({
            "subscriber_uuid": "00000000-0000-0000-0000-000000000000",
            "post_slug": "test",
            "subject": "Test",
            "html_content": "<p>Hello</p>"
        }))
        .await;
    response.assert_status(axum::http::StatusCode::NOT_FOUND);
}
