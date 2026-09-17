//! Persistent app settings: one `AppSettings` struct in `settings_store.json`.
//!
//! Every field has a serde default, so a store written by an older build loads
//! cleanly. A field that fails to parse falls back to its default without
//! resetting the other fields. The Groq API key is never stored here; it lives
//! in the macOS Keychain (see `keychain.rs`).

use log::warn;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use tauri_specta::Event;

pub const SETTINGS_STORE_PATH: &str = "settings_store.json";
pub const DEFAULT_SHORTCUT: &str = "option+space";

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum ActivationMode {
    Hold,
    Toggle,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum AppLanguage {
    En,
    Vi,
}

impl AppLanguage {
    pub fn code(self) -> &'static str {
        match self {
            AppLanguage::En => "en",
            AppLanguage::Vi => "vi",
        }
    }

    /// `vi` when the macOS preferred language is Vietnamese, else `en`.
    pub fn from_locale(locale: Option<&str>) -> Self {
        match locale {
            Some(l) if l.to_lowercase().starts_with("vi") => AppLanguage::Vi,
            _ => AppLanguage::En,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Type)]
pub struct Replacement {
    pub trigger: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Type)]
#[serde(default)]
pub struct AppSettings {
    pub onboarding_complete: bool,
    pub shortcut: String,
    pub activation_mode: ActivationMode,
    /// Microphone name; `None` follows the system default input.
    pub selected_microphone: Option<String>,
    /// Zero-based input channel; `None` mixes all channels down to mono.
    pub selected_channel: Option<u16>,
    pub mute_while_recording: bool,
    pub start_hidden: bool,
    pub autostart_enabled: bool,
    pub show_tray_icon: bool,
    pub remove_filler_words: bool,
    pub app_language: AppLanguage,
    pub clean_and_reformat: bool,
    pub custom_words: Vec<String>,
    pub replacements: Vec<Replacement>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            onboarding_complete: false,
            shortcut: DEFAULT_SHORTCUT.to_string(),
            activation_mode: ActivationMode::Hold,
            selected_microphone: None,
            selected_channel: None,
            mute_while_recording: false,
            start_hidden: false,
            autostart_enabled: false,
            show_tray_icon: true,
            remove_filler_words: true,
            app_language: AppLanguage::En,
            clean_and_reformat: false,
            custom_words: Vec::new(),
            replacements: Vec::new(),
        }
    }
}

impl AppSettings {
    /// Start Hidden is ignored while the menu bar icon is off, so the app can
    /// never become unreachable.
    pub fn effective_start_hidden(&self) -> bool {
        self.start_hidden && self.show_tray_icon
    }
}

/// Emitted with the full settings after every change. The overlay webview
/// uses it to follow the interface language live.
#[derive(Serialize, Deserialize, Debug, Clone, Type, Event)]
pub struct SettingsChanged(pub AppSettings);

pub fn get_settings(app: &AppHandle) -> AppSettings {
    let store = app
        .store(SETTINGS_STORE_PATH)
        .expect("Failed to open settings store");

    match store.get("settings") {
        Some(value) => match serde_json::from_value::<AppSettings>(value.clone()) {
            Ok(settings) => settings,
            Err(e) => {
                warn!("Failed to parse stored settings ({e}); salvaging valid fields");
                let settings = salvage_settings(&value);
                store.set("settings", serde_json::to_value(&settings).unwrap());
                settings
            }
        },
        None => {
            let settings = AppSettings {
                app_language: AppLanguage::from_locale(tauri_plugin_os::locale().as_deref()),
                ..AppSettings::default()
            };
            store.set("settings", serde_json::to_value(&settings).unwrap());
            settings
        }
    }
}

pub fn write_settings(app: &AppHandle, settings: &AppSettings) {
    let store = app
        .store(SETTINGS_STORE_PATH)
        .expect("Failed to open settings store");
    store.set("settings", serde_json::to_value(settings).unwrap());
}

