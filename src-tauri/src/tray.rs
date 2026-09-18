//! Menu bar (tray) icon and menu.
//!
//! Callers record intent (`set_tray_state`, `sync_tray`) and one applier runs
//! on the main thread. It touches the native tray only when the icon or the
//! menu inputs changed. Bursts of updates coalesce into one apply. Handy found
//! that concurrent menu rebuilds can make the macOS status item vanish
//! (tauri-apps/tauri#12060), which this avoids.

use crate::settings::{self, AppLanguage};
use crate::tray_i18n::get_tray_translations;
use log::{error, info, warn};
use std::sync::{Mutex, MutexGuard};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Manager};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayIconState {
    Idle,
    Recording,
    Processing,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MenuInputs {
    busy: bool,
    secure_input_warning: bool,
    language: AppLanguage,
}

struct Inner {
    icon_state: TrayIconState,
    secure_input_warning: bool,
    applied_icon: Option<TrayIconState>,
    applied_menu: Option<MenuInputs>,
    pending: bool,
}

pub struct TrayState(Mutex<Inner>);

impl TrayState {
    pub fn new() -> Self {
        Self(Mutex::new(Inner {
            icon_state: TrayIconState::Idle,
            secure_input_warning: false,
            applied_icon: None,
            applied_menu: None,
            pending: false,
        }))
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }
}

fn icon_bytes(state: TrayIconState) -> &'static [u8] {
    match state {
        TrayIconState::Idle => include_bytes!("../resources/tray_idle.png"),
        TrayIconState::Recording => include_bytes!("../resources/tray_recording.png"),
        TrayIconState::Processing => include_bytes!("../resources/tray_processing.png"),
    }
}

/// Builds the tray at startup. Menu events are routed to `lib.rs` handlers.
pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    app.manage(TrayState::new());
    let tray = TrayIconBuilder::with_id("main")
        .icon(Image::from_bytes(icon_bytes(TrayIconState::Idle))?)
        .icon_as_template(true)
        .tooltip("Audibl")
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" | "secure_input_warning" => crate::show_main_window(app),
            "copy_last_transcript" => crate::history::copy_last_transcript(app),
            "cancel" => crate::actions::cancel_current_operation(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    app.manage(tray);
    sync_tray(app);

    let visible = settings::get_settings(app).show_tray_icon;
    if !visible {
        set_tray_visibility(app, false);
    }
    Ok(())
}

pub fn set_tray_state(app: &AppHandle, state: TrayIconState) {
    if let Some(tray_state) = app.try_state::<TrayState>() {
        tray_state.lock().icon_state = state;
    }
    sync_tray(app);
}

pub fn set_secure_input_warning(app: &AppHandle, active: bool) {
    if let Some(tray_state) = app.try_state::<TrayState>() {
        tray_state.lock().secure_input_warning = active;
    }
    sync_tray(app);
}

/// Schedules one main-thread apply unless one is already pending.
pub fn sync_tray(app: &AppHandle) {
    let Some(state) = app.try_state::<TrayState>() else {
        return;
    };
    let schedule = !std::mem::replace(&mut state.lock().pending, true);
    if !schedule {
        return;
    }
    let handle = app.clone();
    if let Err(e) = app.run_on_main_thread(move || apply_on_main(&handle)) {
        error!("Failed to dispatch tray update: {e}");
        state.lock().pending = false;
    }
}

fn apply_on_main(app: &AppHandle) {
    let (Some(state), Some(tray)) = (app.try_state::<TrayState>(), app.try_state::<TrayIcon>())
    else {
        return;
    };
    // Settings are read before taking the tray lock so a slow store read
    // never blocks other callers.
    let language = settings::get_settings(app).app_language;

    let (icon_state, menu_inputs, icon_changed, menu_changed) = {
        let mut inner = state.lock();
        inner.pending = false;
        let menu_inputs = MenuInputs {
            busy: inner.icon_state != TrayIconState::Idle,
            secure_input_warning: inner.secure_input_warning,
            language,
        };
        let icon_changed = inner.applied_icon != Some(inner.icon_state);
        let menu_changed = inner.applied_menu.as_ref() != Some(&menu_inputs);
        (inner.icon_state, menu_inputs, icon_changed, menu_changed)
    };

    if icon_changed {
        match Image::from_bytes(icon_bytes(icon_state))
            .and_then(|image| tray.set_icon_with_as_template(Some(image), true))
        {
            Ok(()) => state.lock().applied_icon = Some(icon_state),
            Err(e) => error!("Failed to update tray icon: {e}"),
        }
    }

    if menu_changed {
        match build_menu(app, &menu_inputs).and_then(|menu| tray.set_menu(Some(menu))) {
            Ok(()) => state.lock().applied_menu = Some(menu_inputs),
            Err(e) => error!("Failed to update tray menu: {e}"),
        }
    }
}

fn build_menu(app: &AppHandle, inputs: &MenuInputs) -> tauri::Result<Menu<tauri::Wry>> {
    let strings = get_tray_translations(inputs.language.code());
    let menu = Menu::new(app)?;

    if inputs.secure_input_warning {
        menu.append(&MenuItem::with_id(
            app,
            "secure_input_warning",
            &strings.secure_input_warning,
            true,
            None::<&str>,
        )?)?;
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }

    menu.append(&MenuItem::with_id(
        app,
        "open",
        &strings.open,
        true,
        Some("Cmd+,"),
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        "copy_last_transcript",
        &strings.copy_last_transcript,
        true,
        None::<&str>,
    )?)?;
    if inputs.busy {
        menu.append(&MenuItem::with_id(
            app,
            "cancel",
            &strings.cancel,
            true,
            None::<&str>,
        )?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        "quit",
        &strings.quit,
        true,
        Some("Cmd+Q"),
    )?)?;
    Ok(menu)
}

pub fn set_tray_visibility(app: &AppHandle, visible: bool) {
    let Some(tray) = app.try_state::<TrayIcon>() else {
        return;
    };
    match tray.set_visible(visible) {
        Ok(()) => info!("Tray visibility set to {visible}"),
        Err(e) => error!("Failed to set tray visibility: {e}"),
    }
}

/// Hiding and re-showing recreates a status item that macOS silently dropped.
/// Called when the user relaunches the already-running app.
pub fn recreate_tray_icon(app: &AppHandle) {
    if !settings::get_settings(app).show_tray_icon {
        return;
    }
    let Some(tray) = app.try_state::<TrayIcon>() else {
        return;
    };
    if let Err(e) = tray.set_visible(false).and_then(|_| tray.set_visible(true)) {
        warn!("Failed to recreate tray icon: {e}");
    }
}
