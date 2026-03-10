use axum::{
    Router,
    extract::{Path, State},
    response::Redirect,
    routing::get,
};
use uuid::Uuid;

use crate::models::email_send::EmailSend;
use crate::models::subscriber::Subscriber;
use crate::state::AppState;

pub fn public_routes() -> Router<AppState> {
    Router::new().route("/api/v1/unsubscribe/{token}", get(unsubscribe_by_token))
}

pub fn authenticated_routes() -> Router<AppState> {
    Router::new()
}

#[utoipa::path(
    get,
    path = "/api/v1/unsubscribe/{token}",
    params(("token" = Uuid, Path, description = "Unsubscribe token from email")),
    responses(
        (status = 302, description = "Redirects to site after unsubscribing"),
        (status = 404, description = "Invalid token"),
    )
)]
pub async fn unsubscribe_by_token(
    State(state): State<AppState>,
    Path(token): Path<Uuid>,
) -> Result<Redirect, Redirect> {
    let email_send = EmailSend::find_by_unsubscribe_token(&state.pool, token)
        .await
        .ok()
        .flatten();

    match email_send {
        Some(send) => {
            // Mark the email send as triggering unsubscribe
            let _ = EmailSend::mark_unsubscribed(&state.pool, send.id).await;
            // Unsubscribe the user from all newsletters
            let _ = Subscriber::unsubscribe_all(&state.pool, send.subscriber_id).await;
            Ok(Redirect::temporary(&format!(
                "{}/?unsubscribed=true",
                state.config.site_url
            )))
        }
        None => Err(Redirect::temporary(&format!(
            "{}/?error=invalid-token",
            state.config.site_url
        ))),
    }
}
