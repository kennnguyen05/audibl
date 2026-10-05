//! Mute While Recording: system output mute. Ported from Handy
//! (`managers/audio.rs`, MIT), which shells out to AppleScript; Audibl sets
//! the default output device's mute through CoreAudio instead, which takes
//! effect at once rather than after two ~200 ms `osascript` launches.
//! AppleScript stays as the fallback for a device without a mute control.
//! The previous mute state is restored, so audio that was already muted stays
//! muted.

use objc2_core_audio::{
    kAudioDevicePropertyMute, kAudioHardwarePropertyDefaultOutputDevice,
    kAudioObjectPropertyElementMain, kAudioObjectPropertyScopeGlobal,
    kAudioObjectPropertyScopeOutput, kAudioObjectSystemObject, AudioObjectGetPropertyData,
    AudioObjectID, AudioObjectPropertyAddress, AudioObjectPropertyScope,
    AudioObjectPropertySelector, AudioObjectSetPropertyData,
};
use std::ffi::c_void;
use std::process::Command;
use std::ptr::{self, NonNull};

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

fn address(
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: scope,
        mElement: kAudioObjectPropertyElementMain,
    }
}

/// Reads a `u32`-sized property (device ids and the mute flag both are).
fn get_u32(object: AudioObjectID, mut addr: AudioObjectPropertyAddress) -> Option<u32> {
    let mut value: u32 = 0;
    let mut size = size_of::<u32>() as u32;
    // SAFETY: every pointer refers to a live local of the size passed.
    let status = unsafe {
        AudioObjectGetPropertyData(
            object,
            NonNull::from(&mut addr),
            0,
            ptr::null(),
            NonNull::from(&mut size),
            NonNull::from(&mut value).cast::<c_void>(),
        )
    };
    (status == 0).then_some(value)
}

fn default_output_device() -> Option<AudioObjectID> {
    let addr = address(
        kAudioHardwarePropertyDefaultOutputDevice,
        kAudioObjectPropertyScopeGlobal,
    );
    get_u32(kAudioObjectSystemObject as AudioObjectID, addr).filter(|&id| id != 0)
}

fn device_muted(device: AudioObjectID) -> Option<bool> {
    let addr = address(kAudioDevicePropertyMute, kAudioObjectPropertyScopeOutput);
    get_u32(device, addr).map(|v| v != 0)
}

fn set_device_muted(device: AudioObjectID, mute: bool) -> bool {
    let mut addr = address(kAudioDevicePropertyMute, kAudioObjectPropertyScopeOutput);
    let mut value = u32::from(mute);
    // SAFETY: every pointer refers to a live local of the size passed.
    let status = unsafe {
        AudioObjectSetPropertyData(
            device,
            NonNull::from(&mut addr),
            0,
            ptr::null(),
            size_of::<u32>() as u32,
            NonNull::from(&mut value).cast::<c_void>(),
        )
    };
    status == 0
}

#[derive(Default)]
pub struct MuteGuard {
    did_mute: bool,
    prev_muted: Option<bool>,
    /// The device muted through CoreAudio; `None` means AppleScript did it.
    device: Option<AudioObjectID>,
}

impl MuteGuard {
    pub fn apply(&mut self) {
        if self.did_mute {
            return;
        }
        let device = default_output_device()
            .and_then(|d| device_muted(d).map(|muted| (d, muted)))
            .filter(|&(d, _)| set_device_muted(d, true));
        match device {
            Some((d, muted)) => {
                self.device = Some(d);
                self.prev_muted = Some(muted);
            }
            None => {
                self.device = None;
                self.prev_muted = get_mute();
                set_mute(true);
            }
        }
        self.did_mute = true;
    }

    /// Unmutes unless the system was already muted before recording. An
    /// unknown prior state unmutes, so audio is never left muted by us. The
    /// device muted is the one unmuted, even if the default output changed.
    pub fn restore(&mut self) {
        if !self.did_mute {
            return;
        }
        if self.prev_muted != Some(true) {
            match self.device {
                Some(d) => {
                    set_device_muted(d, false);
                }
                None => set_mute(false),
            }
        }
        self.did_mute = false;
    }
}
