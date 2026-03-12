use crate::config::EnvConfig;
use crate::keystore;

pub fn run(env_config: &EnvConfig) -> anyhow::Result<()> {
    println!("Storing API key for {} environment", env_config.name);

    let api_key = rpassword::prompt_password("Enter M2M API key: ")?;
    if api_key.is_empty() {
        anyhow::bail!("API key cannot be empty");
    }

    let password = rpassword::prompt_password("Enter encryption password: ")?;
    if password.is_empty() {
        anyhow::bail!("Password cannot be empty");
    }

    let password_confirm = rpassword::prompt_password("Confirm encryption password: ")?;
    if password != password_confirm {
        anyhow::bail!("Passwords do not match");
    }

    keystore::encrypt_and_store(env_config, &api_key, &password)?;

    let path = keystore::key_path(env_config)?;
    println!("API key encrypted and saved to {}", path.display());

    Ok(())
}
