//! Dictation history. Filled in by the text pipeline milestone.

use tauri::AppHandle;

pub fn copy_last_transcript(_app: &AppHandle) {
    log::info!("Copy last transcript requested");
}
