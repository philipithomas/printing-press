use figment::{Figment, providers::Env};
use serde::Deserialize;

/// Base URL for the production website, used to resolve relative links in email content.
pub const SITE_BASE_URL: &str = "https://www.philipithomas.com";
pub const PRODUCTION_PUBLIC_URL: &str = "https://printing-press.contraption.co";
const DEFAULT_PUBLIC_URL: &str = "http://localhost:8080";

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_database_url")]
    pub database_url: String,
    #[serde(default = "default_api_key")]
    pub m2m_api_key: String,
    #[serde(default = "default_aws_region")]
    pub aws_region: String,
    #[serde(default = "default_ses_from_email")]
    pub ses_from_email: String,
    #[serde(default = "default_site_url")]
    pub site_url: String,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_email_backend")]
    pub email_backend: String,
    #[serde(default = "default_smtp_host")]
    pub smtp_host: String,
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,
    #[serde(default = "default_ses_rate_per_second")]
    pub ses_rate_per_second: u32,
    #[serde(default = "default_public_url")]
    pub public_url: String,
}

fn default_database_url() -> String {
    "postgres://postgres:postgres@localhost:5433/printing_press".to_string()
}
fn default_api_key() -> String {
    "dev-api-key".to_string()
}
fn default_aws_region() -> String {
    "us-east-1".to_string()
}
fn default_ses_from_email() -> String {
    "Philip I. Thomas <mail@philipithomas.com>".to_string()
}
fn default_site_url() -> String {
    "http://localhost:3000".to_string()
}
fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    8080
}
fn default_email_backend() -> String {
    "smtp".to_string()
}
fn default_smtp_host() -> String {
    "localhost".to_string()
}
fn default_smtp_port() -> u16 {
    1025
}
fn default_ses_rate_per_second() -> u32 {
    14
}
fn default_public_url() -> String {
    DEFAULT_PUBLIC_URL.to_string()
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let mut config: Self = Figment::new().merge(Env::raw()).extract()?;
        config.normalize_for_delivery();
        config.validate_delivery_urls()?;
        Ok(config)
    }

    pub fn unsubscribe_url(&self, token: impl std::fmt::Display) -> String {
        format!("{}/unsubscribe?token={}", self.site_url, token)
    }

    pub fn unsubscribe_post_url(&self, token: impl std::fmt::Display) -> String {
        format!("{}/api/v1/unsubscribe/{}", self.public_url, token)
    }

    fn normalize_for_delivery(&mut self) {
        self.site_url = normalize_url(&self.site_url);
        self.public_url = normalize_url(&self.public_url);

        if self.public_url == DEFAULT_PUBLIC_URL && self.uses_production_site() {
            self.public_url = PRODUCTION_PUBLIC_URL.to_string();
        }
    }

    fn validate_delivery_urls(&self) -> anyhow::Result<()> {
        if !self.sends_external_email() {
            return Ok(());
        }

        if is_local_url(&self.public_url) {
            anyhow::bail!(
                "PUBLIC_URL must be a public HTTPS URL when sending production email; got {}",
                self.public_url
            );
        }

        if !self.public_url.starts_with("https://") {
            anyhow::bail!(
                "PUBLIC_URL must use HTTPS when sending production email; got {}",
                self.public_url
            );
        }

        Ok(())
    }

    fn sends_external_email(&self) -> bool {
        self.email_backend == "ses" || self.uses_production_site()
    }

    fn uses_production_site(&self) -> bool {
        self.site_url.contains("philipithomas.com")
    }
}

fn normalize_url(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

fn is_local_url(url: &str) -> bool {
    url.contains("localhost") || url.contains("127.0.0.1") || url.contains("0.0.0.0")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> Config {
        Config {
            database_url: default_database_url(),
            m2m_api_key: default_api_key(),
            aws_region: default_aws_region(),
            ses_from_email: default_ses_from_email(),
            site_url: default_site_url(),
            host: default_host(),
            port: default_port(),
            email_backend: default_email_backend(),
            smtp_host: default_smtp_host(),
            smtp_port: default_smtp_port(),
            ses_rate_per_second: default_ses_rate_per_second(),
            public_url: default_public_url(),
        }
    }

    #[test]
    fn production_site_falls_back_to_production_public_url() {
        let mut config = base_config();
        config.site_url = "https://philipithomas.com/".to_string();

        config.normalize_for_delivery();

        assert_eq!(config.site_url, "https://philipithomas.com");
        assert_eq!(config.public_url, PRODUCTION_PUBLIC_URL);
    }

    #[test]
    fn production_delivery_rejects_local_public_url() {
        let mut config = base_config();
        config.site_url = "https://philipithomas.com".to_string();
        config.public_url = "http://127.0.0.1:8080".to_string();

        config.normalize_for_delivery();
        let err = config.validate_delivery_urls().unwrap_err().to_string();

        assert!(err.contains("PUBLIC_URL must be a public HTTPS URL"));
    }

    #[test]
    fn production_delivery_rejects_non_https_public_url() {
        let mut config = base_config();
        config.site_url = "https://philipithomas.com".to_string();
        config.public_url = "http://printing-press.contraption.co".to_string();

        config.normalize_for_delivery();
        let err = config.validate_delivery_urls().unwrap_err().to_string();

        assert!(err.contains("PUBLIC_URL must use HTTPS"));
    }

    #[test]
    fn unsubscribe_links_use_normalized_urls() {
        let mut config = base_config();
        config.site_url = "http://localhost:3000/".to_string();
        config.public_url = "http://localhost:8080/".to_string();

        config.normalize_for_delivery();

        assert_eq!(
            config.unsubscribe_url("token"),
            "http://localhost:3000/unsubscribe?token=token"
        );
        assert_eq!(
            config.unsubscribe_post_url("token"),
            "http://localhost:8080/api/v1/unsubscribe/token"
        );
    }
}
