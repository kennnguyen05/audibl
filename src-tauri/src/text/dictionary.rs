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
//! A custom word that holds a symbol is also matched as it is spoken, since the
//! recognizer writes the symbol out as a word: "CLAUDE.md" is reached both by
//! "claude.md" and by "cloud dot md".
//!
//! Replacements: whole-phrase matches, longest trigger first, always
//! case-insensitive — the speaker's capitalization is an accident of the
//! recognizer (and of the filler step, which capitalizes the first word), never
//! an instruction. Punctuation and spacing inside a trigger still have to
//! match. Inserted values are always kept exactly as typed and are returned so
//! the Groq step can require them verbatim.

use crate::settings::Replacement;
use regex::{Captures, Regex};
use std::collections::HashSet;
use std::sync::OnceLock;
use unicode_normalization::UnicodeNormalization;

const MAX_DISTANCE_RATIO: f64 = 0.18;
/// Used instead of `MAX_DISTANCE_RATIO` for a term dictated with its
/// punctuation spoken out ("cloud dot md"), where the symbol word is an anchor
/// the ordinary path does not have.
const SPOKEN_MAX_DISTANCE_RATIO: f64 = 0.34;
/// Below this length a word inside a spoken form has to match exactly. At three
/// or four characters `SPOKEN_MAX_DISTANCE_RATIO` buys a whole substituted
/// syllable, which is a different word rather than a misheard one — "ben at
/// sample dot com" is somebody else's address, not Ken's. "cloud" for "claude"
/// is six characters at its longer end, so the case the loose ratio exists for
/// is untouched.
const SPOKEN_LOOSE_MIN_CHARS: usize = 6;
const MIN_KEY_CHARS: usize = 4;
const MAX_NGRAM: usize = 3;
/// A spoken form may be longer than `MAX_NGRAM` — "ken@example.com" is five
/// words — and its exact word count plus its exact symbol words make the extra
/// span safe. This caps how far that can reach.
const MAX_SPOKEN_NGRAM: usize = 6;

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

/// How a symbol inside a custom word is dictated. "CLAUDE.md" is spoken
/// "claude dot md", which `match_key` alone can never reach: its key is
/// "claudemd" and the three spoken words key as "clouddotmd".
const SPOKEN_SYMBOLS: &[(char, &str)] = &[
    ('.', "dot"),
    ('@', "at"),
    ('/', "slash"),
    ('-', "dash"),
    ('_', "underscore"),
    ('+', "plus"),
    ('#', "hash"),
];

/// The match keys of a custom word said out loud, one per spoken word, or
/// `None` when the word holds no symbol and is simply said as it is written.
fn spoken_tokens(word: &str) -> Option<Vec<String>> {
    let mut spoken = String::with_capacity(word.len());
    let mut found = false;
    for c in word.nfc() {
        match SPOKEN_SYMBOLS.iter().find(|(symbol, _)| *symbol == c) {
            Some((_, said)) => {
                found = true;
                spoken.push(' ');
                spoken.push_str(said);
                spoken.push(' ');
            }
            None => spoken.push(c),
        }
    }
    if !found {
        return None;
    }
    let tokens: Vec<String> = spoken.split_whitespace().map(match_key).collect();
    (tokens.len() > 1).then_some(tokens)
}

/// Whether a spoken form is indistinguishable from ordinary speech: every word
/// beside the symbol is a common English word. "@home" is said "at home", and
/// "I work at home" is not a dictation of the term, so that form is never
/// registered — the same protection `is_protected_word` gives the written form,
/// which keys "@home" as "home" and leaves it alone. The anchor is only
/// evidence when a word beside it is not plain English: "claude" in "cloud dot
/// md", "ken" and "com" in "ken at example dot com". Groq still sees the term
/// as a preferred spelling either way.
fn is_ordinary_speech(form: &[String]) -> bool {
    form.iter()
        .filter(|token| !SPOKEN_SYMBOLS.iter().any(|(_, said)| said == *token))
        .all(|token| is_common_english(token))
}

