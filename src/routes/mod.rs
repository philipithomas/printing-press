use axum::{Router, middleware};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::state::AppState;

mod emails;
mod health;
mod publish;
mod subscribers;
mod unsubscribe;
mod verify;

#[derive(OpenApi)]
#[openapi(
    paths(
        health::health,
        subscribers::create_subscriber,
        subscribers::get_subscriber,
        subscribers::update_subscriber,
        subscribers::unsubscribe_subscriber,
        verify::verify,
        unsubscribe::unsubscribe_by_token,
        unsubscribe::get_preferences,
        unsubscribe::update_preferences,
        unsubscribe::delete_account,
        emails::send_email,
        publish::validate,
        publish::send,
        publish::send_one,
    ),
    components(schemas(
        crate::models::subscriber::Subscriber,
        crate::models::subscriber::CreateSubscriberRequest,
        crate::models::subscriber::UpdateSubscriberRequest,
        crate::models::email_send::EmailSend,
        crate::models::login::Login,
        verify::VerifyRequest,
        emails::SendEmailRequest,
        publish::ValidateRequest,
        publish::ValidateResponse,
        publish::SendRequest,
        publish::SendResponse,
        publish::SendOneRequest,
        publish::SendOneResponse,
        unsubscribe::PreferencesResponse,
        unsubscribe::UpdatePreferencesRequest,
        unsubscribe::SuccessResponse,
    ))
)]
struct ApiDoc;

pub fn router(state: AppState) -> Router {
    let api_routes = Router::new()
        .merge(subscribers::routes())
        .merge(verify::routes())
        .merge(unsubscribe::authenticated_routes())
        .merge(emails::routes())
        .merge(publish::routes())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::auth::require_api_key,
        ));

    Router::new()
        .merge(health::routes())
        .merge(unsubscribe::public_routes())
        .nest("/api/v1", api_routes)
        .merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
        .with_state(state)
}
