//! Groq API key storage in the macOS Keychain.
//!
//! The key never goes to `settings_store.json` and is never returned to the
//! frontend. The existence check reads item attributes only, so it does not
//! trigger the Keychain access prompt that reading the secret can show for
//! unsigned dev builds.

use keyring::Entry;
use security_framework::item::{ItemClass, ItemSearchOptions, Limit};

const SERVICE: &str = "com.kennnguyen.audible";
const ACCOUNT: &str = "groq_api_key";

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
        .map_err(|e| format!("keychain: {e}"))
}

pub fn get_groq_api_key() -> Option<String> {
    match entry().ok()?.get_password() {
        Ok(key) if !key.trim().is_empty() => Some(key),
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
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("keychain: {e}")),
    }
}
