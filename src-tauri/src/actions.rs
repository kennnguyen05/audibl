//! Dictation actions: start, finish, cancel. Filled in by the dictation
//! milestone.

use tauri::AppHandle;

pub fn cancel_current_operation(_app: &AppHandle) {
    log::info!("Cancel requested");
}
