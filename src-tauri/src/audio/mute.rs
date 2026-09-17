//! Mute While Recording: system output mute via AppleScript. Ported from
//! Handy (`managers/audio.rs`, MIT). The previous mute state is restored, so
//! audio that was already muted stays muted.

use std::process::Command;

fn set_mute(mute: bool) {
    let script = format!("set volume output muted {mute}");
    let _ = Command::new("osascript").args(["-e", &script]).output();
}

fn get_mute() -> Option<bool> {
    let out = Command::new("osascript")
        .args(["-e", "output muted of (get volume settings)"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    match String::from_utf8_lossy(&out.stdout).trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[derive(Default)]
pub struct MuteGuard {
    did_mute: bool,
    prev_muted: Option<bool>,
}

impl MuteGuard {
    pub fn apply(&mut self) {
        if self.did_mute {
            return;
        }
        self.prev_muted = get_mute();
        set_mute(true);
        self.did_mute = true;
    }

    /// Unmutes unless the system was already muted before recording. An
    /// unknown prior state unmutes, so audio is never left muted by us.
    pub fn restore(&mut self) {
        if !self.did_mute {
            return;
        }
        if self.prev_muted != Some(true) {
            set_mute(false);
        }
        self.did_mute = false;
    }
}
