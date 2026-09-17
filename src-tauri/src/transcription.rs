//! Local transcription: lazy model load, idle unload after 10 minutes,
//! cancellable runs, and the en/vi language guard.
//!
//! Lifecycle pattern ported from Handy (`managers/transcription.rs`, MIT).

use crate::model::{model_path, ModelManager, MODEL};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};
use transcribe_cpp::{
    CancelToken, Model, ModelOptions, RunExtension, RunOptions, Session, WhisperRunOptions,
};

/// Fixed idle unload. No UI; debug builds accept `AUDIBLE_UNLOAD_SECS`.
const UNLOAD_AFTER: Duration = Duration::from_secs(600);

fn unload_after() -> Duration {
    #[cfg(debug_assertions)]
    if let Some(secs) = std::env::var("AUDIBLE_UNLOAD_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
    {
        return Duration::from_secs(secs);
    }
    UNLOAD_AFTER
}

#[derive(Debug, PartialEq, Eq)]
pub enum TranscribeError {
    Cancelled,
    ModelMissing,
    Failed(String),
}

impl std::fmt::Display for TranscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TranscribeError::Cancelled => write!(f, "cancelled"),
            TranscribeError::ModelMissing => write!(f, "model missing"),
            TranscribeError::Failed(e) => write!(f, "{e}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transcription {
    pub text: String,
    /// `en` or `vi`.
    pub language: &'static str,
}

// ---------------------------------------------------------------------------
// Language guard (pure)
// ---------------------------------------------------------------------------

/// Maps a model language code or name to `en`/`vi`; anything else is `None`.
pub fn supported_language(code: Option<&str>) -> Option<&'static str> {
    let code = code?.trim().to_lowercase();
    let primary = code.split(['-', '_']).next().unwrap_or("");
    match primary {
        "en" | "eng" | "english" => Some("en"),
        "vi" | "vie" | "vietnamese" => Some("vi"),
        _ => None,
    }
}

/// After a forced-Vietnamese re-run: keep it only when whatlang reliably
/// reads the text as Vietnamese; otherwise re-run forced to English.
pub fn is_reliably_vietnamese(text: &str) -> bool {
    whatlang::detect(text)
        .is_some_and(|info| info.lang() == whatlang::Lang::Vie && info.is_reliable())
}

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

struct Engine {
    session: Session,
    is_whisper: bool,
}

pub struct TranscriptionManager {
    app: AppHandle,
    engine: Mutex<Option<Engine>>,
    cancel: CancelToken,
    last_activity_ms: AtomicU64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

impl TranscriptionManager {
    pub fn new(app: &AppHandle) -> Arc<Self> {
        let manager = Arc::new(Self {
            app: app.clone(),
            engine: Mutex::new(None),
            cancel: CancelToken::new(),
            last_activity_ms: AtomicU64::new(now_ms()),
        });
        Self::spawn_idle_watcher(Arc::downgrade(&manager));
        manager
    }

    pub fn init_backend() {
        transcribe_cpp::init_logging();
        if let Err(e) = transcribe_cpp::init_backends_default() {
            log::warn!("Failed to initialize transcribe-cpp backends: {e}");
        }
    }

    fn lock_engine(&self) -> MutexGuard<'_, Option<Engine>> {
        self.engine.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn spawn_idle_watcher(manager: std::sync::Weak<Self>) {
        thread::spawn(move || {
            let limit = unload_after();
            let tick = Duration::from_secs(if limit < Duration::from_secs(60) { 1 } else { 10 });
            loop {
                thread::sleep(tick);
                let Some(manager) = manager.upgrade() else {
                    return;
                };
                let recording = manager
                    .app
                    .try_state::<crate::audio::AudioManager>()
                    .is_some_and(|a| a.is_recording());
                if recording {
                    manager.touch();
                    continue;
                }
                let idle_ms = now_ms().saturating_sub(manager.last_activity_ms.load(Ordering::Relaxed));
                if idle_ms > limit.as_millis() as u64 && manager.is_loaded() {
                    // try_lock: a running transcription holds the lock and
                    // is activity, so never block or unload under it.
                    if let Ok(mut engine) = manager.engine.try_lock() {
                        if engine.take().is_some() {
                            log::info!("Model unloaded after {}s idle", idle_ms / 1000);
                        }
                    }
                }
            }
        });
    }

    fn touch(&self) {
        self.last_activity_ms.store(now_ms(), Ordering::Relaxed);
    }

    pub fn is_loaded(&self) -> bool {
        self.lock_engine().is_some()
    }

