use axum::{
    Json, Router,
    extract::{Path, State},
    response::Redirect,
    routing::get,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::email_send::EmailSend;
use crate::models::subscriber::Subscriber;
use crate::state::AppState;

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/unsubscribe/{token}",
            get(unsubscribe_by_token).post(one_click_unsubscribe),
        )
        .route(
            "/api/v1/unsubscribe/{token}/preferences",
            get(get_preferences).patch(update_preferences),
        )
        // Account deletion requires JWT auth — use DELETE /api/v1/subscribers/{uuid} instead
}

pub fn authenticated_routes() -> Router<AppState> {
    Router::new()
}

// --- Legacy redirect (for old emails) ---

#[utoipa::path(
    get,
    path = "/api/v1/unsubscribe/{token}",
    params(("token" = Uuid, Path, description = "Unsubscribe token from email")),
    responses(
        (status = 302, description = "Redirects to unsubscribe page"),
    )
)]
pub async fn unsubscribe_by_token(
    State(state): State<AppState>,
    Path(token): Path<Uuid>,
) -> Redirect {
    Redirect::temporary(&format!(
        "{}/unsubscribe?token={}",
        state.config.site_url, token
    ))
}

// --- Get preferences ---

#[derive(Debug, Serialize, ToSchema)]
pub struct PreferencesResponse {
    pub email: String,
    pub newsletter: Option<String>,
    pub subscribed_postcard: bool,
    pub subscribed_contraption: bool,
    pub subscribed_workshop: bool,
}

fn mask_email(email: &str) -> String {
    match email.split_once('@') {
        Some((local, domain)) => {
            if local.len() <= 1 {
                format!("*@{}", domain)
            } else {
                let first = &local[..1];
                let masked = "*".repeat(local.len() - 1);
                format!("{}{}@{}", first, masked, domain)
            }
        }
        None => "***".to_string(),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/unsubscribe/{token}/preferences",
    params(("token" = Uuid, Path, description = "Unsubscribe token from email")),
    responses(
        (status = 200, description = "Subscriber preferences", body = PreferencesResponse),
        (status = 404, description = "Invalid token"),
    )
)]
pub async fn get_preferences(
    State(state): State<AppState>,
    Path(token): Path<Uuid>,
) -> Result<Json<PreferencesResponse>, AppError> {
    let email_send = EmailSend::find_by_unsubscribe_token(&state.pool, token)
        .await?
        .ok_or(AppError::NotFound)?;

    let subscriber = Subscriber::find_by_id(&state.pool, email_send.subscriber_id)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(PreferencesResponse {
        email: mask_email(&subscriber.email),
        newsletter: email_send.newsletter,
        subscribed_postcard: subscriber.subscribed_postcard,
        subscribed_contraption: subscriber.subscribed_contraption,
        subscribed_workshop: subscriber.subscribed_workshop,
    }))
}

// --- Update preferences ---

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePreferencesRequest {
    pub subscribed_postcard: Option<bool>,
    pub subscribed_contraption: Option<bool>,
    pub subscribed_workshop: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SuccessResponse {
    pub success: bool,
}

#[utoipa::path(
    patch,
    path = "/api/v1/unsubscribe/{token}/preferences",
    params(("token" = Uuid, Path, description = "Unsubscribe token from email")),
    request_body = UpdatePreferencesRequest,
    responses(
        (status = 200, description = "Preferences updated", body = SuccessResponse),
        (status = 404, description = "Invalid token"),
    )
)]
pub async fn update_preferences(
    State(state): State<AppState>,
    Path(token): Path<Uuid>,
    Json(req): Json<UpdatePreferencesRequest>,
) -> Result<Json<SuccessResponse>, AppError> {
    let email_send = EmailSend::find_by_unsubscribe_token(&state.pool, token)
        .await?
        .ok_or(AppError::NotFound)?;

    let subscriber = Subscriber::find_by_id(&state.pool, email_send.subscriber_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let update = crate::models::subscriber::UpdateSubscriberRequest {
        name: None,
        subscribed_postcard: req.subscribed_postcard,
        subscribed_contraption: req.subscribed_contraption,
        subscribed_workshop: req.subscribed_workshop,
    };
    Subscriber::update(&state.pool, subscriber.uuid, &update).await?;
    tracing::info!(email = %subscriber.email, "Subscriber preferences updated via unsubscribe token");

    Ok(Json(SuccessResponse { success: true }))
}

// --- RFC 8058 one-click unsubscribe POST ---

/// Maps a newsletter name to an UpdateSubscriberRequest that unsubscribes from that newsletter.
fn unsubscribe_request_for_newsletter(
    newsletter: Option<&str>,
) -> crate::models::subscriber::UpdateSubscriberRequest {
    let (postcard, contraption, workshop) = match newsletter {
        Some("postcard") => (Some(false), None, None),
        Some("contraption") => (None, Some(false), None),
        Some("workshop") => (None, None, Some(false)),
        _ => (Some(false), Some(false), Some(false)),
    };
    crate::models::subscriber::UpdateSubscriberRequest {
        name: None,
        subscribed_postcard: postcard,
        subscribed_contraption: contraption,
        subscribed_workshop: workshop,
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/unsubscribe/{token}",
    params(("token" = Uuid, Path, description = "Unsubscribe token from email")),
    responses(
        (status = 200, description = "Unsubscribed via one-click", body = SuccessResponse),
        (status = 404, description = "Invalid token"),
    )
)]
pub async fn one_click_unsubscribe(
    State(state): State<AppState>,
    Path(token): Path<Uuid>,
) -> Result<Json<SuccessResponse>, AppError> {
    let email_send = EmailSend::find_by_unsubscribe_token(&state.pool, token)
        .await?
        .ok_or(AppError::NotFound)?;

    let subscriber = Subscriber::find_by_id(&state.pool, email_send.subscriber_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let update = unsubscribe_request_for_newsletter(email_send.newsletter.as_deref());
    Subscriber::update(&state.pool, subscriber.uuid, &update).await?;
    EmailSend::mark_unsubscribed(&state.pool, email_send.id).await?;
    tracing::info!(
        email = %subscriber.email,
        newsletter = ?email_send.newsletter,
        "One-click unsubscribe"
    );

    Ok(Json(SuccessResponse { success: true }))
}

