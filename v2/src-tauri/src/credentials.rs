use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64ct::{Base64, Encoding};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::Sha256;
use std::{collections::HashMap, fs, path::Path};

const CREDENTIALS_FILE: &str = "credentials.json";
const KEY_LEN: usize = 32;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const PBKDF2_ITERATIONS: u32 = 100_000;
const APP_PEPPER: &str = "vibewing-tauri-v2-credentials";

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct CredentialStore {
    #[serde(default)]
    salt: String,
    #[serde(default)]
    keys: HashMap<String, String>,
}

fn derive_password() -> String {
    let home = dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "vibewing".into());
    format!("{}::{}", APP_PEPPER, home)
}

fn derive_key(password: &str, salt: &[u8]) -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);
    key
}

fn read_store(data_dir: &Path) -> CredentialStore {
    fs::read_to_string(data_dir.join(CREDENTIALS_FILE))
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

fn write_store(data_dir: &Path, store: &CredentialStore) -> Result<(), String> {
    let path = data_dir.join(CREDENTIALS_FILE);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temporary = path.with_extension("tmp");
    let content = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(&temporary, content).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    fs::rename(temporary, path).map_err(|e| e.to_string())
}

fn ensure_salt(store: &mut CredentialStore) -> Vec<u8> {
    if store.salt.is_empty() {
        let mut salt = [0u8; SALT_LEN];
        rand::thread_rng().fill_bytes(&mut salt);
        store.salt = Base64::encode_string(&salt);
    }
    Base64::decode_vec(&store.salt).unwrap_or_default()
}

fn encrypt(value: &str, key: &[u8]) -> Result<String, String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| e.to_string())?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, value.as_bytes())
        .map_err(|e| e.to_string())?;
    let mut combined = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);
    Ok(Base64::encode_string(&combined))
}

fn decrypt(token: &str, key: &[u8]) -> Result<String, String> {
    let bytes = Base64::decode_vec(token).map_err(|e| e.to_string())?;
    if bytes.len() < NONCE_LEN {
        return Err("invalid credential token".into());
    }
    let (nonce_bytes, ciphertext) = bytes.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| e.to_string())?;
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| e.to_string())?;
    String::from_utf8(plaintext).map_err(|e| e.to_string())
}

/// Read a provider API key from the encrypted local store.
pub fn get(data_dir: &Path, provider_id: &str) -> Result<String, String> {
    let mut store = read_store(data_dir);
    let salt = ensure_salt(&mut store);
    let key = derive_key(&derive_password(), &salt);

    if let Some(token) = store.keys.get(provider_id) {
        return decrypt(token, &key);
    }

    Err("API key not configured".into())
}

/// Save or delete a provider API key in the encrypted local store.
pub fn set(data_dir: &Path, provider_id: &str, value: &str) -> Result<(), String> {
    let mut store = read_store(data_dir);
    let salt = ensure_salt(&mut store);
    let key = derive_key(&derive_password(), &salt);

    if value.is_empty() {
        store.keys.remove(provider_id);
    } else {
        let encrypted = encrypt(value, &key)?;
        store.keys.insert(provider_id.to_string(), encrypted);
    }

    write_store(data_dir, &store)
}
