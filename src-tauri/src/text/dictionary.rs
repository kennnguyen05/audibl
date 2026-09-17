//! Dictionary: custom-word correction and replacements.
//!
//! Custom words: Unicode-aware fuzzy correction (NFC, Levenshtein distance
//! over characters ≤ 18% of the longer key, 1–3-word n-grams, keys of at least
//! 4 characters). Handy's version (`text.rs`, MIT) skips non-ASCII words; this
//! one keeps diacritics, so "Nguyen" can become "Nguyễn".
//!
//! Real words are protected: a single common English word (SCOWL levels
//! 10–20, `common_words_en.txt`) is never changed, not even recased, so
//! "right" stays "right" with custom "Wright" and "apple" stays "apple" with
//! "Apple"; the Groq step decides those from context. A single word with
//! non-ASCII letters (Vietnamese) is only normalized to an exact case-insensitive
//! match, so "nguyên" stays but "nguyễn" becomes "Nguyễn". Multi-word n-grams
//! ("charge bee" → "ChargeBee") are corrected even when each word is common, but
//! only if every word is needed: "on cloud flare" keeps "on".
//!
//! Replacements: whole-phrase matches, longest trigger first. Case-insensitive
//! unless Clean and Reformat is on. Inserted values are always kept exactly as
//! typed and are returned so the Groq step can require them verbatim.

use crate::settings::Replacement;
use regex::{Captures, Regex};
use std::collections::HashSet;
use std::sync::OnceLock;
use unicode_normalization::UnicodeNormalization;

const MAX_DISTANCE_RATIO: f64 = 0.18;
const MIN_KEY_CHARS: usize = 4;
const MAX_NGRAM: usize = 3;

/// Lowercase ASCII, 4+ letters, one per line; see THIRD_PARTY_NOTICES.md.
const COMMON_WORDS_EN: &str = include_str!("common_words_en.txt");

fn common_words_en() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| COMMON_WORDS_EN.lines().filter(|l| !l.is_empty()).collect())
}

/// Whether a lowercase word is in the embedded common English word list.
pub fn is_common_english(word: &str) -> bool {
    common_words_en().contains(word)
}

/// Whether a single word (its `match_key`) must be left alone when it matches
/// a custom word at `ratio`: a common English word is never changed, not even
/// recased; a word with non-ASCII letters (a Vietnamese syllable, which ASR
/// only emits for real syllables) is only normalized to an exact match.
fn is_protected_word(key: &str, ratio: f64) -> bool {
    common_words_en().contains(key) || (!key.is_ascii() && ratio > 0.0)
}

fn ngram_key(words: &[&str]) -> String {
    words.iter().map(|w| match_key(w)).collect()
}

pub fn nfc(text: &str) -> String {
    text.nfc().collect()
}

/// Lowercase NFC letters and digits only: "Charge B," → "chargeb".
fn match_key(text: &str) -> String {
    text.nfc()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Splits leading and trailing punctuation off a whitespace-delimited token.
fn split_punctuation(word: &str) -> (&str, &str, &str) {
    let start = word
        .char_indices()
        .find(|(_, c)| c.is_alphanumeric())
        .map(|(i, _)| i)
        .unwrap_or(word.len());
    let end = word
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_alphanumeric())
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(start);
    if start >= end {
        return (word, "", "");
    }
    (&word[..start], &word[start..end], &word[end..])
}

fn distance_ratio(a: &str, b: &str) -> f64 {
    let max_len = a.chars().count().max(b.chars().count());
    if max_len == 0 {
        return 1.0;
    }
    strsim::levenshtein(a, b) as f64 / max_len as f64
}

