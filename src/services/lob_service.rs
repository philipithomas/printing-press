use reqwest::multipart;
use serde::Deserialize;

use super::stripe_service::ShippingAddress;

#[derive(Debug, Deserialize)]
pub struct LobLetterResponse {
    pub id: String,
}

pub struct CreateLetterParams<'a> {
    pub to: &'a ShippingAddress,
    pub pdf_bytes: Vec<u8>,
    pub color: bool,
    pub double_sided: bool,
    pub mail_type: &'a str,
    pub idempotency_key: &'a str,
    pub post_slug: &'a str,
    pub tracked_url: &'a str,
    pub stripe_customer_id: &'a str,
    pub customer_email: Option<&'a str>,
}

pub async fn create_letter(
    lob_api_key: &str,
    params: CreateLetterParams<'_>,
) -> anyhow::Result<LobLetterResponse> {
    let to = params.to;

    let mut form = multipart::Form::new()
        .text("to[name]", to.name.clone())
        .text("to[address_line1]", to.address_line1.clone())
        .text("to[address_city]", to.address_city.clone())
        .text("to[address_country]", to.address_country.clone())
        .text("from[name]", "Contraption Co.")
        .text("from[address_line1]", "315 Montgomery St")
        .text("from[address_line2]", "Ste 900")
        .text("from[address_city]", "San Francisco")
        .text("from[address_state]", "CA")
        .text("from[address_zip]", "94104")
        .text("from[address_country]", "US")
        .text("color", params.color.to_string())
        .text("double_sided", params.double_sided.to_string())
        .text("address_placement", "insert_blank_page")
        .text("mail_type", params.mail_type.to_string())
        .text("use_type", "operational")
        .text("metadata[post_slug]", params.post_slug.to_string())
        .text("metadata[post_url]", params.tracked_url.to_string())
        .text(
            "metadata[stripe_customer_id]",
            params.stripe_customer_id.to_string(),
        )
        .part(
            "file",
            multipart::Part::bytes(params.pdf_bytes)
                .file_name("letter.pdf")
                .mime_str("application/pdf")?,
        );

    if let Some(line2) = &to.address_line2 {
        form = form.text("to[address_line2]", line2.clone());
    }
    if let Some(state) = &to.address_state {
        form = form.text("to[address_state]", state.clone());
    }
    if let Some(zip) = &to.address_zip {
        form = form.text("to[address_zip]", zip.clone());
    }
    if let Some(email) = params.customer_email {
        form = form.text("metadata[customer_email]", email.to_string());
    }

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.lob.com/v1/letters")
        .basic_auth(lob_api_key, Some(""))
        .header("Idempotency-Key", params.idempotency_key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Lob API request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Lob API error ({}): {}", status, body);
    }

    let letter: LobLetterResponse = resp.json().await?;
    Ok(letter)
}
