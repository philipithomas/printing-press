use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, patch, post},
};
use uuid::Uuid;

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::error::AppError;
use crate::models::subscriber::{CreateSubscriberRequest, Subscriber, UpdateSubscriberRequest};
use crate::services::subscriber_service;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct DeleteResponse {
    pub success: bool,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ExportQuery {
    /// Return subscribers with id greater than this (keyset pagination).
    #[serde(default)]
    pub after_id: i64,
    /// Page size, capped at 1000.
    #[serde(default = "default_export_limit")]
    pub limit: i64,
}

fn default_export_limit() -> i64 {
    500
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ExportPage {
    pub subscribers: Vec<Subscriber>,
    pub total: i64,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/subscribers",
            post(create_subscriber).get(export_subscribers),
        )
        .route("/subscribers/{uuid}", get(get_subscriber))
        .route("/subscribers/{uuid}", patch(update_subscriber))
        .route(
            "/subscribers/{uuid}/unsubscribe",
            post(unsubscribe_subscriber),
        )
        .route("/subscribers/{uuid}", delete(delete_subscriber))
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

/// One-time migration export: pages through every subscriber (including
/// unconfirmed and fully-unsubscribed rows) so the successor system can
/// mirror the data exactly. Auth is the same x-api-key as the other
/// authenticated routes.
#[utoipa::path(
    get,
    path = "/api/v1/subscribers",
    params(ExportQuery),
    responses(
        (status = 200, description = "Page of subscribers", body = ExportPage),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn export_subscribers(
    State(state): State<AppState>,
    Query(query): Query<ExportQuery>,
) -> Result<Json<ExportPage>, AppError> {
    let limit = query.limit.clamp(1, 1000);
    let subscribers = Subscriber::list_page(&state.pool, query.after_id, limit).await?;
    let total = Subscriber::count_all(&state.pool).await?;
    Ok(Json(ExportPage { subscribers, total }))
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

#[utoipa::path(
    delete,
    path = "/api/v1/subscribers/{uuid}",
    params(("uuid" = Uuid, Path, description = "Subscriber UUID")),
    responses(
        (status = 200, description = "Subscriber deleted", body = DeleteResponse),
        (status = 404, description = "Not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn delete_subscriber(
    State(state): State<AppState>,
    Path(uuid): Path<Uuid>,
) -> Result<Json<DeleteResponse>, AppError> {
    let subscriber = Subscriber::find_by_uuid(&state.pool, uuid)
        .await?
        .ok_or(AppError::NotFound)?;
    Subscriber::delete_with_data(&state.pool, subscriber.id).await?;
    tracing::info!(subscriber_uuid = %uuid, "Subscriber account deleted via authenticated API");
    Ok(Json(DeleteResponse { success: true }))
}
