mod common;

use printing_press::models::email_send::EmailSend;
use printing_press::models::subscriber::Subscriber;

#[tokio::test]
async fn unsubscribe_by_token_valid() {
    let app = common::TestApp::new().await;

    let sub = Subscriber::create(&app.pool, "tokenunsub@example.com", None, None)
        .await
        .unwrap();

    let email_send = EmailSend::create(&app.pool, sub.id, "test-post")
        .await
        .unwrap();

    let response = app
        .server
        .get(&format!(
            "/api/v1/unsubscribe/{}",
            email_send.unsubscribe_token
        ))
        .await;
    response.assert_status(axum::http::StatusCode::TEMPORARY_REDIRECT);
}

#[tokio::test]
async fn unsubscribe_by_invalid_token() {
    let app = common::TestApp::new().await;

    let response = app
        .server
        .get("/api/v1/unsubscribe/00000000-0000-0000-0000-000000000000")
        .await;
    response.assert_status(axum::http::StatusCode::TEMPORARY_REDIRECT);
}
