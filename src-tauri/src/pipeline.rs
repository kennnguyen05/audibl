//! Text pipeline after transcription:
//! filler removal (if on) → custom-word correction → replacements → Magic
//! Touch (if on) → dictionary spelling → output for paste and history.
//!
//! The spelling pass runs last, on whichever text won, so a custom word always
//! reaches the user written the way they typed it. It is given the replacement
//! values inserted locally, which it leaves whole: those were already typed by
//! the user and are promised verbatim to the Groq step as well.

use crate::settings::get_settings;
use crate::text::{dictionary, filler};
use crate::transcription::Transcription;
use crate::{cleanup, context};
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
) -> (String, Vec<String>) {
    let mut text = dictionary::nfc(text);
    if remove_fillers {
        text = filler::remove_filler_words(&text);
    }
    text = dictionary::apply_custom_words(&text, custom_words);
    dictionary::apply_replacements(&text, replacements)
}

/// `None` when the session was cancelled mid-pipeline.
pub fn process(
    app: &AppHandle,
    session: u64,
    transcription: &Transcription,
) -> Option<PipelineOutput> {
    let settings = get_settings(app);
    let context = context::current();
    let (local_text, inserted) = process_local(
        &transcription.text,
        settings.remove_filler_words,
        &settings.custom_words,
        &settings.replacements,
    );

    if !crate::actions::is_session_active(session) {
        return None;
    }

    let mut final_text = local_text.clone();
    if settings.clean_and_reformat && !local_text.trim().is_empty() {
        match crate::keychain::get_groq_api_key() {
            Some(key) => {
                crate::overlay::show(app, crate::overlay::OverlayState::Cleaning);
                let input = cleanup::CleanupInput::new(
                    &context,
                    transcription.language,
                    &settings.custom_words,
                    &inserted,
                    &local_text,
                );
                let started = std::time::Instant::now();
                let groq = tauri::async_runtime::block_on(async {
                    tokio::select! {
                        result = cleanup::request(&key, &input) => Some(result),
                        _ = wait_for_cancel(session) => None,
                    }
                })?;
                if !crate::actions::is_session_active(session) {
                    return None;
                }
                let (text, used) = cleanup::choose_output(groq, &local_text, &inserted);
                log::info!(
                    "Groq cleanup {} in {:.2}s",
                    if used {
                        "applied"
                    } else {
                        "skipped (local text kept)"
                    },
                    started.elapsed().as_secs_f32()
                );
                final_text = text;
            }
            None => log::warn!("Magic Touch is on but no Groq key is saved"),
        }
    }

    Some(PipelineOutput {
        raw_text: transcription.text.clone(),
        final_text: dictionary::apply_dictionary_spelling(
            &final_text,
            &settings.custom_words,
            &inserted,
        ),
        app_name: context.app_name,
    })
}

/// Resolves when Esc (or tray Cancel) ends the session, so an in-flight
/// Groq request is dropped instead of waiting for its timeout.
async fn wait_for_cancel(session: u64) {
    while crate::actions::is_session_active(session) {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
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
        );
        assert_eq!(text, "Send the Kubernetes doc to ken@example.com");
        assert_eq!(inserted, vec!["ken@example.com".to_string()]);
    }

    #[test]
    fn filler_removal_can_be_off() {
        let (text, _) = process_local("um okay", false, &[], &[]);
        assert_eq!(text, "um okay");
    }

    /// The filler step capitalizes the first word, so a trigger the user typed
    /// in lowercase (or in caps) has to match whatever case reaches it.
    #[test]
    fn replacement_trigger_survives_capitalization() {
        let replacements = [Replacement {
            trigger: "my IG".into(),
            value: "@hkhang.exe".into(),
        }];
        let (text, _) = process_local("my IG is this", true, &[], &replacements);
        assert_eq!(text, "@hkhang.exe is this");
    }
}
