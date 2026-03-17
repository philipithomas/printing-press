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
    // Pad both to the same length to avoid leaking length information via early return.
    let max_len = a.len().max(b.len()).max(1);
    let a_bytes: Vec<u8> = a.bytes().chain(std::iter::repeat(0u8)).take(max_len).collect();
    let b_bytes: Vec<u8> = b.bytes().chain(std::iter::repeat(0u8)).take(max_len).collect();
    let mut acc = (a.len() ^ b.len()) as u8;
    for (x, y) in a_bytes.iter().zip(b_bytes.iter()) {
        acc |= x ^ y;
    }
    acc == 0
}
