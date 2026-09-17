//! Text pipeline after transcription. Filled in by the text pipeline and
//! Clean and Reformat milestones.

use crate::transcription::Transcription;
use tauri::AppHandle;

#[derive(Debug, Clone)]
pub struct PipelineOutput {
    pub raw_text: String,
    pub final_text: String,
}

pub fn process(_app: &AppHandle, _session: u64, transcription: &Transcription) -> Option<PipelineOutput> {
    Some(PipelineOutput {
        raw_text: transcription.text.clone(),
        final_text: transcription.text.clone(),
    })
}
