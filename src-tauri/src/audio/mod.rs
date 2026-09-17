//! Audio capture for dictation. The microphone opens on shortcut press and
//! closes when the recording ends, so the macOS mic indicator is only on
//! while Audible is actually listening.

pub mod devices;
pub mod mute;
pub mod recorder;
pub mod resampler;
pub mod vad;
pub mod visualizer;

use crate::settings::get_settings;
use recorder::AudioRecorder;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use vad::{SileroVad, SmoothedVad, SAMPLE_RATE, SILERO_THRESHOLD};

pub const OVERLAY_LABEL: &str = "recording_overlay";
const LEVEL_EMIT_INTERVAL_MS: u64 = 33;

pub struct AudioManager {
    app: AppHandle,
    recorder: Mutex<Option<AudioRecorder>>,
    recording: AtomicBool,
    mute: Mutex<mute::MuteGuard>,
}

impl AudioManager {
    pub fn new(app: &AppHandle) -> Self {
        Self {
            app: app.clone(),
            recorder: Mutex::new(None),
            recording: AtomicBool::new(false),
            mute: Mutex::new(mute::MuteGuard::default()),
        }
    }

    fn create_recorder(&self) -> anyhow::Result<AudioRecorder> {
        let vad_path = self
            .app
            .path()
            .resolve(
                "resources/silero_vad_v4.onnx",
                tauri::path::BaseDirectory::Resource,
            )
            .map_err(|e| anyhow::anyhow!("Failed to resolve VAD path: {e}"))?;
        let silero = SileroVad::new(vad_path, SILERO_THRESHOLD)?;
        let vad = SmoothedVad::with_default_timing(Box::new(silero));
        let app = self.app.clone();
        let last_emit = AtomicU64::new(0);
        Ok(AudioRecorder::new(Box::new(vad)).with_level_callback(move |levels| {
            // ~30 FPS is plenty for the overlay waveform.
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            if now.saturating_sub(last_emit.load(Ordering::Relaxed)) >= LEVEL_EMIT_INTERVAL_MS {
                last_emit.store(now, Ordering::Relaxed);
                let _ = app.emit_to(OVERLAY_LABEL, "mic-level", levels);
            }
        }))
    }

    pub fn start_recording(&self) -> Result<(), String> {
        if self.recording.load(Ordering::SeqCst) {
            return Err("already_recording".into());
        }
        let settings = get_settings(&self.app);
        let mut guard = self.recorder.lock().unwrap();
        if guard.is_none() {
            *guard = Some(self.create_recorder().map_err(|e| e.to_string())?);
        }
        let recorder = guard.as_mut().unwrap();
        recorder.set_selected_channel(settings.selected_channel);

        // A selected microphone that is gone falls back to the system default.
        let device = devices::find_input_device(settings.selected_microphone.as_deref())
            .or_else(|| devices::find_input_device(None));
        recorder.open(device).map_err(|e| e.to_string())?;
        recorder.start().map_err(|e| e.to_string())?;
        self.recording.store(true, Ordering::SeqCst);
        drop(guard);

        if settings.mute_while_recording {
            self.mute.lock().unwrap().apply();
        }
        log::info!("Recording started");
        Ok(())
    }

    /// Stops capture and returns the VAD-filtered 16 kHz samples.
    pub fn stop_recording(&self) -> Vec<f32> {
        if !self.recording.swap(false, Ordering::SeqCst) {
            return Vec::new();
        }
        let samples = {
            let mut guard = self.recorder.lock().unwrap();
            let samples = guard
                .as_ref()
                .and_then(|r| r.stop().inspect_err(|e| log::error!("stop failed: {e}")).ok())
                .unwrap_or_default();
            if let Some(r) = guard.as_mut() {
                r.close();
            }
            samples
        };
        self.mute.lock().unwrap().restore();
        log::info!(
            "Recording stopped: {:.2}s of speech",
            samples.len() as f32 / SAMPLE_RATE as f32
        );
        pad_short_audio(samples)
    }

    pub fn cancel_recording(&self) {
        let _ = self.stop_recording();
    }

    pub fn is_recording(&self) -> bool {
        self.recording.load(Ordering::SeqCst)
    }
}

/// Speech shorter than one second is padded to 1.25 s with silence; very
/// short buffers make ASR models hallucinate or return nothing.
fn pad_short_audio(mut samples: Vec<f32>) -> Vec<f32> {
    let one_second = SAMPLE_RATE as usize;
    if !samples.is_empty() && samples.len() < one_second {
        samples.resize(one_second * 5 / 4, 0.0);
    }
    samples
}

#[cfg(test)]
mod tests {
    use super::pad_short_audio;

    #[test]
    fn pads_only_short_nonempty_audio() {
        assert!(pad_short_audio(vec![]).is_empty());
        assert_eq!(pad_short_audio(vec![0.1; 100]).len(), 20_000);
        assert_eq!(pad_short_audio(vec![0.1; 32_000]).len(), 32_000);
    }
}
