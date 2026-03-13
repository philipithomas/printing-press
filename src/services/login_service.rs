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

    // Mark emails as sent
    let code_login = Login::mark_email_sent(&state.pool, code_login.id).await?;
    let magic_login = Login::mark_email_sent(&state.pool, magic_login.id).await?;

    Ok((code_login, magic_login))
}

pub async fn verify_token(state: &AppState, token: &str) -> Result<Subscriber, AppError> {
    let login = Login::find_valid_by_token(&state.pool, token)
        .await?
        .ok_or(AppError::BadRequest("Invalid or expired token".to_string()))?;

    Login::mark_verified(&state.pool, login.id).await?;
    let subscriber = Subscriber::confirm(&state.pool, login.subscriber_id).await?;

    Ok(subscriber)
}