pub fn apply_custom_words(text: &str, custom_words: &[String]) -> String {
    let keys: Vec<(String, &String)> = custom_words
        .iter()
        .map(|w| (match_key(w), w))
        .filter(|(k, _)| k.chars().count() >= MIN_KEY_CHARS)
        .collect();
    if keys.is_empty() {
        return text.to_string();
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    let mut out: Vec<String> = Vec::with_capacity(words.len());
    let mut i = 0;
    while i < words.len() {
        // Best (lowest ratio) match over 1–3 word n-grams starting here.
        let mut best: Option<(usize, &String, f64)> = None;
        for n in 1..=MAX_NGRAM.min(words.len() - i) {
            let ngram = &words[i..i + n];
            // Never join across punctuation: "Charge B, che" stops at "B,".
            if ngram[..n - 1]
                .iter()
                .any(|w| !split_punctuation(w).2.is_empty())
            {
                break;
            }
            let candidate = ngram_key(ngram);
            if candidate.chars().count() < MIN_KEY_CHARS {
                continue;
            }
            for (key, word) in &keys {
                let ratio = distance_ratio(&candidate, key);
                if ratio > MAX_DISTANCE_RATIO || best.is_some_and(|(_, _, r)| ratio >= r) {
                    continue;
                }
                if n == 1 && is_protected_word(&candidate, ratio) {
                    continue;
                }
                // Every word must belong to the term: "on cloud flare" is
                // "on" + "Cloudflare", not "Cloudflare".
                if n > 1
                    && (distance_ratio(&ngram_key(&ngram[1..]), key) <= ratio
                        || distance_ratio(&ngram_key(&ngram[..n - 1]), key) <= ratio)
                {
                    continue;
                }
                best = Some((n, word, ratio));
            }
        }

        match best {
            Some((n, word, _)) => {
                let (prefix, _, _) = split_punctuation(words[i]);
                let (_, _, suffix) = split_punctuation(words[i + n - 1]);
                out.push(format!("{prefix}{word}{suffix}"));
                i += n;
            }
            None => {
                out.push(words[i].to_string());
                i += 1;
            }
        }
    }
    out.join(" ")
}

/// Returns the text with replacements applied and the values inserted, in
/// order of first insertion.
pub fn apply_replacements(
    text: &str,
    replacements: &[Replacement],
    case_sensitive: bool,
) -> (String, Vec<String>) {
    let mut rows: Vec<(String, &str)> = replacements
        .iter()
        .map(|r| (nfc(r.trigger.trim()), r.value.as_str()))
        .filter(|(trigger, _)| !trigger.is_empty())
        .collect();
    if rows.is_empty() {
        return (text.to_string(), Vec::new());
    }
    // Leftmost-first alternation: listing longer triggers first makes
    // "my email address" win over "my email" at the same position.
    rows.sort_by_key(|(trigger, _)| std::cmp::Reverse(trigger.chars().count()));

    let alternation = rows
        .iter()
        .enumerate()
        .map(|(index, (trigger, _))| {
            let first_is_word = trigger.chars().next().is_some_and(char::is_alphanumeric);
            let last_is_word = trigger.chars().last().is_some_and(char::is_alphanumeric);
            format!(
                "{}(?P<r{index}>{}){}",
                if first_is_word { r"\b" } else { "" },
                regex::escape(trigger),
                if last_is_word { r"\b" } else { "" },
            )
        })
        .collect::<Vec<_>>()
        .join("|");
    let flags = if case_sensitive { "" } else { "(?i)" };
    let Ok(pattern) = Regex::new(&format!("{flags}(?:{alternation})")) else {
        return (text.to_string(), Vec::new());
    };

    let mut inserted: Vec<String> = Vec::new();
    let text = nfc(text);
    let result = pattern.replace_all(&text, |caps: &Captures| {
        let index = (0..rows.len())
            .find(|i| caps.name(&format!("r{i}")).is_some())
            .expect("one alternative matched");
        let value = rows[index].1.to_string();
        if !inserted.contains(&value) {
            inserted.push(value.clone());
        }
        value
    });
    (result.into_owned(), inserted)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(list: &[&str]) -> Vec<String> {
        list.iter().map(|w| w.to_string()).collect()
    }

    fn rep(trigger: &str, value: &str) -> Replacement {
        Replacement {
            trigger: trigger.into(),
            value: value.into(),
        }
    }

    // ---- custom words ------------------------------------------------------

    #[test]
    fn corrects_close_spelling() {
        assert_eq!(
            apply_custom_words("I use kubernets daily", &words(&["Kubernetes"])),
            "I use Kubernetes daily"
        );
    }

    #[test]
    fn corrects_multi_word_ngram_and_keeps_punctuation() {
        assert_eq!(
            apply_custom_words("We pay with charge bee.", &words(&["ChargeBee"])),
            "We pay with ChargeBee."
        );
    }

    #[test]
    fn corrects_vietnamese_diacritics() {
        assert_eq!(
            apply_custom_words("anh Nguyen gọi", &words(&["Nguyễn"])),
            "anh Nguyễn gọi"
        );
    }

    #[test]
    fn short_keys_are_never_fuzzy_matched() {
        assert_eq!(
            apply_custom_words("the cat sat", &words(&["Cab"])),
            "the cat sat"
        );
    }

    #[test]
    fn distant_words_are_left_alone() {
        assert_eq!(
            apply_custom_words("the cloud is down", &words(&["Claude"])),
            "the cloud is down"
        );
    }

    #[test]
    fn common_word_is_not_replaced_by_close_custom_word() {
        let text = "Check to see if the spelling is right.";
        assert_eq!(apply_custom_words(text, &words(&["Wright"])), text);
    }

    #[test]
    fn common_word_is_not_recased() {
        assert_eq!(
            apply_custom_words("an apple a day", &words(&["Apple"])),
            "an apple a day"
        );
    }

    #[test]
    fn uncommon_exact_word_is_recased() {
        assert_eq!(
            apply_custom_words("mr wright called", &words(&["Wright"])),
            "mr Wright called"
        );
    }

    #[test]
    fn multi_word_ngram_of_common_words_is_corrected() {
        assert_eq!(
            apply_custom_words(
                "open visual studio code now",
                &words(&["Visual Studio Code"])
            ),
            "open Visual Studio Code now"
        );
        assert_eq!(
            apply_custom_words("deploy on cloud flare", &words(&["Cloudflare"])),
            "deploy on Cloudflare"
        );
    }

    #[test]
    fn vietnamese_syllable_is_not_replaced() {
        // "nguyên" is a real word, one diacritic away from "Nguyễn".
        assert_eq!(
            apply_custom_words("số nguyên dương", &words(&["Nguyễn"])),
            "số nguyên dương"
        );
    }

    #[test]
    fn common_word_list_is_well_formed() {
        assert!(common_words_en().len() > 5000);
        assert!(common_words_en().contains("right"));
        assert!(!common_words_en().contains("wright"));
        assert!(COMMON_WORDS_EN
            .lines()
            .all(|l| l.len() >= MIN_KEY_CHARS && l.bytes().all(|b| b.is_ascii_lowercase())));
    }

    #[test]
    fn does_not_join_across_punctuation() {
        assert_eq!(
            apply_custom_words("Charge, bee", &words(&["ChargeBee"])),
            "Charge, bee"
        );
    }

    #[test]
    fn nfd_input_matches_nfc_word() {
        let decomposed: String = "Nguyễn".nfd().collect();
        assert_eq!(
            apply_custom_words(&decomposed, &words(&["Nguyễn"])),
            "Nguyễn"
        );
    }

    // ---- replacements ------------------------------------------------------

    const EMAIL: &str = "kennnguyen0507@gmail.com";

    #[test]
    fn replaces_whole_phrase_only() {
        let rows = [rep("my email", EMAIL)];
        let (text, inserted) = apply_replacements("send it to my email please", &rows, false);
        assert_eq!(text, format!("send it to {EMAIL} please"));
        assert_eq!(inserted, vec![EMAIL.to_string()]);

        let (text, inserted) = apply_replacements("send it to my emails", &rows, false);
        assert_eq!(text, "send it to my emails");
        assert!(inserted.is_empty());
    }

    #[test]
    fn longest_trigger_wins() {
        let rows = [rep("my email", EMAIL), rep("my email address", "ADDRESS")];
        let (text, _) = apply_replacements("my email address is", &rows, false);
        assert_eq!(text, "ADDRESS is");
    }

    #[test]
    fn vietnamese_trigger_matches() {
        let rows = [rep("địa chỉ nhà", "12 Lý Thường Kiệt")];
        let (text, _) = apply_replacements("gửi về địa chỉ nhà nhé", &rows, false);
        assert_eq!(text, "gửi về 12 Lý Thường Kiệt nhé");
    }

    #[test]
    fn case_insensitive_when_clean_is_off() {
        let rows = [rep("my email", EMAIL)];
        let (text, _) = apply_replacements("My email is here. Write to MY EMAIL.", &rows, false);
        assert_eq!(text, format!("{EMAIL} is here. Write to {EMAIL}."));
    }

    #[test]
    fn case_sensitive_when_clean_is_on() {
        let rows = [rep("my email", EMAIL)];
        let (text, inserted) =
            apply_replacements("My email is here. Write to my email.", &rows, true);
        assert_eq!(text, format!("My email is here. Write to {EMAIL}."));
        assert_eq!(inserted, vec![EMAIL.to_string()]);
    }

    #[test]
    fn value_is_inserted_verbatim() {
        let rows = [rep("sig", "Best,\n$1 Ken")];
        let (text, _) = apply_replacements("sig", &rows, false);
        assert_eq!(text, "Best,\n$1 Ken");
    }

    #[test]
    fn trigger_with_symbol_edges_matches() {
        let rows = [rep("@home", "123 Main St")];
        let (text, _) = apply_replacements("ship to @home.", &rows, false);
        assert_eq!(text, "ship to 123 Main St.");
    }

    #[test]
    fn no_replacements_is_identity() {
        let (text, inserted) = apply_replacements("hello", &[], true);
        assert_eq!(text, "hello");
        assert!(inserted.is_empty());
    }
}
