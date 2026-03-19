use printing_press::models::subscriber::{ImportResult, ImportSubscriberEntry};
use serde::{Deserialize, Serialize};

use crate::config::EnvConfig;

pub struct PpClient {
    client: reqwest::Client,
    server_url: String,
    website_url: String,
    api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct PostInfo {
    pub title: String,
    pub newsletter: String,
    pub email_html: String,
    pub subtitle: Option<String>,
}

#[derive(Debug, Serialize)]
struct ValidateBody {
    post_slug: String,
    newsletter: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ValidateResponse {
    pub post_slug: String,
    pub newsletter: String,
    pub eligible_subscribers: i64,
    pub already_sent: i64,
}

#[derive(Debug, Serialize)]
struct SendBody {
    post_slug: String,
    newsletter: String,
    subject: String,
    html_content: String,
    force: bool,
    preview_text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SendResponse {
    pub enqueued: i64,
    pub already_sent: i64,
}

#[derive(Debug, Serialize)]
struct SendOneBody {
    email: String,
    post_slug: String,
    newsletter: String,
    subject: String,
    html_content: String,
    preview_text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SendOneResponse {
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: String,
}

impl PpClient {
    pub fn new(env_config: &EnvConfig, api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            server_url: env_config.server_url.to_string(),
            website_url: env_config.website_url.to_string(),
            api_key,
        }
    }

    pub async fn fetch_post(&self, slug: &str) -> anyhow::Result<PostInfo> {
        let url = format!("{}/api/posts/{}", self.website_url, slug);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to reach website at {}: {}", url, e))?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!("Post '{}' not found at {}", slug, url);
        }
        if !resp.status().is_success() {
            anyhow::bail!("Website returned {} for post '{}'", resp.status(), slug);
        }

        resp.json::<PostInfo>()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse post response: {}", e))
    }

    pub async fn validate(
        &self,
        post_slug: &str,
        newsletter: &str,
    ) -> anyhow::Result<ValidateResponse> {
        let url = format!("{}/api/v1/publish/validate", self.server_url);
        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .json(&ValidateBody {
                post_slug: post_slug.to_string(),
                newsletter: newsletter.to_string(),
            })
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to reach server at {}: {}", url, e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body: ErrorResponse = resp.json().await.unwrap_or(ErrorResponse {
                error: "Unknown error".to_string(),
            });
            anyhow::bail!("Validate failed ({}): {}", status, body.error);
        }

        Ok(resp.json().await?)
    }

    pub async fn send(
        &self,
        post_slug: &str,
        newsletter: &str,
        subject: &str,
        html_content: &str,
        force: bool,
        preview_text: Option<&str>,
    ) -> anyhow::Result<SendResponse> {
        let url = format!("{}/api/v1/publish/send", self.server_url);
        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .json(&SendBody {
                post_slug: post_slug.to_string(),
                newsletter: newsletter.to_string(),
                subject: subject.to_string(),
                html_content: html_content.to_string(),
                force,
                preview_text: preview_text.map(|s| s.to_string()),
            })
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to reach server: {}", e))?;

        if resp.status() == reqwest::StatusCode::CONFLICT {
            let body: ErrorResponse = resp.json().await.unwrap_or(ErrorResponse {
                error: "Already sent".to_string(),
            });
            anyhow::bail!("{}", body.error);
        }
        if !resp.status().is_success() {
            let status = resp.status();
            let body: ErrorResponse = resp.json().await.unwrap_or(ErrorResponse {
                error: "Unknown error".to_string(),
            });
            anyhow::bail!("Send failed ({}): {}", status, body.error);
        }

        Ok(resp.json().await?)
    }

    pub async fn send_one(
        &self,
        email: &str,
        post_slug: &str,
        newsletter: &str,
        subject: &str,
        html_content: &str,
        preview_text: Option<&str>,
    ) -> anyhow::Result<SendOneResponse> {
        let url = format!("{}/api/v1/publish/send-one", self.server_url);
        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .json(&SendOneBody {
                email: email.to_string(),
                post_slug: post_slug.to_string(),
                newsletter: newsletter.to_string(),
                subject: subject.to_string(),
                html_content: html_content.to_string(),
                preview_text: preview_text.map(|s| s.to_string()),
            })
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to reach server: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body: ErrorResponse = resp.json().await.unwrap_or(ErrorResponse {
                error: "Unknown error".to_string(),
            });
            anyhow::bail!("Send failed ({}): {}", status, body.error);
        }

        Ok(resp.json().await?)
    }

    pub async fn import_subscribers(
        &self,
        entries: Vec<ImportSubscriberEntry>,
    ) -> anyhow::Result<ImportResult> {
        let url = format!("{}/api/v1/subscribers/import", self.server_url);
        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .json(&serde_json::json!({ "subscribers": entries }))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to reach server at {}: {}", url, e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body: ErrorResponse = resp.json().await.unwrap_or(ErrorResponse {
                error: "Unknown error".to_string(),
            });
            anyhow::bail!("Import failed ({}): {}", status, body.error);
        }

        Ok(resp.json().await?)
    }
}
