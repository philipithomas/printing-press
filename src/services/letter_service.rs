use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::config::Config;
use crate::models::mail_send::MailSend;
use crate::services::lob_service::{self, CreateLetterParams};
use crate::services::pdf_service;
use crate::services::stripe_service::{self, ShippingAddress, StripeCustomer};

#[derive(Debug, Clone)]
pub struct PostData {
    pub slug: String,
    pub newsletter: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub published_at: Option<String>,
    pub cover_image: Option<String>,
    pub cover_image_alt: Option<String>,
    pub html_content: String,
}

#[derive(Debug)]
pub struct MailSummary {
    pub sent: u32,
    pub skipped: u32,
    pub errors: u32,
}

fn idempotency_key(slug: &str, stripe_customer_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{}:{}", slug, stripe_customer_id));
    format!("{:x}", hasher.finalize())
}

fn tracked_url(site_url: &str, slug: &str, customer_id: &str) -> String {
    format!(
        "{}/{}?utm_source=snailmail&utm_medium=letter&utm_campaign={}&utm_content=stripe_{}&utm_id={}",
        site_url, slug, slug, customer_id, customer_id
    )
}

fn carrier_for(shipping: &ShippingAddress) -> &'static str {
    let country = shipping.address_country.to_uppercase();
    if country == "US" || country == "USA" {
        "usps_standard"
    } else {
        "usps_first_class"
    }
}

fn logo_file_for(newsletter: &str) -> &'static str {
    match newsletter {
        "workshop" => "workshop-brand.svg",
        "postcard" => "postcard.svg",
        _ => "contraption.svg",
    }
}

fn generate_qr_svg(url: &str) -> anyhow::Result<String> {
    let code = qrcode::QrCode::new(url)?;
    let svg = code
        .render::<qrcode::render::svg::Color<'_>>()
        .quiet_zone(false)
        .min_dimensions(100, 100)
        .max_dimensions(150, 150)
        .build();
    Ok(svg)
}

pub async fn send_letter(
    pool: &PgPool,
    config: &Config,
    customer: &StripeCustomer,
    post: &PostData,
) -> anyhow::Result<()> {
    let shipping = customer
        .shipping
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Customer {} has no shipping address", customer.id))?;

    let key = idempotency_key(&post.slug, &customer.id);

    if MailSend::exists_by_idempotency_key(pool, &key).await? {
        tracing::info!(
            "Skipping {} — already sent for post '{}'",
            customer.id,
            post.slug
        );
        return Ok(());
    }

    let url = tracked_url(&config.site_url, &post.slug, &customer.id);
    let qr_svg = generate_qr_svg(&url)?;
    let logo_file = logo_file_for(&post.newsletter);

    let letter_html = crate::templates::render_letter(
        &post.title,
        post.subtitle.as_deref(),
        post.published_at.as_deref(),
        &post.html_content,
        post.cover_image.as_deref(),
        post.cover_image_alt.as_deref(),
        &post.newsletter,
        logo_file,
        &config.site_url,
        &qr_svg,
    )
    .map_err(|e| anyhow::anyhow!("Template rendering failed: {}", e))?;

    let chromium_path = if config.chromium_path.is_empty() {
        pdf_service::find_chromium()
            .ok_or_else(|| anyhow::anyhow!("No chromium binary found. Set CHROMIUM_PATH."))?
    } else {
        config.chromium_path.clone()
    };

    let pdf_bytes = pdf_service::render_pdf(&letter_html, &chromium_path)?;
    let pages = pdf_service::count_pages(&pdf_bytes)? as i32;
    let double_sided = pages > 5;
    let carrier = carrier_for(shipping);

    let mail_send = MailSend::create(
        pool,
        &customer.id,
        customer.email.as_deref(),
        &post.slug,
        &post.newsletter,
        &key,
    )
    .await?;

    match lob_service::create_letter(
        &config.lob_api_key,
        CreateLetterParams {
            to: shipping,
            pdf_bytes,
            color: true,
            double_sided,
            mail_type: carrier,
            idempotency_key: &key,
            post_slug: &post.slug,
            tracked_url: &url,
            stripe_customer_id: &customer.id,
            customer_email: customer.email.as_deref(),
        },
    )
    .await
    {
        Ok(lob_resp) => {
            MailSend::mark_sent(
                pool,
                mail_send.id,
                &lob_resp.id,
                pages,
                double_sided,
                carrier,
            )
            .await?;
            tracing::info!(
                "Sent letter {} to {} for post '{}'",
                lob_resp.id,
                customer.id,
                post.slug
            );
        }
        Err(e) => {
            let error_msg = e.to_string();
            let _ = MailSend::record_error(pool, mail_send.id, &error_msg).await;
            tracing::error!(
                "Failed to send letter to {} for post '{}': {}",
                customer.id,
                post.slug,
                error_msg
            );
            return Err(e);
        }
    }

    Ok(())
}

pub async fn send_all(
    pool: &PgPool,
    config: &Config,
    post: &PostData,
    force: bool,
) -> anyhow::Result<MailSummary> {
    let customers =
        stripe_service::list_mail_subscribers(&config.stripe_secret_key, &config.stripe_product_id)
            .await?;

    let mut sent = 0u32;
    let mut skipped = 0u32;
    let mut errors = 0u32;

    for customer in &customers {
        let key = idempotency_key(&post.slug, &customer.id);
        if !force
            && MailSend::exists_by_idempotency_key(pool, &key)
                .await
                .unwrap_or(false)
        {
            skipped += 1;
            continue;
        }

        match send_letter(pool, config, customer, post).await {
            Ok(()) => sent += 1,
            Err(e) => {
                tracing::error!("Error sending to {}: {}", customer.id, e);
                errors += 1;
            }
        }
    }

    Ok(MailSummary {
        sent,
        skipped,
        errors,
    })
}
