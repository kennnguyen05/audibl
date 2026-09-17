//! Dictation history. Filled in by the text pipeline milestone.

use crate::pipeline::PipelineOutput;
use tauri::AppHandle;

pub fn add(_app: &AppHandle, _output: PipelineOutput) {}

pub fn copy_last_transcript(_app: &AppHandle) {
    log::info!("Copy last transcript requested");
}
