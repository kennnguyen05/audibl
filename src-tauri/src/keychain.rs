//! Groq API key storage in the macOS Keychain.
//!
//! The key never goes to `settings_store.json` and is never returned to the
//! frontend. The existence check reads item attributes only, so it does not
//! trigger the Keychain access prompt that reading the secret can show for
//! unsigned dev builds.

use keyring::Entry;
use security_framework::item::{ItemClass, ItemSearchOptions, Limit};
use std::sync::Mutex;

const SERVICE: &str = "com.kennnguyen.audible";
const ACCOUNT: &str = "groq_api_key";

/// The key read once per launch, so dictations don't hit the Keychain (and
/// its access prompt in unsigned dev builds) every time.
static CACHED_KEY: Mutex<Option<String>> = Mutex::new(None);

fn entry() -> Result<Entry, String> {
    Entry::new(SERVICE, ACCOUNT).map_err(|e| format!("keychain: {e}"))
}

pub fn set_groq_api_key(key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("empty_key".into());
    }
    entry()?
        .set_password(key)
        .map_err(|e| format!("keychain: {e}"))?;
    *CACHED_KEY.lock().unwrap() = Some(key.to_string());
    Ok(())
}

pub fn get_groq_api_key() -> Option<String> {
    if let Some(key) = CACHED_KEY.lock().unwrap().clone() {
        return Some(key);
    }
    match entry().ok()?.get_password() {
        Ok(key) if !key.trim().is_empty() => {
            *CACHED_KEY.lock().unwrap() = Some(key.clone());
            Some(key)
        }
        Ok(_) => None,
        Err(keyring::Error::NoEntry) => None,
        Err(e) => {
            log::warn!("Failed to read Groq API key from Keychain: {e}");
            None
        }
    }
}

pub fn has_groq_api_key() -> bool {
    ItemSearchOptions::new()
        .class(ItemClass::generic_password())
        .service(SERVICE)
        .account(ACCOUNT)
        .load_attributes(true)
        .limit(Limit::Max(1))
        .search()
        .map(|results| !results.is_empty())
        .unwrap_or(false)
}

pub fn clear_groq_api_key() -> Result<(), String> {
    *CACHED_KEY.lock().unwrap() = None;
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("keychain: {e}")),
    }
}
