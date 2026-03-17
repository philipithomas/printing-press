use axum::{Json, Router, extract::State, routing::post};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::models::subscriber::Subscriber;
use crate::services::login_service;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/subscribers/verify", post(verify))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyRequest {
    pub token: String,
    #[serde(default)]
    pub email: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/v1/subscribers/verify",
    request_body = VerifyRequest,
    responses(
        (status = 200, description = "Token verified", body = Subscriber),
        (status = 400, description = "Invalid or expired token"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn verify(
    State(state): State<AppState>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<Subscriber>, AppError> {
    let subscriber =
        login_service::verify_token(&state, &req.token, req.email.as_deref()).await?;
    Ok(Json(subscriber))
}