/// Reads, mutates, persists, and broadcasts the settings in one step.
pub fn update_settings(app: &AppHandle, mutate: impl FnOnce(&mut AppSettings)) -> AppSettings {
    let mut settings = get_settings(app);
    mutate(&mut settings);
    write_settings(app, &settings);
    if let Err(e) = SettingsChanged(settings.clone()).emit(app) {
        warn!("Failed to emit settings-changed: {e}");
    }
    settings
}

/// Trims words, drops empty ones and case-insensitive duplicates, keeping the
/// first spelling entered.
pub fn normalize_custom_words(words: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    words
        .into_iter()
        .map(|w| w.trim().to_string())
        .filter(|w| !w.is_empty() && seen.insert(w.to_lowercase()))
        .collect()
}

/// Trims triggers (values are kept exactly as typed), drops rows with an empty
/// trigger or value, and keeps the first row for a repeated trigger.
pub fn normalize_replacements(replacements: Vec<Replacement>) -> Vec<Replacement> {
    let mut seen = std::collections::HashSet::new();
    replacements
        .into_iter()
        .map(|r| Replacement {
            trigger: r.trigger.trim().to_string(),
            value: r.value,
        })
        .filter(|r| {
            !r.trigger.is_empty() && !r.value.trim().is_empty() && seen.insert(r.trigger.clone())
        })
        .collect()
}

/// Keeps every stored field that is individually valid; broken ones fall back
/// to their default.
fn salvage_settings(stored: &serde_json::Value) -> AppSettings {
    let Some(stored_map) = stored.as_object() else {
        return AppSettings::default();
    };
    let mut merged = serde_json::to_value(AppSettings::default()).unwrap();
    for (key, value) in stored_map {
        let map = merged.as_object_mut().unwrap();
        let previous = map.insert(key.clone(), value.clone());
        if serde_json::from_value::<AppSettings>(merged.clone()).is_err() {
            warn!("Dropping invalid settings field '{key}', keeping its default");
            let map = merged.as_object_mut().unwrap();
            match previous {
                Some(previous) => map.insert(key.clone(), previous),
                None => map.remove(key),
            };
        }
    }
    serde_json::from_value(merged).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_use_defaults() {
        let parsed: AppSettings =
            serde_json::from_value(serde_json::json!({ "start_hidden": true })).unwrap();
        assert!(parsed.start_hidden);
        assert_eq!(parsed.shortcut, DEFAULT_SHORTCUT);
        assert!(parsed.remove_filler_words);
    }

    #[test]
    fn salvage_keeps_valid_fields() {
        let stored = serde_json::json!({
            "start_hidden": true,
            "activation_mode": "not_a_mode",
        });
        let salvaged = salvage_settings(&stored);
        assert!(salvaged.start_hidden);
        assert_eq!(salvaged.activation_mode, ActivationMode::Hold);
    }

    #[test]
    fn language_follows_locale() {
        assert_eq!(AppLanguage::from_locale(Some("vi-VN")), AppLanguage::Vi);
        assert_eq!(AppLanguage::from_locale(Some("en-US")), AppLanguage::En);
        assert_eq!(AppLanguage::from_locale(None), AppLanguage::En);
    }

    #[test]
    fn custom_words_are_trimmed_and_deduplicated() {
        let words = normalize_custom_words(vec![
            " Tauri ".into(),
            "".into(),
            "tauri".into(),
            "Nguyễn".into(),
        ]);
        assert_eq!(words, vec!["Tauri".to_string(), "Nguyễn".to_string()]);
    }

    #[test]
    fn replacements_keep_value_verbatim() {
        let rows = normalize_replacements(vec![
            Replacement {
                trigger: " my email ".into(),
                value: "Ken@Example.com".into(),
            },
            Replacement {
                trigger: "my email".into(),
                value: "other".into(),
            },
            Replacement {
                trigger: "empty".into(),
                value: "  ".into(),
            },
        ]);
        assert_eq!(
            rows,
            vec![Replacement {
                trigger: "my email".into(),
                value: "Ken@Example.com".into(),
            }]
        );
    }

    #[test]
    fn start_hidden_requires_tray_icon() {
        let settings = AppSettings {
            start_hidden: true,
            show_tray_icon: false,
            ..AppSettings::default()
        };
        assert!(!settings.effective_start_hidden());
    }
}
