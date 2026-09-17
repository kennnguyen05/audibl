//! Text pipeline after transcription:
//! filler removal (if on) → custom-word correction → replacements
//! (case-sensitive only when Clean and Reformat is on) → Clean and Reformat
//! (if on) → output for paste and history.

use crate::context;
use crate::settings::get_settings;
use crate::text::{dictionary, filler};
use crate::transcription::Transcription;
use tauri::AppHandle;

#[derive(Debug, Clone)]
pub struct PipelineOutput {
    pub raw_text: String,
    pub final_text: String,
    pub app_name: Option<String>,
}

/// Local steps only; pure apart from the settings passed in.
pub fn process_local(
    text: &str,
    remove_fillers: bool,
    custom_words: &[String],
    replacements: &[crate::settings::Replacement],
    case_sensitive_replacements: bool,
) -> (String, Vec<String>) {
    let mut text = dictionary::nfc(text);
    if remove_fillers {
        text = filler::remove_filler_words(&text);
    }
    text = dictionary::apply_custom_words(&text, custom_words);
    dictionary::apply_replacements(&text, replacements, case_sensitive_replacements)
}

/// `None` when the session was cancelled mid-pipeline.
pub fn process(app: &AppHandle, session: u64, transcription: &Transcription) -> Option<PipelineOutput> {
    let settings = get_settings(app);
    let context = context::current();
    let (local_text, _inserted) = process_local(
        &transcription.text,
        settings.remove_filler_words,
        &settings.custom_words,
        &settings.replacements,
        // The rule follows the setting, not the outcome: matching stays
        // case-sensitive even if the Groq call later fails.
        settings.clean_and_reformat,
    );

    if !crate::actions::is_session_active(session) {
        return None;
    }

    Some(PipelineOutput {
        raw_text: transcription.text.clone(),
        final_text: local_text,
        app_name: context.app_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Replacement;

    #[test]
    fn runs_steps_in_order() {
        let replacements = [Replacement {
            trigger: "my email".into(),
            value: "ken@example.com".into(),
        }];
        let (text, inserted) = process_local(
            "Um, send the kubernets doc to my email",
            true,
            &["Kubernetes".to_string()],
            &replacements,
            false,
        );
        assert_eq!(text, "Send the Kubernetes doc to ken@example.com");
        assert_eq!(inserted, vec!["ken@example.com".to_string()]);
    }

    #[test]
    fn filler_removal_can_be_off() {
        let (text, _) = process_local("um okay", false, &[], &[], false);
        assert_eq!(text, "um okay");
    }
}
