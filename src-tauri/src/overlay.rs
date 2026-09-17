//! Recording overlay: a non-activating NSPanel pill centred above the Dock on
//! the screen with the mouse cursor. It never takes focus, so the target app
//! keeps its caret. Ported from Handy (`overlay.rs`, MIT), pill style only.

use crate::audio::OVERLAY_LABEL;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl};
use tauri_nspanel::{tauri_panel, CollectionBehavior, PanelBuilder, PanelLevel, StyleMask};

tauri_panel! {
    panel!(RecordingOverlayPanel {
        config: {
            can_become_key_window: false,
            is_floating_panel: true
        }
    })
}

const WIDTH: f64 = 240.0;
const HEIGHT: f64 = 48.0;
const BOTTOM_OFFSET: f64 = 16.0;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OverlayState {
    Recording,
    Transcribing,
    Cleaning,
}

#[derive(Clone, Serialize)]
struct ShowPayload {
    state: OverlayState,
    /// Toggle mode shows the Esc / Return hint while recording.
    toggle: bool,
}

/// Bumped on every show, so a delayed hide from an older session never hides
/// a newer one.
static SHOW_GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn create(app: &AppHandle) {
    let result = PanelBuilder::<_, RecordingOverlayPanel>::new(app, OVERLAY_LABEL)
        .url(WebviewUrl::App("src/overlay/index.html".into()))
        .title("Audible Overlay")
        .level(PanelLevel::Status)
        .size(tauri::Size::Logical(tauri::LogicalSize {
            width: WIDTH,
            height: HEIGHT,
        }))
        .has_shadow(false)
        .transparent(true)
        .no_activate(true)
        .corner_radius(0.0)
        .style_mask(StyleMask::empty().borderless().nonactivating_panel())
        .with_window(|w| w.decorations(false).transparent(true).focusable(false))
        .collection_behavior(
            CollectionBehavior::new()
                .can_join_all_spaces()
                .full_screen_auxiliary(),
        )
        .build();
    match result {
        Ok(panel) => panel.hide(),
        Err(e) => log::error!("Failed to create recording overlay: {e}"),
    }
}

/// Logical position centred on the work area of the monitor under the cursor.
fn position(app: &AppHandle) -> Option<(f64, f64)> {
    let cursor = crate::paste::cursor_location();
    let monitor = app
        .available_monitors()
        .ok()
        .and_then(|monitors| {
            let (cx, cy) = cursor?;
            monitors.into_iter().find(|m| {
                let scale = m.scale_factor();
                let x = m.position().x as f64 / scale;
                let y = m.position().y as f64 / scale;
                let w = m.size().width as f64 / scale;
                let h = m.size().height as f64 / scale;
                (cx as f64) >= x && (cx as f64) < x + w && (cy as f64) >= y && (cy as f64) < y + h
            })
        })
        .or_else(|| app.primary_monitor().ok().flatten())?;

    let scale = monitor.scale_factor();
    let x = monitor.position().x as f64 / scale + (monitor.size().width as f64 / scale - WIDTH) / 2.0;
    let work = monitor.work_area();
    let bottom = (work.position.y as f64 + work.size.height as f64) / scale;
    Some((x, bottom - HEIGHT - BOTTOM_OFFSET))
}

pub fn show(app: &AppHandle, state: OverlayState, toggle: bool) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(window) = handle.get_webview_window(OVERLAY_LABEL) else {
            return;
        };
        SHOW_GENERATION.fetch_add(1, Ordering::SeqCst);
        if let Some((x, y)) = position(&handle) {
            let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
        }
        let _ = window.show();
        let _ = handle.emit_to(OVERLAY_LABEL, "show-overlay", ShowPayload { state, toggle });
    });
}

pub fn hide(app: &AppHandle) {
    let Some(window) = app.get_webview_window(OVERLAY_LABEL) else {
        return;
    };
    let scheduled_at = SHOW_GENERATION.load(Ordering::SeqCst);
    let _ = app.emit_to(OVERLAY_LABEL, "hide-overlay", ());
    // Let the fade-out finish unless a newer session showed the overlay.
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));
        if SHOW_GENERATION.load(Ordering::SeqCst) == scheduled_at {
            let _ = window.hide();
        }
    });
}
