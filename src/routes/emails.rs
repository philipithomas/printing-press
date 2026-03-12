use axum::{Json, Router, extract::State, routing::post};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::models::email_send::EmailSend;
use crate::models::subscriber::Subscriber;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/emails/send", post(send_email))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SendEmailRequest {
    pub subscriber_uuid: uuid::Uuid,
    pub post_slug: String,
    pub subject: String,
    pub html_content: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/emails/send",
    request_body = SendEmailRequest,
    responses(
        (status = 200, description = "Email sent", body = EmailSend),
        (status = 404, description = "Subscriber not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn send_email(
    State(state): State<AppState>,
    Json(req): Json<SendEmailRequest>,
) -> Result<Json<EmailSend>, AppError> {
    let subscriber = Subscriber::find_by_uuid(&state.pool, req.subscriber_uuid)
        .await?
        .ok_or(AppError::NotFound)?;

    // Create email_send record
    let email_send = EmailSend::create(&state.pool, subscriber.id, &req.post_slug).await?;

    // Build unsubscribe URL
    let unsubscribe_url = format!(
        "{}/unsubscribe?token={}",
        state.config.site_url, email_send.unsubscribe_token
    );

    // Wrap content in newsletter template
    let html = crate::templates::render_newsletter(
        &req.html_content,
        &unsubscribe_url,
        &state.config.site_url,
    )
    .map_err(|e| AppError::Internal(format!("Template error: {}", e)))?;

    // Send via email service
    match state
        .email_service
        .send_newsletter(&subscriber.email, &req.subject, &html, &unsubscribe_url)
        .await
    {
        Ok(()) => Ok(Json(email_send)),
        Err(e) => {
            let error_msg = e.to_string();
            let _updated = EmailSend::record_error(&state.pool, email_send.id, &error_msg).await?;
            Err(AppError::Internal(format!(
                "Failed to send email: {}",
                error_msg
            )))
        }
    }
}
