mod actions;
mod audio;
mod autostart;
mod cleanup;
mod commands;
mod context;
mod coordinator;
mod history;
mod keychain;
mod model;
mod overlay;
mod paste;
mod permissions;
mod pipeline;
mod secure_input;
mod settings;
mod sfx;
mod shortcut;
mod text;
mod transcription;
mod tray;
mod tray_i18n;

use settings::get_settings;
use tauri::{AppHandle, Manager};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};
use tauri_specta::{collect_commands, collect_events, Builder};

/// Pins the whole app to the dark system appearance. Audibl has one palette,
/// so the traffic lights, the `<select>` popup menu and every other native
/// control must not follow the user's macOS appearance setting.
fn force_dark_appearance() {
    use objc2_app_kit::{NSAppearance, NSAppearanceNameDarkAqua, NSApplication};
    use objc2_foundation::MainThreadMarker;

    let Some(mtm) = MainThreadMarker::new() else {
        log::error!("force_dark_appearance called off the main thread");
        return;
    };
    let appearance = unsafe { NSAppearance::appearanceNamed(NSAppearanceNameDarkAqua) };
    NSApplication::sharedApplication(mtm).setAppearance(appearance.as_deref());
}

pub fn show_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        log::error!("Main window not found");
        return;
    };
    let _ = window.unminimize();
    if let Err(e) = window.show() {
        log::error!("Failed to show main window: {e}");
    }
    let _ = window.set_focus();
    if let Err(e) = app.set_activation_policy(tauri::ActivationPolicy::Regular) {
        log::error!("Failed to set activation policy to Regular: {e}");
    }
}

/// True when dictation can run: onboarding done, permissions granted, and
/// the model on disk. Otherwise the main window must show onboarding.
pub fn is_setup_complete(app: &AppHandle) -> bool {
    get_settings(app).onboarding_complete
        && permissions::has_microphone()
        && permissions::has_accessibility()
        && app.state::<model::ModelManager>().is_ready(app)
}

/// Registers the Transcribe Shortcut once onboarding is done and the event
/// tap can run. A missing model does not block it: pressing the shortcut
/// then opens the download screen instead of recording.
pub fn on_ready(app: &AppHandle) {
    let settings = get_settings(app);
    if settings.onboarding_complete && permissions::has_accessibility() {
        shortcut::init(app);
        log::info!("Shortcut '{}' ready", settings.shortcut);
    }
}

fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::get_app_settings,
            commands::show_main_window_command,
            commands::set_activation_mode,
            commands::set_mute_while_recording,
            commands::set_start_hidden,
            commands::set_autostart,
            commands::set_show_tray_icon,
            commands::set_remove_filler_words,
            commands::set_app_language,
            commands::set_clean_and_reformat,
            commands::set_custom_words,
            commands::set_replacements,
            commands::set_groq_api_key,
            commands::has_groq_api_key,
            commands::clear_groq_api_key,
            commands::open_groq_keys_page,
            commands::get_microphones,
            commands::set_microphone,
            commands::get_channel_count,
            commands::set_channel,
            commands::get_model_status,
            commands::start_model_download,
            commands::cancel_model_download,
            commands::complete_onboarding,
            commands::ensure_shortcut,
            commands::quit_app,
            commands::change_shortcut,
            commands::start_shortcut_capture,
            commands::stop_shortcut_capture,
            commands::get_history,
            commands::delete_history_entry,
            commands::copy_text,
        ])
        .events(collect_events![
            settings::SettingsChanged,
            model::ModelDownloadProgress,
            model::ModelDownloadFailed,
            model::ModelDownloadComplete,
            shortcut::ShortcutCaptureEvent,
            history::HistoryChanged,
        ])
}

