use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};

use crate::state::AppState;

pub async fn require_api_key(
    axum::extract::State(state): axum::extract::State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let api_key = request
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok());

    match api_key {
        Some(key) if constant_time_eq(key, &state.config.m2m_api_key) => {
            Ok(next.run(request).await)
        }
        _ => {
            tracing::warn!(
                method = %request.method(),
                uri = %request.uri(),
                "Unauthorized API request (missing or invalid x-api-key)"
            );
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}
