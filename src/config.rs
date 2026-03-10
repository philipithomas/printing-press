use figment::{Figment, providers::Env};
use serde::Deserialize;

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
}

fn default_database_url() -> String {
    "postgres://postgres:postgres@localhost:5432/printing_press".to_string()
}
fn default_api_key() -> String {
    "dev-api-key".to_string()
}
fn default_aws_region() -> String {
    "us-east-1".to_string()
}
fn default_ses_from_email() -> String {
    "mail@philipithomas.com".to_string()
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

impl Config {
    pub fn load() -> Result<Self, Box<figment::Error>> {
        Figment::new()
            .merge(Env::raw())
            .extract()
            .map_err(Box::new)
    }
}
