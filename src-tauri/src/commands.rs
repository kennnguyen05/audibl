//! Tauri commands called by the frontend. All are exported to
//! `src/bindings.ts` through tauri-specta.

use crate::settings::{get_settings, AppSettings};
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
