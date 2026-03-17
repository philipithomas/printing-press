use chrono::{Duration, Utc};
use rand::Rng;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::login::Login;
use crate::models::subscriber::Subscriber;
use crate::state::AppState;

pub async fn create_and_send_login(
    state: &AppState,
    subscriber: &Subscriber,
) -> Result<(Login, Login), AppError> {
    let expiry = Utc::now() + Duration::minutes(15);

    // Generate 6-digit code
    let code: String = format!("{:06}", rand::rng().random_range(0..1_000_000));
    let code_login = Login::create(&state.pool, subscriber.id, &code, "code", expiry).await?;

    // Generate magic link token
    let magic_token = Uuid::new_v4().to_string();
    let magic_login = Login::create(
        &state.pool,
        subscriber.id,
        &magic_token,
        "magic_link",
        expiry,
    )
    .await?;

    // Send confirmation email
    let magic_link = format!(
        "{}/auth/verify?token={}",
        state.config.site_url, magic_token
    );

    state
        .email_service
        .send_confirmation(
            &subscriber.email,
            &code,
            &magic_link,
            &state.config.site_url,
        )
        .await
        .map_err(|e| AppError::Internal(format!("Failed to send email: {}", e)))?;

    tracing::info!(email = %subscriber.email, "Confirmation email sent");

    // Mark emails as sent
    let code_login = Login::mark_email_sent(&state.pool, code_login.id).await?;
    let magic_login = Login::mark_email_sent(&state.pool, magic_login.id).await?;

    Ok((code_login, magic_login))
}

pub async fn verify_token(
    state: &AppState,
    token: &str,
    email: Option<&str>,
) -> Result<Subscriber, AppError> {
    // Determine token type: 6-digit codes are "code", everything else is "magic_link"
    let token_type = if token.len() == 6 && token.chars().all(|c| c.is_ascii_digit()) {
        "code"
    } else {
        "magic_link"
    };

    let login = Login::find_valid_by_token(&state.pool, token, token_type).await?;

    let login = match login {
        Some(l) => l,
        None => {
            // On failed code verification, increment attempt counter for this subscriber
            if token_type == "code"
                && let Some(email) = email
                && let Ok(Some(subscriber)) =
                    Subscriber::find_by_email(&state.pool, email).await
            {
                let _ =
                    Login::increment_attempts_for_subscriber(&state.pool, subscriber.id).await;
            }
            return Err(AppError::BadRequest(
                "Invalid or expired token".to_string(),
            ));
        }
    };

    // Check if already confirmed (to avoid duplicate notifications on re-verify)
    let existing = Subscriber::find_by_id(&state.pool, login.subscriber_id).await?;
    let was_already_confirmed = existing
        .as_ref()
        .is_some_and(|s| s.confirmed_at.is_some());

    Login::mark_verified(&state.pool, login.id).await?;
    let subscriber = Subscriber::confirm(&state.pool, login.subscriber_id).await?;

    if was_already_confirmed {
        tracing::info!(email = %subscriber.email, "Returning subscriber signed in");
    } else {
        tracing::info!(email = %subscriber.email, "Subscriber confirmed via email verification");
    }

    if !was_already_confirmed
        && let Err(e) = state
            .email_service
            .send_new_subscriber_notification(
                &subscriber.email,
                subscriber.name.as_deref(),
                subscriber.source.as_deref(),
                &state.config.site_url,
            )
            .await
    {
        tracing::error!(
            "Failed to send new subscriber notification for {}: {}",
            subscriber.email,
            e
        );
    }

    Ok(subscriber)
}