/// One way a custom word can arrive in the transcript.
enum Form {
    /// As written, as a single match key: "ChargeBee" is "chargebee", and the
    /// n-gram's own words are joined before they are compared, so "charge bee"
    /// reaches it.
    Written(String),
    /// As dictated, one key per spoken word: "CLAUDE.md" is
    /// ["claude", "dot", "md"].
    Spoken(Vec<String>),
}

/// How far an n-gram is from a spoken form, 0.0 for a perfect match, `None`
/// when it is not that term at all.
///
/// Words are compared one to one, and the spoken symbol has to be there
/// exactly. That anchor is strong enough to be worth a looser threshold on the
/// words around it: nobody says "cloud dot md" about a cloud, and "cloud" is
/// two edits from "claude", which the ordinary threshold would never allow.
fn spoken_ratio(tokens: &[String], form: &[String]) -> Option<f64> {
    if tokens.len() != form.len() {
        return None;
    }
    let mut worst = 0.0_f64;
    for (spoken, expected) in tokens.iter().zip(form) {
        if SPOKEN_SYMBOLS.iter().any(|(_, said)| said == expected) {
            if spoken != expected {
                return None;
            }
            continue;
        }
        let ratio = distance_ratio(spoken, expected);
        if ratio > 0.0
            && spoken.chars().count().max(expected.chars().count()) < SPOKEN_LOOSE_MIN_CHARS
        {
            return None;
        }
        worst = worst.max(ratio);
    }
    (worst <= SPOKEN_MAX_DISTANCE_RATIO).then_some(worst)
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
    // One entry per spoken form of a custom word: its words' match keys, and
    // the word to write. "CLAUDE.md" has two forms, ["claudemd"] (said as one
    // word) and ["claude", "dot", "md"]; "Kubernetes" has only the first.
    let mut forms: Vec<(Form, &String)> = Vec::new();
    for word in custom_words {
        let key = match_key(word);
        if key.chars().count() < MIN_KEY_CHARS {
            continue;
        }
        forms.push((Form::Written(key), word));
        if let Some(spoken) = spoken_tokens(word) {
            if !is_ordinary_speech(&spoken) {
                forms.push((Form::Spoken(spoken), word));
            }
        }
    }
    if forms.is_empty() {
        return text.to_string();
    }

    // Long enough for the longest spoken form, since that form only matches an
    // n-gram of exactly its own length.
    let max_ngram = forms
        .iter()
        .filter_map(|(form, _)| match form {
            Form::Spoken(tokens) => Some(tokens.len()),
            Form::Written(_) => None,
        })
        .max()
        .unwrap_or(0)
        .clamp(MAX_NGRAM, MAX_SPOKEN_NGRAM);

    let words: Vec<&str> = text.split_whitespace().collect();
    let mut out: Vec<String> = Vec::with_capacity(words.len());
    let mut i = 0;
    while i < words.len() {
        // Best (lowest ratio) match over the n-grams starting here.
        let mut best: Option<(usize, &String, f64)> = None;
        for n in 1..=max_ngram.min(words.len() - i) {
            let ngram = &words[i..i + n];
            // Never join across punctuation: "Charge B, che" stops at "B,".
            if ngram[..n - 1]
                .iter()
                .any(|w| !split_punctuation(w).2.is_empty())
            {
                break;
            }
            let tokens: Vec<String> = ngram.iter().map(|w| match_key(w)).collect();
            if tokens.iter().map(|t| t.chars().count()).sum::<usize>() < MIN_KEY_CHARS {
                continue;
            }
            let candidate = tokens.concat();
            for (form, word) in &forms {
                let ratio = match form {
                    Form::Written(key) => {
                        let ratio = distance_ratio(&candidate, key);
                        if n > MAX_NGRAM || ratio > MAX_DISTANCE_RATIO {
                            continue;
                        }
                        ratio
                    }
                    // A spoken form uses one word per token, so "every word
                    // belongs" is already true of any match.
                    Form::Spoken(spoken) => match spoken_ratio(&tokens, spoken) {
                        Some(ratio) => ratio,
                        None => continue,
                    },
                };
                if best.is_some_and(|(_, _, r)| ratio >= r) {
                    continue;
                }
                if n == 1 && is_protected_word(&candidate, ratio) {
                    continue;
                }
                // Every word must belong to the term: "on cloud flare" is
                // "on" + "Cloudflare", not "Cloudflare".
                if n > 1 {
                    if let Form::Written(key) = form {
                        if distance_ratio(&tokens[1..].concat(), key) <= ratio
                            || distance_ratio(&tokens[..n - 1].concat(), key) <= ratio
                        {
                            continue;
                        }
                    }
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

/// One named alternative of a whole-phrase pattern. `\b` is only useful where
/// the phrase's own edge is a word character: "@home" has to match after a
/// space, where `\b` would not hold.
fn alternative(index: usize, phrase: &str) -> String {
    let first_is_word = phrase.chars().next().is_some_and(char::is_alphanumeric);
    let last_is_word = phrase.chars().last().is_some_and(char::is_alphanumeric);
    format!(
        "{}(?P<r{index}>{}){}",
        if first_is_word { r"\b" } else { "" },
        regex::escape(phrase),
        if last_is_word { r"\b" } else { "" },
    )
}

/// Which alternative of `alternative`'s pattern matched.
fn matched_index(caps: &Captures, count: usize) -> usize {
    (0..count)
        .find(|i| caps.name(&format!("r{i}")).is_some())
        .expect("one alternative matched")
}

/// Rewrites every whole-phrase occurrence of a custom word to the spelling the
/// user typed, ignoring case. This is the last word on a custom word's
/// capitalization: the Groq step is told to keep it, but nothing stops the
/// model from writing "Claude.md" for "CLAUDE.md", and re-casing a term the
/// user spelled out is never what they meant.
///
/// A single common English word is left alone, exactly as in
/// `apply_custom_words`: with "Apple" in the dictionary, "an apple a day"
/// keeps its fruit.
///
/// `keep_verbatim` holds the replacement values already inserted, which were
/// written exactly as the user typed them and must survive this pass whole: a
/// custom word "Audibl" may not turn the value "audibl.app" into "Audibl.app".
/// They are matched first, so where both could match the value wins.
pub fn apply_dictionary_spelling(
    text: &str,
    custom_words: &[String],
    keep_verbatim: &[String],
) -> String {
    let mut rows: Vec<String> = custom_words
        .iter()
        .map(|w| nfc(w.trim()))
        .filter(|w| {
            let key = match_key(w);
            key.chars().count() >= MIN_KEY_CHARS
                && !(w.split_whitespace().count() == 1 && is_common_english(&key))
        })
        .collect();
    if rows.is_empty() {
        return text.to_string();
    }
    rows.sort_by_key(|w| std::cmp::Reverse(w.chars().count()));

    let mut guards: Vec<String> = keep_verbatim
        .iter()
        .map(|value| nfc(value.trim()))
        .filter(|value| !value.is_empty())
        .collect();
    guards.sort_by_key(|value| std::cmp::Reverse(value.chars().count()));

    // Guards first: the alternation is leftmost-first, so at a position where a
    // value and a custom word both start, the whole value matches and is kept.
    let phrases: Vec<&String> = guards.iter().chain(rows.iter()).collect();
    let alternation = phrases
        .iter()
        .enumerate()
        .map(|(index, phrase)| alternative(index, phrase))
        .collect::<Vec<_>>()
        .join("|");
    let Ok(pattern) = Regex::new(&format!("(?i)(?:{alternation})")) else {
        return text.to_string();
    };
    pattern
        .replace_all(text, |caps: &Captures| {
            match matched_index(caps, phrases.len()).checked_sub(guards.len()) {
                Some(row) => rows[row].clone(),
                None => caps[0].to_string(),
            }
        })
        .into_owned()
}

/// Returns the text with replacements applied and the values inserted, in
/// order of first insertion.
pub fn apply_replacements(text: &str, replacements: &[Replacement]) -> (String, Vec<String>) {
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
        .map(|(index, (trigger, _))| alternative(index, trigger))
        .collect::<Vec<_>>()
        .join("|");
    let Ok(pattern) = Regex::new(&format!("(?i)(?:{alternation})")) else {
        return (text.to_string(), Vec::new());
    };

    let mut inserted: Vec<String> = Vec::new();
    let text = nfc(text);
    let result = pattern.replace_all(&text, |caps: &Captures| {
        let value = rows[matched_index(caps, rows.len())].1.to_string();
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
    fn corrects_spoken_symbol_form() {
        // Ken's case: the recognizer hears "CLAUDE.md" as three words.
        assert_eq!(
            apply_custom_words("Add the change to my cloud dot md.", &words(&["CLAUDE.md"])),
            "Add the change to my CLAUDE.md."
        );
        // The written form still matches on its own.
        assert_eq!(
            apply_custom_words("open claude.md", &words(&["CLAUDE.md"])),
            "open CLAUDE.md"
        );
    }

    #[test]
    fn spoken_symbol_form_does_not_swallow_neighbours() {
        // "dot" on its own is not part of the term.
        assert_eq!(
            apply_custom_words("the dot is red", &words(&["CLAUDE.md"])),
            "the dot is red"
        );
    }

    #[test]
    fn spoken_symbol_form_does_not_loosen_the_bare_word() {
        // Ken's real dictionary. The looser threshold only applies with the
        // spoken symbol present, so "cloud" is still a cloud.
        let list = words(&["Claude", "CLAUDE.md"]);
        assert_eq!(
            apply_custom_words("the cloud is down", &list),
            "the cloud is down"
        );
    }

    #[test]
    fn spoken_email_form_is_corrected() {
        assert_eq!(
            apply_custom_words(
                "write to ken at example dot com",
                &words(&["ken@example.com"])
            ),
            "write to ken@example.com"
        );
    }

    #[test]
    fn spoken_form_of_ordinary_words_is_not_registered() {
        // "@home" is said "at home", which is also just English. The term is
        // left to Groq rather than rewriting the middle of a sentence.
        assert_eq!(
            apply_custom_words("I will work at home today", &words(&["@home"])),
            "I will work at home today"
        );
        // Written out, it is still corrected.
        assert_eq!(
            apply_custom_words("ship it to home.", &words(&["@home"])),
            "ship it to home."
        );
    }

    #[test]
    fn spoken_form_does_not_rewrite_short_words() {
        // Somebody else's address: "ben" is not a misheard "ken".
        assert_eq!(
            apply_custom_words(
                "the ben at sample dot com",
                &words(&["ken@example.com"])
            ),
            "the ben at sample dot com"
        );
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

    // ---- dictionary spelling -----------------------------------------------

    #[test]
    fn dictionary_spelling_leaves_replacement_values_whole() {
        // The value was inserted exactly as typed; a custom word may not
        // re-case its insides.
        assert_eq!(
            apply_dictionary_spelling(
                "we are at audibl.app",
                &words(&["Audibl"]),
                &words(&["audibl.app"])
            ),
            "we are at audibl.app"
        );
        // Outside a value the same word is still spelled the user's way.
        assert_eq!(
            apply_dictionary_spelling(
                "audibl lives at audibl.app",
                &words(&["Audibl"]),
                &words(&["audibl.app"])
            ),
            "Audibl lives at audibl.app"
        );
    }

    #[test]
    fn dictionary_spelling_is_restored() {
        let list = words(&["CLAUDE.md", "ChargeBee"]);
        assert_eq!(
            apply_dictionary_spelling("Add it to Claude.md and chargebee.", &list, &[]),
            "Add it to CLAUDE.md and ChargeBee."
        );
    }

    #[test]
    fn dictionary_spelling_leaves_common_words_alone() {
        assert_eq!(
            apply_dictionary_spelling("an apple a day", &words(&["Apple"]), &[]),
            "an apple a day"
        );
        assert_eq!(
            apply_dictionary_spelling("the spelling is right", &words(&["Wright"]), &[]),
            "the spelling is right"
        );
    }

    #[test]
    fn dictionary_spelling_matches_whole_phrases_only() {
        assert_eq!(
            apply_dictionary_spelling("unclaudely claude", &words(&["Claude"]), &[]),
            "unclaudely Claude"
        );
        assert_eq!(
            apply_dictionary_spelling(
                "open visual studio code",
                &words(&["Visual Studio Code"]),
                &[]
            ),
            "open Visual Studio Code"
        );
    }

    // ---- replacements ------------------------------------------------------

    const EMAIL: &str = "kennnguyen0507@gmail.com";

    #[test]
    fn replaces_whole_phrase_only() {
        let rows = [rep("my email", EMAIL)];
        let (text, inserted) = apply_replacements("send it to my email please", &rows);
        assert_eq!(text, format!("send it to {EMAIL} please"));
        assert_eq!(inserted, vec![EMAIL.to_string()]);

        let (text, inserted) = apply_replacements("send it to my emails", &rows);
        assert_eq!(text, "send it to my emails");
        assert!(inserted.is_empty());
    }

    #[test]
    fn longest_trigger_wins() {
        let rows = [rep("my email", EMAIL), rep("my email address", "ADDRESS")];
        let (text, _) = apply_replacements("my email address is", &rows);
        assert_eq!(text, "ADDRESS is");
    }

    #[test]
    fn vietnamese_trigger_matches() {
        let rows = [rep("địa chỉ nhà", "12 Lý Thường Kiệt")];
        let (text, _) = apply_replacements("gửi về địa chỉ nhà nhé", &rows);
        assert_eq!(text, "gửi về 12 Lý Thường Kiệt nhé");
    }

    #[test]
    fn case_is_always_ignored() {
        let rows = [rep("my email", EMAIL)];
        let (text, _) = apply_replacements("My email is here. Write to MY EMAIL.", &rows);
        assert_eq!(text, format!("{EMAIL} is here. Write to {EMAIL}."));
    }

    #[test]
    fn uppercase_trigger_matches_any_case() {
        // Ken's case: the trigger is typed "my IG", the filler step capitalizes
        // the first word, and the recognizer's casing is its own guess.
        let rows = [rep("my IG", "@hkhang.exe")];
        for text in ["My IG is here.", "my ig is here.", "my Ig is here."] {
            let (out, inserted) = apply_replacements(text, &rows);
            assert_eq!(out, "@hkhang.exe is here.", "for {text:?}");
            assert_eq!(inserted, vec!["@hkhang.exe".to_string()]);
        }
    }

    #[test]
    fn value_is_inserted_verbatim() {
        let rows = [rep("sig", "Best,\n$1 Ken")];
        let (text, _) = apply_replacements("sig", &rows);
        assert_eq!(text, "Best,\n$1 Ken");
    }

    #[test]
    fn trigger_with_symbol_edges_matches() {
        let rows = [rep("@home", "123 Main St")];
        let (text, _) = apply_replacements("ship to @home.", &rows);
        assert_eq!(text, "ship to 123 Main St.");
    }

    #[test]
    fn no_replacements_is_identity() {
        let (text, inserted) = apply_replacements("hello", &[]);
        assert_eq!(text, "hello");
        assert!(inserted.is_empty());
    }
}
