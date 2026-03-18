use axum::{Json, Router, response::Html, routing::get};
use serde_json::{Value, json};

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
}

async fn root() -> Html<&'static str> {
    Html(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>Printing Press</title>
<style>
  @font-face { font-family: 'Sohne'; src: url('https://fonts.philipithomas.com/klim/soehne-buch.woff2') format('woff2'); font-weight: 400; font-display: swap; }
  @font-face { font-family: 'Sohne'; src: url('https://fonts.philipithomas.com/klim/soehne-halbfett.woff2') format('woff2'); font-weight: 600; font-display: swap; }
  body { margin: 0; min-height: 100vh; display: flex; align-items: center; justify-content: center; background: #F5F3F0; font-family: 'Sohne', -apple-system, BlinkMacSystemFont, sans-serif; color: #111110; }
</style>
</head>
<body>
<p style="font-size: 15px; color: #7E7A73;">Hello, you've reached the printing press.</p>
</body>
</html>"#,
    )
}

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy")
    )
)]
pub async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
