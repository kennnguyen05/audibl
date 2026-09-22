//! Dictation state machine: Auto / Hold / Toggle, Cancel (Esc).
//!
//! `CoordinatorState::on_input` is pure and unit-tested. `Coordinator` runs it
//! on one thread that owns the state, so key events from any thread are
//! handled in order and effects never race each other. Pattern ported from
//! Handy (`transcription_coordinator.rs`, MIT), without external triggers or
//! busy-input replay. Handy's release grace timer is not ported either: it
//! exists for X11 auto-repeat, and handy-keys already collapses a held key
//! into one press and one release, so the thread can keep blocking on `recv`.

use crate::settings::ActivationMode;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::{Duration, Instant};
use tauri::AppHandle;

/// Presses closer together than this are key bounce, not a new press.
const DEBOUNCE: Duration = Duration::from_millis(30);

/// Auto mode only: a press held at least this long is hold-to-talk, anything
/// shorter is a tap that locks recording on until the next press.
const HOLD_THRESHOLD: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    ShortcutPressed,
    ShortcutReleased,
    /// Esc while recording or processing, or the tray Cancel item.
    Cancel,
    /// The recorder could not start.
    StartFailed,
    /// The pipeline for session `u64` finished (pasted, empty, or failed).
    ProcessingDone(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Open the mic and show the overlay.
    StartRecording {
        session: u64,
        mode: ActivationMode,
    },
    /// Stop the mic, transcribe, and paste.
    FinishRecording {
        session: u64,
    },
    CancelRecording {
        session: u64,
    },
    CancelProcessing {
        session: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Idle,
    /// The mode is fixed when recording starts, so changing the setting
    /// mid-recording cannot strand a session.
    Recording {
        session: u64,
        mode: ActivationMode,
    },
    Processing {
        session: u64,
    },
}

/// Press bookkeeping for the live recording. Ported from Handy's `Hold`.
#[derive(Debug)]
struct Hold {
    /// When the key that started this recording went down. Repeat presses
    /// never move it, so a hold is always measured from the first press.
    pressed_at: Instant,
    /// Recording outlives the key: the next press stops it and releases are
    /// ignored. Set at once in Toggle, and in Auto once a release has been
    /// classified as a tap. Never set in Hold.
    locked: bool,
}

#[derive(Debug)]
pub struct CoordinatorState {
    pub stage: Stage,
    next_session: u64,
    last_press: Option<Instant>,
    /// `Some` exactly while `stage` is `Recording`.
    hold: Option<Hold>,
}

impl Default for CoordinatorState {
    fn default() -> Self {
        Self {
            stage: Stage::Idle,
            next_session: 1,
            last_press: None,
            hold: None,
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
                self.hold = Some(Hold {
                    pressed_at: now,
                    locked: mode == ActivationMode::Toggle,
                });
                Some(Effect::StartRecording { session, mode })
            }
            (Stage::Idle, _) => None,

            (Stage::Recording { session, mode }, input) => match (mode, input) {
                // A locked session is stopped by the next press; in Hold the
                // key release stops it.
                (_, Input::ShortcutPressed) if self.is_locked() => self.finish(session),
                (ActivationMode::Hold, Input::ShortcutReleased) => self.finish(session),
                (ActivationMode::Auto, Input::ShortcutReleased) => {
                    let held = self
                        .hold
                        .as_ref()
                        .map(|h| now.saturating_duration_since(h.pressed_at))
                        // No bookkeeping means a tap cannot be told from a
                        // hold; stopping is the safe reading.
                        .unwrap_or(Duration::MAX);
                    if held >= HOLD_THRESHOLD {
                        return self.finish(session);
                    }
                    if let Some(hold) = &mut self.hold {
                        hold.locked = true;
                    }
                    None
                }
                (_, Input::Cancel) => {
                    self.stage = Stage::Idle;
                    self.hold = None;
                    Some(Effect::CancelRecording { session })
                }
                (_, Input::StartFailed) => {
                    self.stage = Stage::Idle;
                    self.hold = None;
                    None
                }
                // Repeated presses while the key is still held, and releases
                // of a locked session.
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

    fn is_locked(&self) -> bool {
        self.hold.as_ref().is_some_and(|h| h.locked)
    }

    fn finish(&mut self, session: u64) -> Option<Effect> {
        self.stage = Stage::Processing { session };
        self.hold = None;
        Some(Effect::FinishRecording { session })
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
    use ActivationMode::{Auto, Hold, Toggle};

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

        /// Advances the clock by 200 ms: clear of DEBOUNCE, and under
        /// HOLD_THRESHOLD, so an Auto release sent this way is a tap.
        fn send(&mut self, input: Input, mode: ActivationMode) -> Option<Effect> {
            self.send_after(Duration::from_millis(200), input, mode)
        }

        fn send_after(
            &mut self,
            elapsed: Duration,
            input: Input,
            mode: ActivationMode,
        ) -> Option<Effect> {
            self.now += elapsed;
            self.state.on_input(input, mode, self.now)
        }
    }

    #[test]
    fn hold_press_records_release_finishes() {
        let mut d = Driver::new();
        assert_eq!(
            d.send(Input::ShortcutPressed, Hold),
            Some(Effect::StartRecording {
                session: 1,
                mode: Hold
            })
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
    fn hold_ignores_repeat_presses() {
        let mut d = Driver::new();
        d.send(Input::ShortcutPressed, Hold);
        assert_eq!(d.send(Input::ShortcutPressed, Hold), None);
        assert!(matches!(d.state.stage, Stage::Recording { .. }));
    }

    #[test]
    fn toggle_press_records_and_release_is_ignored() {
        let mut d = Driver::new();
        assert_eq!(
            d.send(Input::ShortcutPressed, Toggle),
            Some(Effect::StartRecording {
                session: 1,
                mode: Toggle
            })
        );
        assert_eq!(d.send(Input::ShortcutReleased, Toggle), None);
        assert!(matches!(d.state.stage, Stage::Recording { .. }));
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
        // Setting switched to Hold mid-recording: the release still does
        // nothing, and the next press still finishes.
        assert_eq!(d.send(Input::ShortcutReleased, Hold), None);
        assert_eq!(
            d.send(Input::ShortcutPressed, Hold),
            Some(Effect::FinishRecording { session: 1 })
        );
    }

    #[test]
    fn processing_ignores_the_shortcut() {
        let mut d = Driver::new();
        d.send(Input::ShortcutPressed, Hold);
        d.send(Input::ShortcutReleased, Hold);
        assert_eq!(d.send(Input::ShortcutPressed, Hold), None);
        assert_eq!(d.send(Input::ShortcutReleased, Hold), None);
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
            state.on_input(
                Input::ShortcutPressed,
                Toggle,
                now + Duration::from_millis(10)
            ),
            None
        );
        assert!(matches!(state.stage, Stage::Recording { .. }));
    }

    #[test]
    fn auto_long_hold_finishes_on_release() {
        let mut d = Driver::new();
        assert_eq!(
            d.send(Input::ShortcutPressed, Auto),
            Some(Effect::StartRecording {
                session: 1,
                mode: Auto
            })
        );
        assert_eq!(
            d.send_after(HOLD_THRESHOLD, Input::ShortcutReleased, Auto),
            Some(Effect::FinishRecording { session: 1 })
        );
        assert_eq!(d.state.stage, Stage::Processing { session: 1 });
    }

    #[test]
    fn auto_tap_locks_recording_until_next_press() {
        let mut d = Driver::new();
        d.send(Input::ShortcutPressed, Auto);
        // A tap changes nothing outwardly; it locks the session on.
        assert_eq!(d.send(Input::ShortcutReleased, Auto), None);
        assert_eq!(
            d.state.stage,
            Stage::Recording {
                session: 1,
                mode: Auto
            }
        );
        assert_eq!(
            d.send(Input::ShortcutPressed, Auto),
            Some(Effect::FinishRecording { session: 1 })
        );
    }

    #[test]
    fn auto_locked_session_ignores_release() {
        let mut d = Driver::new();
        d.send(Input::ShortcutPressed, Auto);
        d.send(Input::ShortcutReleased, Auto);
        // The press that ends a locked session is followed by its own release.
        d.send(Input::ShortcutPressed, Auto);
        assert_eq!(d.send(Input::ShortcutReleased, Auto), None);
        assert_eq!(d.state.stage, Stage::Processing { session: 1 });
    }

    #[test]
    fn auto_press_while_held_does_not_reset_hold() {
        let mut d = Driver::new();
        d.send(Input::ShortcutPressed, Auto);
        // A stray repeat 200 ms in is ignored and must not move `pressed_at`,
        // so the release 200 ms later is still past the threshold.
        assert_eq!(d.send(Input::ShortcutPressed, Auto), None);
        assert_eq!(
            d.send(Input::ShortcutReleased, Auto),
            Some(Effect::FinishRecording { session: 1 })
        );
    }

    #[test]
    fn auto_cancel_clears_locked_session() {
        let mut d = Driver::new();
        d.send(Input::ShortcutPressed, Auto);
        d.send(Input::ShortcutReleased, Auto);
        assert_eq!(
            d.send(Input::Cancel, Auto),
            Some(Effect::CancelRecording { session: 1 })
        );
        assert_eq!(d.state.stage, Stage::Idle);
        // The next press is a fresh, unlocked session.
        assert_eq!(
            d.send(Input::ShortcutPressed, Auto),
            Some(Effect::StartRecording {
                session: 2,
                mode: Auto
            })
        );
        assert_eq!(
            d.send_after(HOLD_THRESHOLD, Input::ShortcutReleased, Auto),
            Some(Effect::FinishRecording { session: 2 })
        );
    }

    #[test]
    fn auto_mode_is_fixed_when_recording_starts() {
        let mut d = Driver::new();
        d.send(Input::ShortcutPressed, Auto);
        // The setting flips to Hold mid-recording: the tap still locks, so
        // the next press is what finishes.
        assert_eq!(d.send(Input::ShortcutReleased, Hold), None);
        assert_eq!(
            d.send(Input::ShortcutPressed, Hold),
            Some(Effect::FinishRecording { session: 1 })
        );
    }
}
