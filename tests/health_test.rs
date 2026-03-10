mod common;

#[tokio::test]
async fn health_check_returns_ok() {
    let app = common::TestApp::new().await;
    let response = app.server.get("/health").await;
    response.assert_status_ok();
    response.assert_json_contains(&serde_json::json!({ "status": "ok" }));
}
