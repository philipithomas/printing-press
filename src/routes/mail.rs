use axum::extract::Path;
use axum::response::Html;
use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::AppError;
use crate::models::mail_send::MailSend;
use crate::services::{letter_service, stripe_service};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/mail/validate", post(validate))
        .route("/mail/send", post(send))
        .route("/mail/send-one", post(send_one))
        .route("/mail/preview/{slug}", get(preview))
}

const VALID_NEWSLETTERS: [&str; 3] = ["postcard", "contraption", "workshop"];

fn validate_newsletter(newsletter: &str) -> Result<(), AppError> {
    if VALID_NEWSLETTERS.contains(&newsletter) {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!(
            "Invalid newsletter '{}'. Must be one of: postcard, contraption, workshop",
            newsletter
        )))
    }
}

// --- Validate ---

#[derive(Debug, Deserialize, ToSchema)]
pub struct MailValidateRequest {
    pub post_slug: String,
    pub newsletter: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MailValidateResponse {
    pub post_slug: String,
    pub newsletter: String,
    pub eligible_recipients: u32,
    pub already_sent: i64,
}

#[utoipa::path(
    post,
    path = "/api/v1/mail/validate",
    request_body = MailValidateRequest,
    responses(
        (status = 200, description = "Mail validation result", body = MailValidateResponse),
        (status = 400, description = "Invalid newsletter"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn validate(
    State(state): State<AppState>,
    Json(req): Json<MailValidateRequest>,
) -> Result<Json<MailValidateResponse>, AppError> {
    validate_newsletter(&req.newsletter)?;

    let customers = stripe_service::list_mail_subscribers(
        &state.config.stripe_secret_key,
        &state.config.stripe_product_id,
    )
    .await
    .map_err(|e| AppError::Internal(format!("Stripe error: {}", e)))?;

    let already_sent = MailSend::count_sent_for_slug(&state.pool, &req.post_slug).await?;

    Ok(Json(MailValidateResponse {
        post_slug: req.post_slug,
        newsletter: req.newsletter,
        eligible_recipients: customers.len() as u32,
        already_sent,
    }))
}

// --- Send (bulk) ---

#[derive(Debug, Deserialize, ToSchema)]
pub struct MailSendRequest {
    pub post_slug: String,
    pub newsletter: String,
    pub subject: String,
    pub html_content: String,
    pub subtitle: Option<String>,
    pub published_at: Option<String>,
    pub cover_image: Option<String>,
    pub cover_image_alt: Option<String>,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MailSendResponse {
    pub sent: u32,
    pub skipped: u32,
    pub errors: u32,
}

#[utoipa::path(
    post,
    path = "/api/v1/mail/send",
    request_body = MailSendRequest,
    responses(
        (status = 200, description = "Mail send result", body = MailSendResponse),
        (status = 400, description = "Invalid newsletter"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Already sent, use force flag"),
    )
)]
pub async fn send(
    State(state): State<AppState>,
    Json(req): Json<MailSendRequest>,
) -> Result<Json<MailSendResponse>, AppError> {
    validate_newsletter(&req.newsletter)?;

    let already_sent = MailSend::count_sent_for_slug(&state.pool, &req.post_slug).await?;
    if already_sent > 0 && !req.force {
        return Err(AppError::Conflict(format!(
            "{} letters have already been sent for this post. Use force=true to send to remaining recipients.",
            already_sent
        )));
    }

    let post_data = letter_service::PostData {
        slug: req.post_slug,
        newsletter: req.newsletter,
        title: req.subject,
        subtitle: req.subtitle,
        published_at: req.published_at,
        cover_image: req.cover_image,
        cover_image_alt: req.cover_image_alt,
        html_content: req.html_content,
    };

    let summary = letter_service::send_all(&state.pool, &state.config, &post_data, req.force)
        .await
        .map_err(|e| AppError::Internal(format!("Mail send failed: {}", e)))?;

    Ok(Json(MailSendResponse {
        sent: summary.sent,
        skipped: summary.skipped,
        errors: summary.errors,
    }))
}

// --- Send One ---

#[derive(Debug, Deserialize, ToSchema)]
pub struct MailSendOneRequest {
    pub email: String,
    pub post_slug: String,
    pub newsletter: String,
    pub subject: String,
    pub html_content: String,
    pub subtitle: Option<String>,
    pub published_at: Option<String>,
    pub cover_image: Option<String>,
    pub cover_image_alt: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MailSendOneResponse {
    pub status: String,
    pub error: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/v1/mail/send-one",
    request_body = MailSendOneRequest,
    responses(
        (status = 200, description = "Single mail send result", body = MailSendOneResponse),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn send_one(
    State(state): State<AppState>,
    Json(req): Json<MailSendOneRequest>,
) -> Result<Json<MailSendOneResponse>, AppError> {
    let customer =
        stripe_service::find_customer_by_email(&state.config.stripe_secret_key, &req.email)
            .await
            .map_err(|e| AppError::Internal(format!("Stripe lookup failed: {}", e)))?
            .ok_or_else(|| {
                AppError::BadRequest(format!(
                    "No Stripe customer found with email '{}'",
                    req.email
                ))
            })?;

    if customer.shipping.is_none() {
        return Err(AppError::BadRequest(format!(
            "Stripe customer '{}' has no shipping address",
            req.email
        )));
    }

    let post_data = letter_service::PostData {
        slug: req.post_slug,
        newsletter: req.newsletter,
        title: req.subject,
        subtitle: req.subtitle,
        published_at: req.published_at,
        cover_image: req.cover_image,
        cover_image_alt: req.cover_image_alt,
        html_content: req.html_content,
    };

    match letter_service::send_letter(&state.pool, &state.config, &customer, &post_data).await {
        Ok(()) => Ok(Json(MailSendOneResponse {
            status: "sent".to_string(),
            error: None,
        })),
        Err(e) => Ok(Json(MailSendOneResponse {
            status: "error".to_string(),
            error: Some(e.to_string()),
        })),
    }
}

// --- Preview ---

#[derive(Debug, Deserialize)]
pub struct PreviewQuery {
    pub newsletter: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/mail/preview/{slug}",
    params(
        ("slug" = String, Path, description = "Post slug"),
        ("newsletter" = Option<String>, Query, description = "Newsletter name"),
    ),
    responses(
        (status = 200, description = "Letter HTML preview", content_type = "text/html"),
        (status = 404, description = "Post not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn preview(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    axum::extract::Query(query): axum::extract::Query<PreviewQuery>,
) -> Result<Html<String>, AppError> {
    // Fetch post from bully-pulpit
    let url = format!("{}/api/posts/{}", state.config.site_url, slug);
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch post: {}", e)))?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(AppError::NotFound);
    }
    if !resp.status().is_success() {
        return Err(AppError::Internal(format!(
            "Website returned {} for post '{}'",
            resp.status(),
            slug
        )));
    }

    let post_info: PostApiResponse = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to parse post: {}", e)))?;

    let newsletter = query.newsletter.as_deref().unwrap_or(&post_info.newsletter);
    let logo_file = match newsletter {
        "workshop" => "workshop-brand.svg",
        "postcard" => "postcard.svg",
        _ => "contraption.svg",
    };

    let dummy_url = format!("{}/{}", state.config.site_url, slug);
    let qr_svg = qrcode::QrCode::new(&dummy_url)
        .map(|code| {
            code.render::<qrcode::render::svg::Color<'_>>()
                .quiet_zone(false)
                .min_dimensions(100, 100)
                .max_dimensions(150, 150)
                .build()
        })
        .unwrap_or_default();

    let html = crate::templates::render_letter(
        &post_info.title,
        post_info.subtitle.as_deref(),
        post_info.published_at.as_deref(),
        &post_info.email_html,
        post_info.cover_image.as_deref(),
        post_info.cover_image_alt.as_deref(),
        newsletter,
        logo_file,
        &state.config.site_url,
        &qr_svg,
    )
    .map_err(|e| AppError::Internal(format!("Template error: {}", e)))?;

    Ok(Html(html))
}

#[derive(Debug, Deserialize)]
struct PostApiResponse {
    title: String,
    newsletter: String,
    published_at: Option<String>,
    email_html: String,
    #[serde(default)]
    subtitle: Option<String>,
    #[serde(default)]
    cover_image: Option<String>,
    #[serde(default)]
    cover_image_alt: Option<String>,
}
