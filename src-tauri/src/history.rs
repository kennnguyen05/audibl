//! History: the last 10 dictations, text only (no audio), in `history.json`.

use crate::pipeline::PipelineOutput;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_store::StoreExt;
use tauri_specta::Event;

pub const MAX_ENTRIES: usize = 10;
const STORE_PATH: &str = "history.json";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Type)]
pub struct HistoryEntry {
    /// Milliseconds since the Unix epoch; also the entry's id.
    pub timestamp: f64,
    /// The text that was pasted.
    pub text: String,
    /// The transcript before local cleanup and Clean and Reformat.
    pub raw_text: String,
    pub app_name: Option<String>,
}

/// Emitted with the full list after any change.
#[derive(Serialize, Deserialize, Debug, Clone, Type, Event)]
pub struct HistoryChanged(pub Vec<HistoryEntry>);

/// Newest first, capped at `MAX_ENTRIES`.
pub fn push_capped(entries: &mut Vec<HistoryEntry>, entry: HistoryEntry) {
    entries.insert(0, entry);
    entries.truncate(MAX_ENTRIES);
}

pub fn load(app: &AppHandle) -> Vec<HistoryEntry> {
    app.store(STORE_PATH)
        .ok()
        .and_then(|store| store.get("entries"))
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

fn save(app: &AppHandle, entries: &[HistoryEntry]) {
    match app.store(STORE_PATH) {
        Ok(store) => store.set("entries", serde_json::to_value(entries).unwrap()),
        Err(e) => log::error!("Failed to open history store: {e}"),
    }
    let _ = HistoryChanged(entries.to_vec()).emit(app);
}

pub fn add(app: &AppHandle, output: PipelineOutput) {
    let mut entries = load(app);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as f64;
    push_capped(
        &mut entries,
        HistoryEntry {
            timestamp,
            text: output.final_text,
            raw_text: output.raw_text,
            app_name: output.app_name,
        },
    );
    save(app, &entries);
}

pub fn delete(app: &AppHandle, timestamp: f64) -> Vec<HistoryEntry> {
    let mut entries = load(app);
    entries.retain(|e| e.timestamp != timestamp);
    save(app, &entries);
    entries
}

pub fn copy_text(app: &AppHandle, text: &str) -> Result<(), String> {
    app.clipboard().write_text(text).map_err(|e| e.to_string())
}

/// Tray "Copy Last Transcript".
pub fn copy_last_transcript(app: &AppHandle) {
    match load(app).first() {
        Some(entry) => {
            if let Err(e) = copy_text(app, &entry.text) {
                log::error!("Failed to copy last transcript: {e}");
            }
        }
        None => log::info!("No transcript to copy"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(n: u32) -> HistoryEntry {
        HistoryEntry {
            timestamp: n as f64,
            text: format!("text {n}"),
            raw_text: format!("raw {n}"),
            app_name: None,
        }
    }

    #[test]
    fn keeps_only_the_ten_newest() {
        let mut entries = Vec::new();
        for n in 1..=11 {
            push_capped(&mut entries, entry(n));
        }
        assert_eq!(entries.len(), MAX_ENTRIES);
        assert_eq!(entries.first().unwrap().timestamp, 11.0);
        assert_eq!(entries.last().unwrap().timestamp, 2.0);
    }

    #[test]
    fn newest_is_first() {
        let mut entries = vec![entry(1)];
        push_capped(&mut entries, entry(2));
        assert_eq!(entries[0].text, "text 2");
    }
}
