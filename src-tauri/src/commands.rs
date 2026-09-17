//! Tauri commands called by the frontend. All are exported to
//! `src/bindings.ts` through tauri-specta. Setting commands return the full
//! updated settings so the frontend store stays in sync.

use crate::model::{ModelManager, ModelStatus};
use crate::settings::{
    get_settings, normalize_custom_words, normalize_replacements, update_settings, ActivationMode,
    AppLanguage, AppSettings, Replacement,
};
use crate::{audio, autostart, keychain, tray};
use tauri::{AppHandle, Manager};

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

/// Microphone names; cpal enumeration can stall, so it runs off the main thread.
#[tauri::command]
#[specta::specta]
pub async fn get_microphones() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        audio::devices::list_input_devices()
            .map(|devices| devices.into_iter().map(|d| d.name).collect())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// `None` follows the system default. Changing the microphone resets the channel.
#[tauri::command]
#[specta::specta]
pub fn set_microphone(app: AppHandle, name: Option<String>) -> AppSettings {
    update_settings(&app, |s| {
        s.selected_microphone = name;
        s.selected_channel = None;
    })
}

/// Input channel count of the selected (or default) microphone.
#[tauri::command]
#[specta::specta]
pub async fn get_channel_count(app: AppHandle) -> Result<u16, String> {
    let name = get_settings(&app).selected_microphone;
    tauri::async_runtime::spawn_blocking(move || {
        match audio::devices::find_input_device(name.as_deref()) {
            Some(device) => {
                audio::recorder::input_channel_count(&device).map_err(|e| e.to_string())
            }
            None => Ok(1),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
#[specta::specta]
pub fn set_channel(app: AppHandle, channel: Option<u16>) -> AppSettings {
    update_settings(&app, |s| s.selected_channel = channel)
}

#[tauri::command]
#[specta::specta]
pub fn get_model_status(app: AppHandle) -> ModelStatus {
    app.state::<ModelManager>().status(&app)
}

#[tauri::command]
#[specta::specta]
pub fn start_model_download(app: AppHandle) {
    app.state::<ModelManager>().start_download(&app);
}

#[tauri::command]
#[specta::specta]
pub fn cancel_model_download(app: AppHandle) {
    app.state::<ModelManager>().cancel_download();
}

/// Finishes onboarding once the model is on disk.
#[tauri::command]
#[specta::specta]
pub fn complete_onboarding(app: AppHandle) -> Result<AppSettings, String> {
    if !app.state::<ModelManager>().is_ready(&app) {
        return Err("model_missing".into());
    }
    let settings = update_settings(&app, |s| s.onboarding_complete = true);
    crate::on_ready(&app);
    Ok(settings)
}

/// Validates and registers a new Transcribe Shortcut, then persists it.
#[tauri::command]
#[specta::specta]
pub fn change_shortcut(app: AppHandle, shortcut: String) -> Result<AppSettings, String> {
    crate::shortcut::change_shortcut(&app, &shortcut)?;
    let shortcut = shortcut.trim().to_string();
    Ok(update_settings(&app, |s| s.shortcut = shortcut))
}

#[tauri::command]
#[specta::specta]
pub fn start_shortcut_capture(app: AppHandle) -> Result<(), String> {
    crate::shortcut::start_capture(&app)
}

#[tauri::command]
#[specta::specta]
pub fn stop_shortcut_capture(app: AppHandle) {
    crate::shortcut::stop_capture(&app);
}

#[tauri::command]
#[specta::specta]
pub fn get_history(app: AppHandle) -> Vec<crate::history::HistoryEntry> {
    crate::history::load(&app)
}

#[tauri::command]
#[specta::specta]
pub fn delete_history_entry(app: AppHandle, timestamp: f64) -> Vec<crate::history::HistoryEntry> {
    crate::history::delete(&app, timestamp)
}

#[tauri::command]
#[specta::specta]
pub fn copy_text(app: AppHandle, text: String) -> Result<(), String> {
    crate::history::copy_text(&app, &text)
}

/// Removing the key also turns Clean and Reformat off.
#[tauri::command]
#[specta::specta]
pub fn clear_groq_api_key(app: AppHandle) -> Result<AppSettings, String> {
    keychain::clear_groq_api_key()?;
    Ok(update_settings(&app, |s| s.clean_and_reformat = false))
}
