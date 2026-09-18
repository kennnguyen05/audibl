//! Global keys: the Transcribe Shortcut plus Esc (cancel) and Return (finish),
//! which are registered only while they apply.
//!
//! Primary backend is handy-keys, a CGEventTap that swallows registered keys
//! and supports modifier-only and fn combos. If the tap cannot start, Audibl
//! falls back to tauri-plugin-global-shortcut. Ported from Handy
//! (`shortcut/handy_keys.rs`, `tauri_impl.rs`, `handler.rs`, MIT).
//!
//! Registration calls go to the handy-keys manager thread and wait for its
//! reply, so they must never run on that thread. Key events only forward to
//! the coordinator channel, which keeps that rule.

use crate::coordinator::{Coordinator, Input};
use crate::settings::get_settings;
use handy_keys::{Hotkey, HotkeyId, HotkeyManager, HotkeyState, KeyboardListener};
use log::{debug, error, info, warn};
use serde::Serialize;
use specta::Type;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_specta::Event;

pub const TRANSCRIBE: &str = "transcribe";
pub const CANCEL: &str = "cancel";
pub const FINISH: &str = "finish";
const CANCEL_KEY: &str = "escape";
const FINISH_KEY: &str = "return";

enum Command {
    Register {
        id: String,
        hotkey: String,
        reply: Sender<Result<(), String>>,
    },
    Unregister {
        id: String,
        reply: Sender<Result<(), String>>,
    },
}

enum Backend {
    HandyKeys(Mutex<Sender<Command>>),
    Tauri(Mutex<HashMap<String, Shortcut>>),
}

pub struct ShortcutManager {
    backend: Backend,
    capture_listener: Arc<Mutex<Option<KeyboardListener>>>,
    capture_running: Arc<AtomicBool>,
}

/// Key event streamed to the Shortcut capture field.
#[derive(Debug, Clone, Serialize, Type, Event)]
pub struct ShortcutCaptureEvent {
    pub modifiers: Vec<String>,
    pub key: Option<String>,
    pub is_key_down: bool,
    pub hotkey_string: String,
}

fn route_event(app: &AppHandle, id: &str, pressed: bool) {
    let Some(coordinator) = app.try_state::<Coordinator>() else {
        return;
    };
    match (id, pressed) {
        (TRANSCRIBE, true) => coordinator.send(Input::ShortcutPressed),
        (TRANSCRIBE, false) => coordinator.send(Input::ShortcutReleased),
        (CANCEL, true) => coordinator.send(Input::Cancel),
        (FINISH, true) => coordinator.send(Input::Finish),
        _ => {}
    }
}

impl ShortcutManager {
    /// Starts the handy-keys tap, or the global-shortcut fallback if it fails.
    pub fn start(app: &AppHandle) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<Command>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
        let thread_app = app.clone();
        thread::spawn(move || manager_thread(thread_app, cmd_rx, ready_tx));

        let backend = match ready_rx.recv() {
            Ok(Ok(())) => {
                info!("Shortcuts: handy-keys event tap active");
                Backend::HandyKeys(Mutex::new(cmd_tx))
            }
            Ok(Err(e)) => {
                warn!("Shortcuts: handy-keys unavailable ({e}); using global-shortcut fallback");
                Backend::Tauri(Mutex::new(HashMap::new()))
            }
            Err(_) => {
                warn!("Shortcuts: handy-keys thread exited; using global-shortcut fallback");
                Backend::Tauri(Mutex::new(HashMap::new()))
            }
        };

