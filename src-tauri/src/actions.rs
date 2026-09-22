//! Coordinator effects: start, finish, and cancel a dictation, plus the
//! pipeline that runs after recording (transcribe → paste).

use crate::audio::AudioManager;
use crate::coordinator::{Coordinator, Effect, Input};
use crate::overlay::{self, OverlayState};
use crate::settings::get_settings;
use crate::sfx::{self, Sound};
use crate::shortcut;
use crate::transcription::{TranscribeError, TranscriptionManager};
use crate::tray::{self, TrayIconState};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// Session id of the dictation in progress; 0 when idle. A pipeline only
/// pastes or tears down UI while its session is still the active one, so a
/// cancelled session can never touch a newer one.
static ACTIVE_SESSION: AtomicU64 = AtomicU64::new(0);

fn is_active(session: u64) -> bool {
    ACTIVE_SESSION.load(Ordering::SeqCst) == session
}

/// Clears the active session if it is still `session`; true when it was.
fn end_session(session: u64) -> bool {
    ACTIVE_SESSION
        .compare_exchange(session, 0, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
}

fn send(app: &AppHandle, input: Input) {
    if let Some(coordinator) = app.try_state::<Coordinator>() {
        coordinator.send(input);
    }
}

pub fn run_effect(app: &AppHandle, effect: Effect) {
    match effect {
        Effect::StartRecording { session, .. } => start(app, session),
        Effect::FinishRecording { session } => finish(app, session),
        Effect::CancelRecording { session } => cancel_recording(app, session),
        Effect::CancelProcessing { session } => cancel_processing(app, session),
    }
}

fn start(app: &AppHandle, session: u64) {
    // No model (deleted, or the model constant changed): open the download
    // screen instead of recording.
    if !crate::is_setup_complete(app) {
        log::warn!("Shortcut pressed before setup is complete; opening Audibl");
        crate::show_main_window(app);
        send(app, Input::StartFailed);
        return;
    }

    ACTIVE_SESSION.store(session, Ordering::SeqCst);
    crate::context::capture(app);

    if let Err(e) = app.state::<AudioManager>().start_recording() {
        log::error!("Could not start recording: {e}");
        end_session(session);
        send(app, Input::StartFailed);
        return;
    }
    sfx::play(app, Sound::On);

    // Warm the model while the user speaks.
    app.state::<Arc<TranscriptionManager>>().preload();

    shortcut::arm_cancel(app);
    overlay::show(app, OverlayState::Recording);
    tray::set_tray_state(app, TrayIconState::Recording);
}

fn finish(app: &AppHandle, session: u64) {
    let samples = app.state::<AudioManager>().stop_recording();
    sfx::play(app, Sound::Off);
    overlay::show(app, OverlayState::Transcribing);
    tray::set_tray_state(app, TrayIconState::Processing);

    let app = app.clone();
    std::thread::spawn(move || {
        run_pipeline(&app, session, samples);
        if end_session(session) {
            teardown(&app);
        }
        send(&app, Input::ProcessingDone(session));
    });
}

fn run_pipeline(app: &AppHandle, session: u64, samples: Vec<f32>) {
    if samples.is_empty() {
        log::info!("No speech detected; nothing to paste");
        return;
    }
    let settings = get_settings(app);
    let transcription = match app
        .state::<Arc<TranscriptionManager>>()
        .transcribe(&samples, &settings.custom_words)
    {
        Ok(t) => t,
        Err(TranscribeError::Cancelled) => return,
        Err(TranscribeError::ModelMissing) => {
            crate::show_main_window(app);
            return;
        }
        Err(e) => {
            log::error!("Transcription failed: {e}");
            return;
        }
    };

    let Some(output) = crate::pipeline::process(app, session, &transcription) else {
        return;
    };
    if output.final_text.trim().is_empty() || !is_active(session) {
        return;
    }

    // Esc must not reach the target app's handling of the paste.
    shortcut::disarm_cancel(app);
    match crate::paste::paste(app, output.final_text.clone()) {
        Ok(()) => log::info!("Pasted {} characters", output.final_text.chars().count()),
        Err(e) => log::error!("Paste failed: {e}"),
    }
    crate::history::add(app, output);
}

fn teardown(app: &AppHandle) {
    shortcut::disarm_cancel(app);
    overlay::hide(app);
    tray::set_tray_state(app, TrayIconState::Idle);
}

fn cancel_recording(app: &AppHandle, session: u64) {
    log::info!("Recording cancelled");
    app.state::<AudioManager>().cancel_recording();
    sfx::play(app, Sound::Off);
    end_session(session);
    teardown(app);
}

fn cancel_processing(app: &AppHandle, session: u64) {
    log::info!("Processing cancelled");
    end_session(session);
    app.state::<Arc<TranscriptionManager>>().cancel();
    teardown(app);
}

/// `pipeline` checks this between steps (e.g. before and after Groq).
pub fn is_session_active(session: u64) -> bool {
    is_active(session)
}

/// Tray Cancel item: same as pressing Esc.
pub fn cancel_current_operation(app: &AppHandle) {
    send(app, Input::Cancel);
}
