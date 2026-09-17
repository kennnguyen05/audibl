//! Secure Input detection. While any process holds macOS Secure Event Input
//! (password fields, Terminal's "Secure Keyboard Entry"), CGEventTaps stop
//! receiving key presses, so the Transcribe Shortcut silently stops working.
//! Audible polls `IsSecureEventInputEnabled()` and, once it has been held for
//! a few seconds, shows a warning at the top of the menu bar menu.
//!
//! Detection is ported from Handy (`secure_input.rs`, MIT). Handy's Carbon
//! re-registration fallback is not ported in v1.

use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

const POLL_INTERVAL: Duration = Duration::from_secs(1);
/// A password field gaining focus for a moment is normal; only warn when held.
const SUSTAIN_THRESHOLD: Duration = Duration::from_secs(3);

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn IsSecureEventInputEnabled() -> u8;
}

pub fn is_enabled() -> bool {
    unsafe { IsSecureEventInputEnabled() != 0 }
}

/// Decides when the warning turns on (held ≥ threshold) and off (released).
#[derive(Default)]
pub struct SustainTracker {
    enabled_since: Option<Instant>,
    warning: bool,
}

impl SustainTracker {
    /// Returns the new warning state when it changes.
    pub fn update(&mut self, enabled: bool, now: Instant) -> Option<bool> {
        let next = if enabled {
            let since = *self.enabled_since.get_or_insert(now);
            now.duration_since(since) >= SUSTAIN_THRESHOLD
        } else {
            self.enabled_since = None;
            false
        };
        (next != self.warning).then(|| {
            self.warning = next;
            next
        })
    }
}

pub fn start_monitor(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let mut tracker = SustainTracker::default();
        loop {
            std::thread::sleep(POLL_INTERVAL);
            // The global-shortcut fallback is not affected by Secure Input.
            let tap_active = app
                .try_state::<crate::shortcut::ShortcutManager>()
                .is_some_and(|m| m.is_event_tap());
            let enabled = tap_active && is_enabled();
            if let Some(warning) = tracker.update(enabled, Instant::now()) {
                if warning {
                    log::warn!("Secure Input is held; keyboard shortcuts are blocked");
                } else {
                    log::info!("Secure Input released");
                }
                crate::tray::set_secure_input_warning(&app, warning);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn momentary_secure_input_does_not_warn() {
        let mut tracker = SustainTracker::default();
        let t0 = Instant::now();
        assert_eq!(tracker.update(true, t0), None);
        assert_eq!(tracker.update(true, t0 + Duration::from_secs(1)), None);
        assert_eq!(tracker.update(false, t0 + Duration::from_secs(2)), None);
    }

    #[test]
    fn sustained_secure_input_warns_until_released() {
        let mut tracker = SustainTracker::default();
        let t0 = Instant::now();
        tracker.update(true, t0);
        assert_eq!(
            tracker.update(true, t0 + Duration::from_secs(3)),
            Some(true)
        );
        assert_eq!(tracker.update(true, t0 + Duration::from_secs(4)), None);
        assert_eq!(
            tracker.update(false, t0 + Duration::from_secs(5)),
            Some(false)
        );
    }
}
