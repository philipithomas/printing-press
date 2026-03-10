use crate::error::AppError;
use crate::models::subscriber::{CreateSubscriberRequest, Subscriber};
use crate::services::login_service;
use crate::state::AppState;

pub struct CreateResult {
    pub subscriber: Subscriber,
    pub is_new: bool,
}

pub async fn create_or_retrieve(
    state: &AppState,
    req: &CreateSubscriberRequest,
) -> Result<CreateResult, AppError> {
    let email = req.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(AppError::BadRequest("Invalid email address".to_string()));
    }

    // Check if subscriber exists
    if let Some(existing) = Subscriber::find_by_email(&state.pool, &email).await? {
        // If Google verified and not yet confirmed, confirm now
        if req.google_verified && existing.confirmed_at.is_none() {
            let confirmed = Subscriber::confirm(&state.pool, existing.id).await?;
            return Ok(CreateResult {
                subscriber: confirmed,
                is_new: false,
            });
        }
        return Ok(CreateResult {
            subscriber: existing,
            is_new: false,
        });
    }

    // Create new subscriber
    let subscriber = Subscriber::create(
        &state.pool,
        &email,
        req.name.as_deref(),
        req.source.as_deref(),
    )
    .await?;

    // If Google verified, confirm immediately
    if req.google_verified {
        let confirmed = Subscriber::confirm(&state.pool, subscriber.id).await?;
        return Ok(CreateResult {
            subscriber: confirmed,
            is_new: true,
        });
    }

    // Generate login tokens and send confirmation email (best-effort)
    if let Err(e) = login_service::create_and_send_login(state, &subscriber).await {
        tracing::error!("Failed to send confirmation email to {}: {}", email, e);
    }

    Ok(CreateResult {
        subscriber,
        is_new: true,
    })
}
