use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;
use utoipa::ToSchema;

use crate::models::subscriber::Subscriber;
use crate::state::AppState;

#[derive(Serialize, ToSchema)]
pub struct SubscriberCountResponse {
    pub count: i64,
}

pub fn public_routes() -> Router<AppState> {
    Router::new().route("/api/v1/stats/subscribers/count", get(subscriber_count))
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/subscribers/count",
    responses(
        (status = 200, description = "Active subscriber count", body = SubscriberCountResponse)
    )
)]
async fn subscriber_count(State(state): State<AppState>) -> Json<SubscriberCountResponse> {
    let count = Subscriber::count_active(&state.pool).await.unwrap_or(0);
    Json(SubscriberCountResponse { count })
}
