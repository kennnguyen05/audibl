//! Tauri commands called by the frontend. All are exported to
//! `src/bindings.ts` through tauri-specta. Setting commands return the full
//! updated settings so the frontend store stays in sync.

use crate::settings::{
    get_settings, normalize_custom_words, normalize_replacements, update_settings,
    ActivationMode, AppLanguage, AppSettings, Replacement,
};
use crate::{autostart, keychain, tray};
use tauri::AppHandle;

#[tauri::command]
#[specta::specta]
pub fn get_app_settings(app: AppHandle) -> AppSettings {
    get_settings(&app)
}

#[tauri::command]
#[specta::specta]
pub fn show_main_window_command(app: AppHandle) {
    crate::show_main_window(&app);
}

#[tauri::command]
#[specta::specta]
pub fn set_activation_mode(app: AppHandle, mode: ActivationMode) -> AppSettings {
    update_settings(&app, |s| s.activation_mode = mode)
}

#[tauri::command]
#[specta::specta]
pub fn set_mute_while_recording(app: AppHandle, enabled: bool) -> AppSettings {
    update_settings(&app, |s| s.mute_while_recording = enabled)
}

#[tauri::command]
#[specta::specta]
pub fn set_start_hidden(app: AppHandle, enabled: bool) -> AppSettings {
    update_settings(&app, |s| s.start_hidden = enabled)
}

#[tauri::command]
#[specta::specta]
pub fn set_autostart(app: AppHandle, enabled: bool) -> AppSettings {
    autostart::apply_autostart(enabled);
    update_settings(&app, |s| s.autostart_enabled = enabled)
}

#[tauri::command]
#[specta::specta]
pub fn set_show_tray_icon(app: AppHandle, enabled: bool) -> AppSettings {
    tray::set_tray_visibility(&app, enabled);
    update_settings(&app, |s| s.show_tray_icon = enabled)
}

#[tauri::command]
#[specta::specta]
pub fn set_remove_filler_words(app: AppHandle, enabled: bool) -> AppSettings {
    update_settings(&app, |s| s.remove_filler_words = enabled)
}

#[tauri::command]
#[specta::specta]
pub fn set_app_language(app: AppHandle, language: AppLanguage) -> AppSettings {
    let settings = update_settings(&app, |s| s.app_language = language);
    tray::sync_tray(&app);
    settings
}

/// Clean and Reformat can only be turned on once a Groq key is saved.
#[tauri::command]
#[specta::specta]
pub fn set_clean_and_reformat(app: AppHandle, enabled: bool) -> Result<AppSettings, String> {
    if enabled && !keychain::has_groq_api_key() {
        return Err("no_api_key".into());
    }
    Ok(update_settings(&app, |s| s.clean_and_reformat = enabled))
}

#[tauri::command]
#[specta::specta]
pub fn set_custom_words(app: AppHandle, words: Vec<String>) -> AppSettings {
    let words = normalize_custom_words(words);
    update_settings(&app, |s| s.custom_words = words)
}

#[tauri::command]
#[specta::specta]
pub fn set_replacements(app: AppHandle, replacements: Vec<Replacement>) -> AppSettings {
    let replacements = normalize_replacements(replacements);
    update_settings(&app, |s| s.replacements = replacements)
}

#[tauri::command]
#[specta::specta]
pub fn set_groq_api_key(key: String) -> Result<(), String> {
    keychain::set_groq_api_key(&key)
}

#[tauri::command]
#[specta::specta]
pub fn has_groq_api_key() -> bool {
    keychain::has_groq_api_key()
}

/// Removing the key also turns Clean and Reformat off.
#[tauri::command]
#[specta::specta]
pub fn clear_groq_api_key(app: AppHandle) -> Result<AppSettings, String> {
    keychain::clear_groq_api_key()?;
    Ok(update_settings(&app, |s| s.clean_and_reformat = false))
}
