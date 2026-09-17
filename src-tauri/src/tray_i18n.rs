//! Menu bar (tray) menu strings.
//!
//! Generated at compile time by build.rs from the frontend locale files
//! (`src/i18n/locales/*/translation.json`, `tray` section). The English file
//! defines the fields. The language comes from the `app_language` setting,
//! not from the system locale.

use once_cell::sync::Lazy;
use std::collections::HashMap;

include!(concat!(env!("OUT_DIR"), "/tray_translations.rs"));

pub fn get_tray_translations(language: &str) -> TrayStrings {
    TRANSLATIONS
        .get(language)
        .or_else(|| TRANSLATIONS.get("en"))
        .cloned()
        .expect("English translations must exist")
}

#[cfg(test)]
mod tests {
    use super::get_tray_translations;

    #[test]
    fn unknown_language_falls_back_to_english() {
        assert_eq!(
            get_tray_translations("xx").quit,
            get_tray_translations("en").quit
        );
    }

    #[test]
    fn vietnamese_is_translated() {
        assert_ne!(
            get_tray_translations("vi").open,
            get_tray_translations("en").open
        );
    }
}
