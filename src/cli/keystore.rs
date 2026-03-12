use std::fs;
use std::path::PathBuf;

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Nonce};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

use crate::config::EnvConfig;

const SALT_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const PBKDF2_ITERATIONS: u32 = 600_000;

fn key_dir() -> anyhow::Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
    Ok(home.join(".printing-press"))
}

pub fn key_path(env_config: &EnvConfig) -> anyhow::Result<PathBuf> {
    Ok(key_dir()?.join(env_config.key_filename))
}

fn derive_key(password: &[u8], salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(password, salt, PBKDF2_ITERATIONS, &mut key);
    key
}

pub fn encrypt_and_store(
    env_config: &EnvConfig,
    api_key: &str,
    password: &str,
) -> anyhow::Result<()> {
    let salt: [u8; SALT_LEN] = rand::random();
    let derived_key = derive_key(password.as_bytes(), &salt);

    let cipher = Aes256Gcm::new_from_slice(&derived_key)
        .map_err(|e| anyhow::anyhow!("Cipher init error: {}", e))?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, api_key.as_bytes())
        .map_err(|e| anyhow::anyhow!("Encryption error: {}", e))?;

    // Format: salt || nonce || ciphertext (includes tag)
    let mut blob = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&ciphertext);

    let encoded = BASE64.encode(&blob);

    let dir = key_dir()?;
    fs::create_dir_all(&dir)?;

    let path = dir.join(env_config.key_filename);
    fs::write(&path, &encoded)?;

    // Set file permissions to 0600 (owner read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

pub fn decrypt(env_config: &EnvConfig, password: &str) -> anyhow::Result<String> {
    let path = key_path(env_config)?;
    let encoded = fs::read_to_string(&path).map_err(|_| {
        anyhow::anyhow!(
            "No API key found for {}. Run `pp login` first.",
            env_config.name
        )
    })?;

    let blob = BASE64
        .decode(encoded.trim())
        .map_err(|_| anyhow::anyhow!("Corrupted key file"))?;

    if blob.len() < SALT_LEN + NONCE_LEN + 1 {
        anyhow::bail!("Corrupted key file");
    }

    let salt = &blob[..SALT_LEN];
    let nonce_bytes = &blob[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ciphertext = &blob[SALT_LEN + NONCE_LEN..];

    let derived_key = derive_key(password.as_bytes(), salt);
    let cipher = Aes256Gcm::new_from_slice(&derived_key)
        .map_err(|e| anyhow::anyhow!("Cipher init error: {}", e))?;
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("Incorrect password or corrupted key file"))?;

    String::from_utf8(plaintext).map_err(|_| anyhow::anyhow!("Corrupted key data"))
}