pub fn run() {
    // Avoid ggml-metal residency-set teardown assertions when a native engine
    // outlives the Tauri shutdown sequence. Must be set before transcribe-cpp
    // initializes its Metal device.
    std::env::set_var("GGML_METAL_NO_RESIDENCY", "1");

    let specta_builder = specta_builder();

    #[cfg(debug_assertions)]
    export_bindings(&specta_builder);

    let invoke_handler = specta_builder.invoke_handler();

    let mut app = tauri::Builder::default()
        // Single-instance must be registered first: a second launch just
        // raises the running app.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::recreate_tray_icon(app);
            show_main_window(app);
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Debug)
                .max_file_size(500_000)
                .rotation_strategy(RotationStrategy::KeepOne)
                .clear_targets()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir {
                        file_name: Some("audibl".into()),
                    }),
                ])
                .build(),
        )
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_macos_permissions::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_nspanel::init())
        .setup(move |app| {
            specta_builder.mount_events(app);
            let handle = app.handle().clone();
            app.manage(model::ModelManager::default());
            app.manage(audio::AudioManager::new(&handle));
            transcription::TranscriptionManager::init_backend();
            app.manage(transcription::TranscriptionManager::new(&handle));
            app.manage(coordinator::Coordinator::new(handle.clone()));

            // The app is dark only, so the native chrome is pinned dark too:
            // otherwise the traffic lights and the <select> popup menu would
            // follow the user's macOS appearance and render light on our ink.
            force_dark_appearance();

            // Overlay + hidden title lets the sidebar run to the top edge with
            // the traffic lights floating over it. Sidebar.tsx reserves the
            // space for them.
            tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("/".into()))
                .title("Audibl")
                // Fixed size: every page is laid out for this one window, so
                // there is nothing for a resize to reflow. 712 is General's
                // full height (both cards, no warning) plus main's pb-10, so
                // that page never scrolls.
                .inner_size(900.0, 712.0)
                .resizable(false)
                .maximizable(false)
                .title_bar_style(tauri::TitleBarStyle::Overlay)
                .hidden_title(true)
                // Matches --color-bg, so showing the window never flashes white.
                .background_color(tauri::window::Color(0x16, 0x14, 0x12, 0xff))
                .visible(false)
                .build()?;

            tray::create_tray(&handle)?;
            overlay::create(&handle);

            let settings = get_settings(&handle);
            autostart::apply_autostart(settings.autostart_enabled);

            on_ready(&handle);
            #[cfg(debug_assertions)]
            dev_overlay_preview(&handle);
            secure_input::start_monitor(&handle);
            if !is_setup_complete(&handle) || !settings.effective_start_hidden() {
                show_main_window(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() != "main" {
                    return;
                }
                api.prevent_close();
                let _ = window.hide();
                // With the menu bar icon visible, the app lives there and
                // leaves the Dock. Without it, the Dock icon is the only way
                // back in, so it stays.
                let app = window.app_handle();
                if get_settings(app).show_tray_icon {
                    if let Err(e) = app.set_activation_policy(tauri::ActivationPolicy::Accessory) {
                        log::error!("Failed to set activation policy: {e}");
                    }
                }
            }
        })
        .invoke_handler(invoke_handler)
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // A hidden launch must start as Accessory (no Dock icon) between build()
    // and run(). Demoting an already-activated app at runtime is unreliable
    // on macOS 26 (Handy #1787).
    let launch_settings = get_settings(app.handle());
    if launch_settings.onboarding_complete && launch_settings.effective_start_hidden() {
        app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    }

    app.run(|app, event| match event {
        tauri::RunEvent::Reopen { .. } => {
            let visible = app
                .get_webview_window("main")
                .and_then(|w| w.is_visible().ok())
                .unwrap_or(false);
            if !visible {
                tray::recreate_tray_icon(app);
            }
            show_main_window(app);
        }
        // ggml-metal asserts if a model's Metal resources outlive static
        // destructors, so drop the engine before exit.
        tauri::RunEvent::Exit => {
            if let Some(tm) = app.try_state::<std::sync::Arc<transcription::TranscriptionManager>>()
            {
                tm.unload();
            }
        }
        _ => {}
    });
}

/// `AUDIBLE_DEV_OVERLAY=recording|transcribing|cleaning` shows the recording
/// overlay in that state at startup and leaves it up, so each state can be
/// screenshotted without speaking into the microphone. Debug builds only;
/// with the variable unset it returns before touching anything.
#[cfg(debug_assertions)]
fn dev_overlay_preview(app: &AppHandle) {
    let Ok(raw) = std::env::var("AUDIBLE_DEV_OVERLAY") else {
        return;
    };
    let state = match raw.trim().to_ascii_lowercase().as_str() {
        "recording" => overlay::OverlayState::Recording,
        "transcribing" => overlay::OverlayState::Transcribing,
        "cleaning" => overlay::OverlayState::Cleaning,
        other => {
            log::warn!("Ignoring AUDIBLE_DEV_OVERLAY={other:?}");
            return;
        }
    };
    let handle = app.clone();
    // The overlay webview subscribes to `show-overlay` after it mounts, which
    // is well after setup runs, so re-show it a few times until it sticks.
    std::thread::spawn(move || {
        for _ in 0..4 {
            std::thread::sleep(std::time::Duration::from_millis(1200));
            overlay::show(&handle, state);
        }
    });
}

#[cfg(debug_assertions)]
fn export_bindings(builder: &Builder<tauri::Wry>) {
    use specta_typescript::{BigIntExportBehavior, Typescript};
    builder
        .export(
            Typescript::default()
                .bigint(BigIntExportBehavior::Number)
                .header("// @ts-nocheck"),
            concat!(env!("CARGO_MANIFEST_DIR"), "/../src/bindings.ts"),
        )
        .expect("Failed to export TypeScript bindings");
}

#[cfg(test)]
mod tests {
    /// `cargo test export_bindings` regenerates `src/bindings.ts` without
    /// launching the app.
    #[test]
    fn export_bindings() {
        super::export_bindings(&super::specta_builder());
    }
}
