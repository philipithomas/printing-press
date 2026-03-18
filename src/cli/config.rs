pub enum ApiKeySource {
    HardCoded(&'static str),
    OnePassword(&'static str),
}

pub struct EnvConfig {
    pub name: &'static str,
    pub server_url: &'static str,
    pub website_url: &'static str,
    pub api_key_source: ApiKeySource,
}

pub fn resolve_env(env_str: &str) -> anyhow::Result<EnvConfig> {
    match env_str {
        "development" | "dev" => Ok(EnvConfig {
            name: "development",
            server_url: "http://localhost:8080",
            website_url: "http://localhost:3000",
            api_key_source: ApiKeySource::HardCoded("dev-api-key"),
        }),
        "production" | "prd" => Ok(EnvConfig {
            name: "production",
            server_url: "https://printing-press.contraption.co",
            website_url: "https://www.philipithomas.com",
            api_key_source: ApiKeySource::OnePassword("printing-press"),
        }),
        other => anyhow::bail!(
            "Unknown environment: '{}'. Use 'development' or 'prd'.",
            other
        ),
    }
}

pub fn read_api_key(env_config: &EnvConfig) -> anyhow::Result<String> {
    match &env_config.api_key_source {
        ApiKeySource::HardCoded(key) => Ok(key.to_string()),
        ApiKeySource::OnePassword(item) => {
            let output = std::process::Command::new("op")
                .args(["item", "get", item, "--field", "M2M_API_KEY", "--reveal"])
                .output()
                .map_err(|e| {
                    anyhow::anyhow!("Failed to run `op` CLI. Is 1Password CLI installed? {}", e)
                })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("1Password lookup failed: {}", stderr.trim());
            }

            let key = String::from_utf8(output.stdout)
                .map_err(|_| anyhow::anyhow!("Invalid UTF-8 from 1Password"))?
                .trim()
                .to_string();

            if key.is_empty() {
                anyhow::bail!("M2M_API_KEY is empty in 1Password item '{}'", item);
            }

            Ok(key)
        }
    }
}
