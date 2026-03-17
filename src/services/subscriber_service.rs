use crate::error::AppError;
use crate::models::subscriber::{CreateSubscriberRequest, Subscriber};
use crate::services::login_service;
use crate::state::AppState;

pub struct CreateResult {
    pub subscriber: Subscriber,
    pub is_new: bool,
}

/// Validate email has the form `local@domain.tld` with non-empty parts and at least one dot in domain.
fn is_valid_email(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    if local.is_empty() || domain.is_empty() {
        return false;
    }
    // Domain must have at least one dot (i.e., a TLD)
    let Some((host, tld)) = domain.rsplit_once('.') else {
        return false;
    };
    if host.is_empty() || tld.len() < 2 {
        return false;
    }
    // No spaces allowed
    !email.contains(' ')
}

pub async fn create_or_retrieve(
    state: &AppState,
    req: &CreateSubscriberRequest,
) -> Result<CreateResult, AppError> {
    let email = req.email.trim().to_lowercase();
    if !is_valid_email(&email) {
        tracing::warn!(email = %req.email, "Invalid email address rejected");
        return Err(AppError::BadRequest("Invalid email address".to_string()));
    }

    // Check if subscriber exists
    if let Some(existing) = Subscriber::find_by_email(&state.pool, &email).await? {
        // If Google verified and not yet confirmed, confirm now
        if req.google_verified && existing.confirmed_at.is_none() {
            let confirmed = Subscriber::confirm(&state.pool, existing.id).await?;
            tracing::info!(email = %confirmed.email, "Subscriber confirmed via Google sign-in (existing)");
            if let Err(e) = state
                .email_service
                .send_new_subscriber_notification(
                    &confirmed.email,
                    confirmed.name.as_deref(),
                    confirmed.source.as_deref(),
                    &state.config.site_url,
                )
                .await
            {
                tracing::error!(
                    "Failed to send new subscriber notification for {}: {}",
                    confirmed.email,
                    e
                );
            }
            return Ok(CreateResult {
                subscriber: confirmed,
                is_new: false,
            });
        }

        // If not yet confirmed, resend confirmation email
        if existing.confirmed_at.is_none() {
            tracing::info!(email = %email, "Existing unconfirmed subscriber, resending confirmation");
            if let Err(e) = login_service::create_and_send_login(state, &existing).await {
                tracing::error!("Failed to resend confirmation email to {}: {}", email, e);
            }
        } else if !req.google_verified {
            tracing::info!(email = %email, "Existing confirmed subscriber, sending sign-in code");
            if let Err(e) = login_service::create_and_send_login(state, &existing).await {
                tracing::error!("Failed to send sign-in email to {}: {}", email, e);
            }
        } else {
            tracing::info!(email = %email, "Existing confirmed subscriber returned (Google verified)");
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
    tracing::info!(email = %email, source = ?req.source, "New subscriber created");

    // If Google verified, confirm immediately
    if req.google_verified {
        let confirmed = Subscriber::confirm(&state.pool, subscriber.id).await?;
        tracing::info!(email = %confirmed.email, "Subscriber confirmed via Google sign-in (new)");
        if let Err(e) = state
            .email_service
            .send_new_subscriber_notification(
                &confirmed.email,
                confirmed.name.as_deref(),
                confirmed.source.as_deref(),
                &state.config.site_url,
            )
            .await
        {
            tracing::error!(
                "Failed to send new subscriber notification for {}: {}",
                confirmed.email,
                e
            );
        }
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
