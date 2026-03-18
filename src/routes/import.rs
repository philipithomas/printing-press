use axum::{Json, Router, extract::State, routing::post};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::models::subscriber::{ImportResult, ImportSubscriberEntry, Subscriber};
use crate::state::AppState;

const MAX_BATCH_SIZE: usize = 1000;

#[derive(Debug, Deserialize, ToSchema)]
pub struct ImportRequest {
    pub subscribers: Vec<ImportSubscriberEntry>,
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/subscribers/import", post(import_subscribers))
}

#[utoipa::path(
    post,
    path = "/api/v1/subscribers/import",
    request_body = ImportRequest,
    responses(
        (status = 200, description = "Import complete", body = ImportResult),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn import_subscribers(
    State(state): State<AppState>,
    Json(req): Json<ImportRequest>,
) -> Result<Json<ImportResult>, AppError> {
    if req.subscribers.is_empty() {
        return Ok(Json(ImportResult {
            created: 0,
            updated: 0,
            total: 0,
        }));
    }

    if req.subscribers.len() > MAX_BATCH_SIZE {
        return Err(AppError::BadRequest(format!(
            "Batch size {} exceeds maximum of {}",
            req.subscribers.len(),
            MAX_BATCH_SIZE
        )));
    }

    for entry in &req.subscribers {
        if entry.email.is_empty() {
            return Err(AppError::BadRequest(
                "Email is required for all entries".to_string(),
            ));
        }
        for newsletter in &entry.newsletters {
            if !matches!(newsletter.as_str(), "postcard" | "contraption" | "workshop") {
                return Err(AppError::BadRequest(format!(
                    "Invalid newsletter: {}",
                    newsletter
                )));
            }
        }
    }

    let result = Subscriber::bulk_import(&state.pool, &req.subscribers).await?;
    Ok(Json(result))
}
