//! Synchronous permission checks for the backend. The frontend uses the
//! same plugin through `tauri-plugin-macos-permissions-api`.

use tauri_plugin_macos_permissions::{check_accessibility_permission, check_microphone_permission};

/// Needed for the global shortcut event tap and for pasting.
pub fn has_accessibility() -> bool {
    tauri::async_runtime::block_on(check_accessibility_permission())
}

pub fn has_microphone() -> bool {
    tauri::async_runtime::block_on(check_microphone_permission())
}
