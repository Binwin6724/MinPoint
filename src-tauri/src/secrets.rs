//! OS credential storage (macOS Keychain / Windows Credential Manager via `keyring`).
//! Values stored here never cross into the webview and are never logged.

use base64::Engine;
use rand::RngCore;

const SERVICE: &str = "app.minpoint.desktop";

fn entry(account: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, account).map_err(|e| format!("credential store unavailable: {e}"))
}

pub fn get(account: &str) -> Result<Option<String>, String> {
    match entry(account)?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("could not read from the credential store: {e}")),
    }
}

pub fn set(account: &str, value: &str) -> Result<(), String> {
    entry(account)?.set_password(value).map_err(|e| format!("could not write to the credential store: {e}"))
}

pub fn delete(account: &str) -> Result<(), String> {
    match entry(account)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("could not delete from the credential store: {e}")),
    }
}

/// 32-byte key for encrypting local clipboard history; created on first use.
pub fn database_key() -> Result<[u8; 32], String> {
    let b64 = base64::engine::general_purpose::STANDARD;
    if let Some(v) = get("database-key")? {
        if let Ok(bytes) = b64.decode(v) {
            if let Ok(k) = <[u8; 32]>::try_from(bytes.as_slice()) {
                return Ok(k);
            }
        }
    }
    let mut k = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut k);
    set("database-key", &b64.encode(k))?;
    Ok(k)
}
