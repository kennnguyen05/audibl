//! Launch on Startup via `SMAppService` (macOS 13+). The login item is
//! attributed to the app bundle, so it shows as "Audible" in System Settings.
//! Registration fails in `tauri dev` (no signed bundle); that is logged, not
//! surfaced.

use objc2_service_management::{SMAppService, SMAppServiceStatus};

pub fn apply_autostart(enabled: bool) {
    let service = unsafe { SMAppService::mainAppService() };
    let status = unsafe { service.status() };

    if enabled {
        if status == SMAppServiceStatus::Enabled {
            return;
        }
        match unsafe { service.registerAndReturnError() } {
            Ok(()) => log::info!("Registered login item"),
            Err(e) => log::warn!("Failed to register login item: {e}"),
        }
    } else {
        if status == SMAppServiceStatus::NotRegistered || status == SMAppServiceStatus::NotFound {
            return;
        }
        match unsafe { service.unregisterAndReturnError() } {
            Ok(()) => log::info!("Unregistered login item"),
            Err(e) => log::warn!("Failed to unregister login item: {e}"),
        }
    }
}