    /// Loads the model if needed. Safe to call on shortcut press to warm up
    /// while the user speaks.
    pub fn ensure_loaded(&self) -> Result<(), TranscribeError> {
        self.touch();
        let mut engine = self.lock_engine();
        if engine.is_some() {
            return Ok(());
        }
        if !self.app.state::<ModelManager>().is_ready(&self.app) {
            return Err(TranscribeError::ModelMissing);
        }
        let started = Instant::now();
        let model = Model::load_with(model_path(&self.app), &ModelOptions::default())
            .map_err(|e| TranscribeError::Failed(format!("load {}: {e}", MODEL.name)))?;
        let is_whisper = model.arch() == "whisper";
        let mut session = model
            .session()
            .map_err(|e| TranscribeError::Failed(format!("session: {e}")))?;
        session.set_cancel_token(&self.cancel);
        log::info!(
            "Model loaded: {} on {} in {:.2}s",
            MODEL.name,
            model.backend(),
            started.elapsed().as_secs_f32()
        );
        *engine = Some(Engine {
            session,
            is_whisper,
        });
        Ok(())
    }

    /// Kicks off loading on a background thread.
    pub fn preload(self: &Arc<Self>) {
        let manager = Arc::clone(self);
        thread::spawn(move || {
            if let Err(e) = manager.ensure_loaded() {
                log::warn!("Model preload failed: {e}");
            }
        });
    }

    /// Aborts an in-flight run. The next `transcribe` resets the token.
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    pub fn unload(&self) {
        self.lock_engine().take();
    }

    /// Transcribes 16 kHz mono audio with auto language detection, then applies
    /// the guard: a language other than en/vi re-runs forced to `vi`, kept only
    /// if the text reads reliably as Vietnamese, else re-runs forced to `en`.
    pub fn transcribe(
        &self,
        audio: &[f32],
        custom_words: &[String],
    ) -> Result<Transcription, TranscribeError> {
        if audio.is_empty() {
            return Ok(Transcription {
                text: String::new(),
                language: "en",
            });
        }
        self.ensure_loaded()?;
        self.cancel.reset();
        let started = Instant::now();

        let mut guard = self.lock_engine();
        let engine = guard.as_mut().ok_or(TranscribeError::ModelMissing)?;

        let first = run(engine, audio, None, custom_words, &self.cancel)?;
        let result = match supported_language(first.1.as_deref()) {
            Some(language) => Transcription {
                text: first.0,
                language,
            },
            None if first.0.trim().is_empty() => Transcription {
                text: first.0,
                language: "en",
            },
            None => {
                log::info!(
                    "Detected language {:?} is not en/vi; re-running as vi",
                    first.1
                );
                let vi = run(engine, audio, Some("vi"), custom_words, &self.cancel)?;
                if is_reliably_vietnamese(&vi.0) {
                    Transcription {
                        text: vi.0,
                        language: "vi",
                    }
                } else {
                    let en = run(engine, audio, Some("en"), custom_words, &self.cancel)?;
                    Transcription {
                        text: en.0,
                        language: "en",
                    }
                }
            }
        };
        drop(guard);
        self.touch();
        log::info!(
            "Transcribed {:.1}s of audio in {:.2}s (language {})",
            audio.len() as f32 / 16_000.0,
            started.elapsed().as_secs_f32(),
            result.language
        );
        Ok(result)
    }
}

fn run(
    engine: &mut Engine,
    audio: &[f32],
    language: Option<&str>,
    custom_words: &[String],
    cancel: &CancelToken,
) -> Result<(String, Option<String>), TranscribeError> {
    // Qwen3-ASR has no prompt biasing; Whisper takes custom words as its
    // initial prompt.
    let family = (engine.is_whisper && !custom_words.is_empty()).then(|| {
        RunExtension::Whisper(WhisperRunOptions {
            initial_prompt: Some(custom_words.join(", ")),
            ..Default::default()
        })
    });
    let options = RunOptions {
        language: language.map(str::to_string),
        family,
        ..Default::default()
    };
    match engine.session.run(audio, &options) {
        Ok(t) => Ok((t.text.trim().to_string(), t.language)),
        Err(_) if cancel.is_cancelled() => Err(TranscribeError::Cancelled),
        Err(e) => Err(TranscribeError::Failed(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_language_accepts_en_and_vi_forms() {
        assert_eq!(supported_language(Some("en")), Some("en"));
        assert_eq!(supported_language(Some("en-US")), Some("en"));
        assert_eq!(supported_language(Some("English")), Some("en"));
        assert_eq!(supported_language(Some("vi")), Some("vi"));
        assert_eq!(supported_language(Some("Vietnamese")), Some("vi"));
    }

    #[test]
    fn supported_language_rejects_others_and_missing() {
        assert_eq!(supported_language(Some("zh")), None);
        assert_eq!(supported_language(Some("")), None);
        assert_eq!(supported_language(None), None);
    }

    #[test]
    fn vietnamese_text_is_reliable() {
        assert!(is_reliably_vietnamese(
            "Hôm nay trời mưa to quá, chắc mình hoãn buổi cà phê sang ngày mai."
        ));
    }

    #[test]
    fn english_text_is_not_vietnamese() {
        assert!(!is_reliably_vietnamese(
            "Hey, can you send me the quarterly report by Friday afternoon?"
        ));
    }

    #[test]
    fn short_ambiguous_text_is_not_reliable() {
        assert!(!is_reliably_vietnamese("ok"));
    }
}
