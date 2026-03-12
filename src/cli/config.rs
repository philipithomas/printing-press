pub struct EnvConfig {
    pub name: &'static str,
    pub server_url: &'static str,
    pub website_url: &'static str,
    pub key_filename: &'static str,
}

pub fn resolve_env(env_str: &str) -> anyhow::Result<EnvConfig> {
    match env_str {
        "development" | "dev" => Ok(EnvConfig {
            name: "development",
            server_url: "http://localhost:8080",
            website_url: "http://localhost:3000",
            key_filename: "dev.key",
        }),
        "production" | "prd" => Ok(EnvConfig {
            name: "production",
            server_url: "https://printing-press.contraption.co",
            website_url: "https://philipithomas.com",
            key_filename: "prd.key",
        }),
        other => anyhow::bail!(
            "Unknown environment: '{}'. Use 'development' or 'prd'.",
            other
        ),
    }
}
