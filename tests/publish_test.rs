mod common;

use printing_press::models::subscriber::Subscriber;
use serde_json::json;

#[tokio::test]
async fn validate_requires_auth() {
    let app = common::TestApp::new().await;

    let response = app
        .server
        .post("/api/v1/publish/validate")
        .json(&json!({
            "post_slug": "test-post",
            "newsletter": "contraption"
        }))
        .await;
    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn validate_rejects_invalid_newsletter() {
    let app = common::TestApp::new().await;

    let response = app
        .server
        .post("/api/v1/publish/validate")
        .add_header("x-api-key", &app.api_key)
        .json(&json!({
            "post_slug": "test-post",
            "newsletter": "invalid"
        }))
        .await;
    response.assert_status(axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn validate_returns_counts() {
    let app = common::TestApp::new().await;

    // Create confirmed subscriber subscribed to contraption
    let sub = Subscriber::create(&app.pool, "pub@example.com", None, None)
        .await
        .unwrap();
    Subscriber::confirm(&app.pool, sub.id).await.unwrap();

    // Create unconfirmed subscriber
    let _unsub = Subscriber::create(&app.pool, "unconfirmed@example.com", None, None)
        .await
        .unwrap();

    let response = app
        .server
        .post("/api/v1/publish/validate")
        .add_header("x-api-key", &app.api_key)
        .json(&json!({
            "post_slug": "test-post",
            "newsletter": "contraption"
        }))
        .await;
    response.assert_status(axum::http::StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["eligible_subscribers"], 1);
    assert_eq!(body["already_sent"], 0);
}

#[tokio::test]
async fn send_enqueues_emails() {
    let app = common::TestApp::new().await;

    // Create 3 confirmed subscribers
    for email in &["a@example.com", "b@example.com", "c@example.com"] {
        let sub = Subscriber::create(&app.pool, email, None, None)
            .await
            .unwrap();
        Subscriber::confirm(&app.pool, sub.id).await.unwrap();
    }

    let response = app
        .server
        .post("/api/v1/publish/send")
        .add_header("x-api-key", &app.api_key)
        .json(&json!({
            "post_slug": "my-post",
            "newsletter": "contraption",
            "subject": "Test Subject",
            "html_content": "<p>Hello</p>"
        }))
        .await;
    response.assert_status(axum::http::StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["enqueued"], 3);
    assert_eq!(body["already_sent"], 0);
}

#[tokio::test]
async fn send_rejects_without_force() {
    let app = common::TestApp::new().await;

    // Create subscriber and simulate a previous send
    let sub = Subscriber::create(&app.pool, "force@example.com", None, None)
        .await
        .unwrap();
    Subscriber::confirm(&app.pool, sub.id).await.unwrap();
    printing_press::models::email_send::EmailSend::create(&app.pool, sub.id, "my-post")
        .await
        .unwrap();

    let response = app
        .server
        .post("/api/v1/publish/send")
        .add_header("x-api-key", &app.api_key)
        .json(&json!({
            "post_slug": "my-post",
            "newsletter": "contraption",
            "subject": "Test",
            "html_content": "<p>Hello</p>"
        }))
        .await;
    response.assert_status(axum::http::StatusCode::CONFLICT);
}

#[tokio::test]
async fn send_with_force_succeeds() {
    let app = common::TestApp::new().await;

    let sub = Subscriber::create(&app.pool, "force2@example.com", None, None)
        .await
        .unwrap();
    Subscriber::confirm(&app.pool, sub.id).await.unwrap();
    printing_press::models::email_send::EmailSend::create(&app.pool, sub.id, "my-post-2")
        .await
        .unwrap();

    // Create another subscriber who hasn't received it yet
    let sub2 = Subscriber::create(&app.pool, "force3@example.com", None, None)
        .await
        .unwrap();
    Subscriber::confirm(&app.pool, sub2.id).await.unwrap();

    let response = app
        .server
        .post("/api/v1/publish/send")
        .add_header("x-api-key", &app.api_key)
        .json(&json!({
            "post_slug": "my-post-2",
            "newsletter": "contraption",
            "subject": "Test",
            "html_content": "<p>Hello</p>",
            "force": true
        }))
        .await;
    response.assert_status(axum::http::StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["enqueued"], 1); // Only the one who hasn't received it
}

#[tokio::test]
async fn send_one_requires_auth() {
    let app = common::TestApp::new().await;

    let response = app
        .server
        .post("/api/v1/publish/send-one")
        .json(&json!({
            "email": "test@example.com",
            "post_slug": "test-post",
            "subject": "Test",
            "html_content": "<p>Hello</p>"
        }))
        .await;
    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unsubscribe_preferences_returns_data() {
    let app = common::TestApp::new().await;

    let sub = Subscriber::create(&app.pool, "prefs@example.com", None, None)
        .await
        .unwrap();

    let email_send =
        printing_press::models::email_send::EmailSend::create(&app.pool, sub.id, "pref-post")
            .await
            .unwrap();

    let response = app
        .server
        .get(&format!(
            "/api/v1/unsubscribe/{}/preferences",
            email_send.unsubscribe_token
        ))
        .await;
    response.assert_status(axum::http::StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["subscribed_postcard"], true);
    assert_eq!(body["subscribed_contraption"], true);
    assert_eq!(body["subscribed_workshop"], true);
    // Email should be masked
    assert!(body["email"].as_str().unwrap().contains("*"));
}

#[tokio::test]
async fn unsubscribe_update_preferences() {
    let app = common::TestApp::new().await;

    let sub = Subscriber::create(&app.pool, "update@example.com", None, None)
        .await
        .unwrap();

    let email_send =
        printing_press::models::email_send::EmailSend::create(&app.pool, sub.id, "upd-post")
            .await
            .unwrap();

    let response = app
        .server
        .patch(&format!(
            "/api/v1/unsubscribe/{}/preferences",
            email_send.unsubscribe_token
        ))
        .json(&json!({
            "subscribed_contraption": false
        }))
        .await;
    response.assert_status(axum::http::StatusCode::OK);

    // Verify it was updated
    let updated = Subscriber::find_by_id(&app.pool, sub.id)
        .await
        .unwrap()
        .unwrap();
    assert!(!updated.subscribed_contraption);
    assert!(updated.subscribed_postcard);
    assert!(updated.subscribed_workshop);
}

#[tokio::test]
async fn unsubscribe_delete_account() {
    let app = common::TestApp::new().await;

    let sub = Subscriber::create(&app.pool, "delete@example.com", None, None)
        .await
        .unwrap();

    let email_send =
        printing_press::models::email_send::EmailSend::create(&app.pool, sub.id, "del-post")
            .await
            .unwrap();

    let response = app
        .server
        .delete(&format!(
            "/api/v1/unsubscribe/{}/account",
            email_send.unsubscribe_token
        ))
        .await;
    response.assert_status(axum::http::StatusCode::OK);

    // Verify subscriber is deleted
    let found = Subscriber::find_by_id(&app.pool, sub.id).await.unwrap();
    assert!(found.is_none());
}
