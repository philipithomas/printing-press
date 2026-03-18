use axum::{Router, http::HeaderValue, middleware};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::state::AppState;

mod emails;
mod health;
mod import;
mod publish;
mod stats;
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
        subscribers::delete_subscriber,
        verify::verify,
        unsubscribe::unsubscribe_by_token,
        unsubscribe::one_click_unsubscribe,
        unsubscribe::get_preferences,
        unsubscribe::update_preferences,
        emails::send_email,
        publish::validate,
        publish::send,
        publish::send_one,
        import::import_subscribers,
        stats::subscriber_count,
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
        subscribers::DeleteResponse,
        unsubscribe::SuccessResponse,
        stats::SubscriberCountResponse,
        import::ImportRequest,
        crate::models::subscriber::ImportSubscriberEntry,
        crate::models::subscriber::ImportResult,
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
        .merge(import::routes())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::auth::require_api_key,
        ));

    let mut app = Router::new()
        .merge(health::routes())
        .merge(unsubscribe::public_routes())
        .merge(stats::public_routes())
        .nest("/api/v1", api_routes);

    // Only expose Swagger UI in development (not on production public_url)
    if state.config.public_url.contains("localhost") {
        app = app.merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()));
    }

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list([
            HeaderValue::from_static("https://philipithomas.com"),
            HeaderValue::from_static("https://www.philipithomas.com"),
            HeaderValue::from_static("http://localhost:3000"),
        ]))
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PATCH,
            axum::http::Method::DELETE,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::HeaderName::from_static("x-api-key"),
        ]);

    app.layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