        Self {
            backend,
            capture_listener: Arc::new(Mutex::new(None)),
            capture_running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn register(&self, app: &AppHandle, id: &str, hotkey: &str) -> Result<(), String> {
        match &self.backend {
            Backend::HandyKeys(tx) => {
                let (reply, rx) = mpsc::channel();
                tx.lock()
                    .unwrap()
                    .send(Command::Register {
                        id: id.to_string(),
                        hotkey: hotkey.to_string(),
                        reply,
                    })
                    .map_err(|_| "shortcut thread stopped".to_string())?;
                rx.recv()
                    .map_err(|_| "shortcut thread stopped".to_string())?
            }
            Backend::Tauri(registered) => {
                let shortcut: Shortcut = hotkey
                    .parse()
                    .map_err(|e| format!("invalid shortcut '{hotkey}': {e}"))?;
                let event_id = id.to_string();
                app.global_shortcut()
                    .on_shortcut(shortcut, move |app, _, event| {
                        route_event(app, &event_id, event.state == ShortcutState::Pressed);
                    })
                    .map_err(|e| e.to_string())?;
                registered.lock().unwrap().insert(id.to_string(), shortcut);
                Ok(())
            }
        }
    }

    pub fn unregister(&self, app: &AppHandle, id: &str) -> Result<(), String> {
        match &self.backend {
            Backend::HandyKeys(tx) => {
                let (reply, rx) = mpsc::channel();
                tx.lock()
                    .unwrap()
                    .send(Command::Unregister {
                        id: id.to_string(),
                        reply,
                    })
                    .map_err(|_| "shortcut thread stopped".to_string())?;
                rx.recv()
                    .map_err(|_| "shortcut thread stopped".to_string())?
            }
            Backend::Tauri(registered) => {
                if let Some(shortcut) = registered.lock().unwrap().remove(id) {
                    app.global_shortcut()
                        .unregister(shortcut)
                        .map_err(|e| e.to_string())?;
                }
                Ok(())
            }
        }
    }

    pub fn is_event_tap(&self) -> bool {
        matches!(self.backend, Backend::HandyKeys(_))
    }
}

fn manager_thread(app: AppHandle, commands: Receiver<Command>, ready: Sender<Result<(), String>>) {
    let manager = match HotkeyManager::new_with_blocking() {
        Ok(m) => {
            let _ = ready.send(Ok(()));
            m
        }
        Err(e) => {
            let _ = ready.send(Err(e.to_string()));
            return;
        }
    };

    let mut ids: HashMap<String, HotkeyId> = HashMap::new();
    let mut names: HashMap<HotkeyId, String> = HashMap::new();

    loop {
        while let Some(event) = manager.try_recv() {
            if let Some(id) = names.get(&event.id) {
                let pressed = event.state == HotkeyState::Pressed;
                debug!("handy-keys event: {id} pressed={pressed}");
                route_event(&app, id, pressed);
            }
        }

        match commands.recv_timeout(Duration::from_millis(5)) {
            Ok(Command::Register { id, hotkey, reply }) => {
                let result = (|| {
                    let parsed: Hotkey = hotkey
                        .parse()
                        .map_err(|e| format!("invalid shortcut '{hotkey}': {e}"))?;
                    // Re-registering the same id replaces the old hotkey.
                    if let Some(old) = ids.remove(&id) {
                        let _ = manager.unregister(old);
                        names.remove(&old);
                    }
                    let hotkey_id = manager
                        .register(parsed)
                        .map_err(|e| format!("could not register '{hotkey}': {e}"))?;
                    ids.insert(id.clone(), hotkey_id);
                    names.insert(hotkey_id, id);
                    Ok(())
                })();
                let _ = reply.send(result);
            }
            Ok(Command::Unregister { id, reply }) => {
                let result = match ids.remove(&id) {
                    Some(hotkey_id) => {
                        names.remove(&hotkey_id);
                        manager.unregister(hotkey_id).map_err(|e| e.to_string())
                    }
                    None => Ok(()),
                };
                let _ = reply.send(result);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
    }
}

// ---------------------------------------------------------------------------
// App-level helpers
// ---------------------------------------------------------------------------

/// Starts the backend and registers the Transcribe Shortcut. Idempotent.
pub fn init(app: &AppHandle) {
    if app.try_state::<ShortcutManager>().is_none() {
        app.manage(ShortcutManager::start(app));
    }
    let shortcut = get_settings(app).shortcut;
    if let Err(e) = register(app, TRANSCRIBE, &shortcut) {
        error!("Failed to register Transcribe Shortcut '{shortcut}': {e}");
    }
}

fn register(app: &AppHandle, id: &str, hotkey: &str) -> Result<(), String> {
    app.try_state::<ShortcutManager>()
        .ok_or("shortcuts not initialized")?
        .register(app, id, hotkey)
}

fn unregister(app: &AppHandle, id: &str) {
    if let Some(manager) = app.try_state::<ShortcutManager>() {
        if let Err(e) = manager.unregister(app, id) {
            warn!("Failed to unregister shortcut {id}: {e}");
        }
    }
}

pub fn arm_cancel(app: &AppHandle) {
    if let Err(e) = register(app, CANCEL, CANCEL_KEY) {
        warn!("Failed to register Esc: {e}");
    }
}

pub fn disarm_cancel(app: &AppHandle) {
    unregister(app, CANCEL);
}

pub fn arm_finish(app: &AppHandle) {
    if let Err(e) = register(app, FINISH, FINISH_KEY) {
        warn!("Failed to register Return: {e}");
    }
}

pub fn disarm_finish(app: &AppHandle) {
    unregister(app, FINISH);
}

/// Swaps the Transcribe Shortcut, restoring the old one if the new one fails.
pub fn change_shortcut(app: &AppHandle, new: &str) -> Result<(), String> {
    let new = new.trim();
    if new.is_empty() {
        return Err("empty_shortcut".into());
    }
    new.parse::<Hotkey>()
        .map_err(|e| format!("invalid shortcut: {e}"))?;
    let old = get_settings(app).shortcut;
    if let Err(e) = register(app, TRANSCRIBE, new) {
        let _ = register(app, TRANSCRIBE, &old);
        return Err(e);
    }
    Ok(())
}

/// Streams key events to the UI so the user can press a new shortcut. The
/// current shortcut is suspended during capture so it can't fire.
pub fn start_capture(app: &AppHandle) -> Result<(), String> {
    let manager = app
        .try_state::<ShortcutManager>()
        .ok_or("shortcuts not initialized")?;
    if manager.capture_running.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let listener = match KeyboardListener::new() {
        Ok(l) => l,
        Err(e) => {
            manager.capture_running.store(false, Ordering::SeqCst);
            return Err(format!("capture_unavailable: {e}"));
        }
    };
    unregister(app, TRANSCRIBE);
    *manager.capture_listener.lock().unwrap() = Some(listener);

    let running = Arc::clone(&manager.capture_running);
    let listener = Arc::clone(&manager.capture_listener);
    let app = app.clone();
    thread::spawn(move || {
        while running.load(Ordering::SeqCst) {
            let event = listener.lock().unwrap().as_ref().and_then(|l| l.try_recv());
            match event {
                Some(e) => {
                    let _ = ShortcutCaptureEvent {
                        modifiers: modifier_names(e.modifiers),
                        key: e.key.map(|k| k.to_string().to_lowercase()),
                        is_key_down: e.is_key_down,
                        hotkey_string: e
                            .as_hotkey()
                            .map(|h| h.to_handy_string())
                            .unwrap_or_default(),
                    }
                    .emit(&app);
                }
                None => thread::sleep(Duration::from_millis(10)),
            }
        }
    });
    Ok(())
}

/// Ends capture and re-registers the stored shortcut.
pub fn stop_capture(app: &AppHandle) {
    if let Some(manager) = app.try_state::<ShortcutManager>() {
        manager.capture_running.store(false, Ordering::SeqCst);
        *manager.capture_listener.lock().unwrap() = None;
    }
    let shortcut = get_settings(app).shortcut;
    if let Err(e) = register(app, TRANSCRIBE, &shortcut) {
        error!("Failed to restore Transcribe Shortcut after capture: {e}");
    }
}

fn modifier_names(modifiers: handy_keys::Modifiers) -> Vec<String> {
    use handy_keys::Modifiers;
    [
        (Modifiers::CTRL, "ctrl"),
        (Modifiers::OPT, "option"),
        (Modifiers::SHIFT, "shift"),
        (Modifiers::CMD, "command"),
        (Modifiers::FN, "fn"),
    ]
    .into_iter()
    .filter(|(flag, _)| modifiers.contains(*flag))
    .map(|(_, name)| name.to_string())
    .collect()
}
