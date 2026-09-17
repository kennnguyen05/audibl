//! Dictation state machine: Hold / Toggle, Finish (Return), Cancel (Esc).
//!
//! `CoordinatorState::on_input` is pure and unit-tested. `Coordinator` runs it
//! on one thread that owns the state, so key events from any thread are
//! handled in order and effects never race each other. Pattern ported from
//! Handy (`transcription_coordinator.rs`, MIT), without hold-or-toggle,
//! external triggers, or busy-input replay.

use crate::settings::ActivationMode;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::{Duration, Instant};
use tauri::AppHandle;

/// Presses closer together than this are key bounce, not a new press.
const DEBOUNCE: Duration = Duration::from_millis(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    ShortcutPressed,
    ShortcutReleased,
    /// Return while a Toggle recording is active.
    Finish,
    /// Esc while recording or processing, or the tray Cancel item.
    Cancel,
    /// The recorder could not start.
    StartFailed,
    /// The pipeline for session `u64` finished (pasted, empty, or failed).
    ProcessingDone(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Open the mic and show the overlay. `mode` decides whether Return is armed.
    StartRecording { session: u64, mode: ActivationMode },
    /// Stop the mic, transcribe, and paste.
    FinishRecording { session: u64 },
    CancelRecording { session: u64 },
    CancelProcessing { session: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Idle,
    /// The mode is fixed when recording starts, so changing the setting
    /// mid-recording cannot strand a session.
    Recording { session: u64, mode: ActivationMode },
    Processing { session: u64 },
}

#[derive(Debug)]
pub struct CoordinatorState {
    pub stage: Stage,
    next_session: u64,
    last_press: Option<Instant>,
}

impl Default for CoordinatorState {
    fn default() -> Self {
        Self {
            stage: Stage::Idle,
            next_session: 1,
            last_press: None,
        }
    }
}

impl CoordinatorState {
    pub fn on_input(&mut self, input: Input, mode: ActivationMode, now: Instant) -> Option<Effect> {
        if input == Input::ShortcutPressed {
            let bounced = self
                .last_press
                .is_some_and(|last| now.duration_since(last) < DEBOUNCE);
            self.last_press = Some(now);
            if bounced {
                return None;
            }
        }

        match (self.stage, input) {
            (Stage::Idle, Input::ShortcutPressed) => {
                let session = self.next_session;
                self.next_session += 1;
                self.stage = Stage::Recording { session, mode };
                Some(Effect::StartRecording { session, mode })
            }
            (Stage::Idle, _) => None,

            (Stage::Recording { session, mode }, input) => match (mode, input) {
                (ActivationMode::Hold, Input::ShortcutReleased)
                | (ActivationMode::Toggle, Input::ShortcutPressed)
                | (ActivationMode::Toggle, Input::Finish) => {
                    self.stage = Stage::Processing { session };
                    Some(Effect::FinishRecording { session })
                }
                (_, Input::Cancel) => {
                    self.stage = Stage::Idle;
                    Some(Effect::CancelRecording { session })
                }
                (_, Input::StartFailed) => {
                    self.stage = Stage::Idle;
                    None
                }
                // Hold: repeated presses while held. Toggle: key releases.
                // Finish is never armed in Hold mode.
                _ => None,
            },

            (Stage::Processing { session }, Input::Cancel) => {
                self.stage = Stage::Idle;
                Some(Effect::CancelProcessing { session })
            }
            (Stage::Processing { session }, Input::ProcessingDone(done)) if done == session => {
                self.stage = Stage::Idle;
                None
            }
            // Shortcut presses are ignored until the paste is done.
            (Stage::Processing { .. }, _) => None,
        }
    }
}

/// Runs the state machine on its own thread.
pub struct Coordinator {
    tx: Sender<Input>,
}

impl Coordinator {
    pub fn new(app: AppHandle) -> Self {
        let (tx, rx) = mpsc::channel::<Input>();
        thread::spawn(move || {
            let mut state = CoordinatorState::default();
            while let Ok(input) = rx.recv() {
                let mode = crate::settings::get_settings(&app).activation_mode;
                let before = state.stage;
                if let Some(effect) = state.on_input(input, mode, Instant::now()) {
                    log::debug!("coordinator: {before:?} + {input:?} -> {effect:?}");
                    crate::actions::run_effect(&app, effect);
                }
            }
        });
        Self { tx }
    }

    pub fn send(&self, input: Input) {
        let _ = self.tx.send(input);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ActivationMode::{Hold, Toggle};

    struct Driver {
        state: CoordinatorState,
        now: Instant,
    }

    impl Driver {
        fn new() -> Self {
            Self {
                state: CoordinatorState::default(),
                now: Instant::now(),
            }
        }

        fn send(&mut self, input: Input, mode: ActivationMode) -> Option<Effect> {
            self.now += Duration::from_millis(200);
            self.state.on_input(input, mode, self.now)
        }
    }

    #[test]
    fn hold_press_records_release_finishes() {
        let mut d = Driver::new();
        assert_eq!(
            d.send(Input::ShortcutPressed, Hold),
            Some(Effect::StartRecording { session: 1, mode: Hold })
        );
        assert_eq!(
            d.send(Input::ShortcutReleased, Hold),
            Some(Effect::FinishRecording { session: 1 })
        );
        assert_eq!(d.state.stage, Stage::Processing { session: 1 });
        assert_eq!(d.send(Input::ProcessingDone(1), Hold), None);
        assert_eq!(d.state.stage, Stage::Idle);
    }

    #[test]
    fn hold_esc_cancels_recording() {
        let mut d = Driver::new();
        d.send(Input::ShortcutPressed, Hold);
        assert_eq!(
            d.send(Input::Cancel, Hold),
            Some(Effect::CancelRecording { session: 1 })
        );
        assert_eq!(d.state.stage, Stage::Idle);
        // The release after Esc does nothing.
        assert_eq!(d.send(Input::ShortcutReleased, Hold), None);
    }

    #[test]
    fn hold_ignores_repeat_presses_and_return() {
        let mut d = Driver::new();
        d.send(Input::ShortcutPressed, Hold);
        assert_eq!(d.send(Input::ShortcutPressed, Hold), None);
        assert_eq!(d.send(Input::Finish, Hold), None);
        assert!(matches!(d.state.stage, Stage::Recording { .. }));
    }

    #[test]
    fn toggle_press_records_and_release_is_ignored() {
        let mut d = Driver::new();
        assert_eq!(
            d.send(Input::ShortcutPressed, Toggle),
            Some(Effect::StartRecording { session: 1, mode: Toggle })
        );
        assert_eq!(d.send(Input::ShortcutReleased, Toggle), None);
        assert!(matches!(d.state.stage, Stage::Recording { .. }));
    }

    #[test]
    fn toggle_return_finishes() {
        let mut d = Driver::new();
        d.send(Input::ShortcutPressed, Toggle);
        assert_eq!(
            d.send(Input::Finish, Toggle),
            Some(Effect::FinishRecording { session: 1 })
        );
    }

    #[test]
    fn toggle_second_press_finishes() {
        let mut d = Driver::new();
        d.send(Input::ShortcutPressed, Toggle);
        d.send(Input::ShortcutReleased, Toggle);
        assert_eq!(
            d.send(Input::ShortcutPressed, Toggle),
            Some(Effect::FinishRecording { session: 1 })
        );
    }

    #[test]
    fn toggle_esc_cancels() {
        let mut d = Driver::new();
        d.send(Input::ShortcutPressed, Toggle);
        assert_eq!(
            d.send(Input::Cancel, Toggle),
            Some(Effect::CancelRecording { session: 1 })
        );
    }

    #[test]
    fn mode_is_fixed_when_recording_starts() {
        let mut d = Driver::new();
        d.send(Input::ShortcutPressed, Toggle);
        // Setting switched to Hold mid-recording: the release still does nothing.
        assert_eq!(d.send(Input::ShortcutReleased, Hold), None);
        assert_eq!(
            d.send(Input::Finish, Hold),
            Some(Effect::FinishRecording { session: 1 })
        );
    }

    #[test]
    fn processing_ignores_shortcut_and_return() {
        let mut d = Driver::new();
        d.send(Input::ShortcutPressed, Hold);
        d.send(Input::ShortcutReleased, Hold);
        assert_eq!(d.send(Input::ShortcutPressed, Hold), None);
        assert_eq!(d.send(Input::ShortcutReleased, Hold), None);
        assert_eq!(d.send(Input::Finish, Toggle), None);
        assert_eq!(d.state.stage, Stage::Processing { session: 1 });
    }

    #[test]
    fn processing_esc_cancels() {
        let mut d = Driver::new();
        d.send(Input::ShortcutPressed, Hold);
        d.send(Input::ShortcutReleased, Hold);
        assert_eq!(
            d.send(Input::Cancel, Hold),
            Some(Effect::CancelProcessing { session: 1 })
        );
        assert_eq!(d.state.stage, Stage::Idle);
    }

    #[test]
    fn stale_processing_done_does_not_end_newer_session() {
        let mut d = Driver::new();
        d.send(Input::ShortcutPressed, Hold);
        d.send(Input::ShortcutReleased, Hold);
        d.send(Input::Cancel, Hold); // cancel session 1 while processing
        d.send(Input::ShortcutPressed, Hold);
        d.send(Input::ShortcutReleased, Hold); // session 2 processing
        assert_eq!(d.send(Input::ProcessingDone(1), Hold), None);
        assert_eq!(d.state.stage, Stage::Processing { session: 2 });
        d.send(Input::ProcessingDone(2), Hold);
        assert_eq!(d.state.stage, Stage::Idle);
    }

    #[test]
    fn start_failure_returns_to_idle() {
        let mut d = Driver::new();
        d.send(Input::ShortcutPressed, Hold);
        assert_eq!(d.send(Input::StartFailed, Hold), None);
        assert_eq!(d.state.stage, Stage::Idle);
    }

    #[test]
    fn idle_ignores_everything_but_press() {
        let mut d = Driver::new();
        for input in [
            Input::ShortcutReleased,
            Input::Finish,
            Input::Cancel,
            Input::StartFailed,
            Input::ProcessingDone(1),
        ] {
            assert_eq!(d.send(input, Hold), None);
            assert_eq!(d.state.stage, Stage::Idle);
        }
    }

    #[test]
    fn bounced_press_is_dropped() {
        let mut state = CoordinatorState::default();
        let now = Instant::now();
        state.on_input(Input::ShortcutPressed, Toggle, now);
        assert_eq!(
            state.on_input(Input::ShortcutPressed, Toggle, now + Duration::from_millis(10)),
            None
        );
        assert!(matches!(state.stage, Stage::Recording { .. }));
    }
}
