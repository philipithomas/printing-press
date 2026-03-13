use axum::{Json, Router, extract::State, routing::post};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::AppError;
use crate::models::email_send::EmailSend;
use crate::models::subscriber::Subscriber;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/publish/validate", post(validate))
        .route("/publish/send", post(send))
        .route("/publish/send-one", post(send_one))
}

const VALID_NEWSLETTERS: [&str; 3] = ["postcard", "contraption", "workshop"];

fn validate_newsletter(newsletter: &str) -> Result<(), AppError> {
    if VALID_NEWSLETTERS.contains(&newsletter) {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!(
            "Invalid newsletter '{}'. Must be one of: postcard, contraption, workshop",
            newsletter
        )))
    }
}

// --- Validate ---

#[derive(Debug, Deserialize, ToSchema)]
pub struct ValidateRequest {
    pub post_slug: String,
    pub newsletter: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ValidateResponse {
    pub post_slug: String,
    pub newsletter: String,
    pub eligible_subscribers: i64,
    pub already_sent: i64,
}

#[utoipa::path(
    post,
    path = "/api/v1/publish/validate",
    request_body = ValidateRequest,
    responses(
        (status = 200, description = "Validation result", body = ValidateResponse),
        (status = 400, description = "Invalid newsletter"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn validate(
    State(state): State<AppState>,
    Json(req): Json<ValidateRequest>,
) -> Result<Json<ValidateResponse>, AppError> {
    validate_newsletter(&req.newsletter)?;

    let eligible = Subscriber::count_eligible(&state.pool, &req.newsletter, &req.post_slug).await?;
    let already_sent = EmailSend::count_by_slug(&state.pool, &req.post_slug).await?;

    Ok(Json(ValidateResponse {
        post_slug: req.post_slug,
        newsletter: req.newsletter,
        eligible_subscribers: eligible,
        already_sent,
    }))
}

// --- Send (bulk enqueue) ---

#[derive(Debug, Deserialize, ToSchema)]
pub struct SendRequest {
    pub post_slug: String,
    pub newsletter: String,
    pub subject: String,
    pub html_content: String,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SendResponse {
    pub post_slug: String,
    pub newsletter: String,
    pub enqueued: i64,
    pub already_sent: i64,
}

#[utoipa::path(
    post,
    path = "/api/v1/publish/send",
    request_body = SendRequest,
    responses(
        (status = 200, description = "Emails enqueued", body = SendResponse),
        (status = 400, description = "Invalid newsletter"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Already sent, use force flag"),
    )
)]
pub async fn send(
    State(state): State<AppState>,
    Json(req): Json<SendRequest>,
) -> Result<Json<SendResponse>, AppError> {
    validate_newsletter(&req.newsletter)?;

    let already_sent = EmailSend::count_by_slug(&state.pool, &req.post_slug).await?;

    if already_sent > 0 && !req.force {
        return Err(AppError::Conflict(format!(
            "{} subscribers have already received this post. Use force=true to send to remaining subscribers.",
            already_sent
        )));
    }

    let subscriber_ids =
        Subscriber::find_eligible_ids(&state.pool, &req.newsletter, &req.post_slug).await?;

    if subscriber_ids.is_empty() {
        return Ok(Json(SendResponse {
            post_slug: req.post_slug,
            newsletter: req.newsletter,
            enqueued: 0,
            already_sent,
        }));
    }

    let enqueued = EmailSend::bulk_create_queued(
        &state.pool,
        &subscriber_ids,
        &req.post_slug,
        &req.newsletter,
        &req.subject,
        &req.html_content,
    )
    .await?;

    tracing::info!(
        "Enqueued {} emails for post '{}' (newsletter: {})",
        enqueued,
        req.post_slug,
        req.newsletter
    );

    Ok(Json(SendResponse {
        post_slug: req.post_slug,
        newsletter: req.newsletter,
        enqueued,
        already_sent,
    }))
}

// --- Send One (immediate test send) ---

#[derive(Debug, Deserialize, ToSchema)]
pub struct SendOneRequest {
    pub email: String,
    pub post_slug: String,
    pub newsletter: Option<String>,
    pub subject: String,
    pub html_content: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SendOneResponse {
    pub email: String,
    pub post_slug: String,
    pub status: String,
    pub error: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/v1/publish/send-one",
    request_body = SendOneRequest,
    responses(
        (status = 200, description = "Send result", body = SendOneResponse),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn send_one(
    State(state): State<AppState>,
    Json(req): Json<SendOneRequest>,
) -> Result<Json<SendOneResponse>, AppError> {
    let subscriber = Subscriber::find_by_email(&state.pool, &req.email).await?;

    // Build unsubscribe URL — only if subscriber exists
    let (unsubscribe_url, email_send) = if let Some(ref sub) = subscriber {
        let es = EmailSend::create(&state.pool, sub.id, &req.post_slug).await?;
        let url = format!(
            "{}/unsubscribe?token={}",
            state.config.site_url, es.unsubscribe_token
        );
        (url, Some(es))
    } else {
        let url = format!("{}/unsubscribe", state.config.site_url);
        (url, None)
    };

    let html = crate::templates::render_newsletter(
        &req.html_content,
        &unsubscribe_url,
        &state.config.site_url,
        req.newsletter.as_deref(),
    )
    .map_err(|e| AppError::Internal(format!("Template error: {}", e)))?;

    match state
        .email_service
        .send_newsletter(&req.email, &req.subject, &html, &unsubscribe_url)
        .await
    {
        Ok(()) => {
            if let Some(es) = &email_send {
                // Mark as sent immediately
                sqlx::query("UPDATE email_sends SET sent_at = NOW() WHERE id = $1")
                    .bind(es.id)
                    .execute(&state.pool)
                    .await?;
            }
            Ok(Json(SendOneResponse {
                email: req.email,
                post_slug: req.post_slug,
                status: "sent".to_string(),
                error: None,
            }))
        }
        Err(e) => {
            let error_msg = e.to_string();
            if let Some(es) = &email_send {
                let _ = EmailSend::record_error(&state.pool, es.id, &error_msg).await;
            }
            Ok(Json(SendOneResponse {
                email: req.email,
                post_slug: req.post_slug,
                status: "error".to_string(),
                error: Some(error_msg),
            }))
        }
    }
}
