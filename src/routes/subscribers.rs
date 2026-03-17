use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, patch, post},
};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::subscriber::{CreateSubscriberRequest, Subscriber, UpdateSubscriberRequest};
use crate::services::subscriber_service;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/subscribers", post(create_subscriber))
        .route("/subscribers/{uuid}", get(get_subscriber))
        .route("/subscribers/{uuid}", patch(update_subscriber))
        .route(
            "/subscribers/{uuid}/unsubscribe",
            post(unsubscribe_subscriber),
        )
}

#[utoipa::path(
    post,
    path = "/api/v1/subscribers",
    request_body = CreateSubscriberRequest,
    responses(
        (status = 200, description = "Subscriber created or retrieved", body = Subscriber),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn create_subscriber(
    State(state): State<AppState>,
    Json(req): Json<CreateSubscriberRequest>,
) -> Result<Json<Subscriber>, AppError> {
    let result = subscriber_service::create_or_retrieve(&state, &req).await?;
    Ok(Json(result.subscriber))
}

#[utoipa::path(
    get,
    path = "/api/v1/subscribers/{uuid}",
    params(("uuid" = Uuid, Path, description = "Subscriber UUID")),
    responses(
        (status = 200, description = "Subscriber found", body = Subscriber),
        (status = 404, description = "Not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_subscriber(
    State(state): State<AppState>,
    Path(uuid): Path<Uuid>,
) -> Result<Json<Subscriber>, AppError> {
    let subscriber = Subscriber::find_by_uuid(&state.pool, uuid)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(subscriber))
}

#[utoipa::path(
    patch,
    path = "/api/v1/subscribers/{uuid}",
    params(("uuid" = Uuid, Path, description = "Subscriber UUID")),
    request_body = UpdateSubscriberRequest,
    responses(
        (status = 200, description = "Subscriber updated", body = Subscriber),
        (status = 404, description = "Not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn update_subscriber(
    State(state): State<AppState>,
    Path(uuid): Path<Uuid>,
    Json(req): Json<UpdateSubscriberRequest>,
) -> Result<Json<Subscriber>, AppError> {
    let subscriber = Subscriber::update(&state.pool, uuid, &req)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(subscriber))
}

#[utoipa::path(
    post,
    path = "/api/v1/subscribers/{uuid}/unsubscribe",
    params(("uuid" = Uuid, Path, description = "Subscriber UUID")),
    responses(
        (status = 200, description = "Unsubscribed", body = Subscriber),
        (status = 404, description = "Not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn unsubscribe_subscriber(
    State(state): State<AppState>,
    Path(uuid): Path<Uuid>,
) -> Result<Json<Subscriber>, AppError> {
    let subscriber = Subscriber::find_by_uuid(&state.pool, uuid)
        .await?
        .ok_or(AppError::NotFound)?;
    let updated = Subscriber::unsubscribe_all(&state.pool, subscriber.id).await?;
    tracing::info!(email = %updated.email, "Subscriber unsubscribed from all newsletters");
    Ok(Json(updated))
}
