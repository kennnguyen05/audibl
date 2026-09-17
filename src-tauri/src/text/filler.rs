//! Remove Filler Words: English and Vietnamese fillers plus stutters.
//!
//! Both lists always apply, so code-switched speech is cleaned too. "à" and
//! "ừ" are not fillers: they are meaningful Vietnamese particles ("yes",
//! question tag). Stutter collapse is ported from Handy (`text.rs`, MIT).

use once_cell::sync::Lazy;
use regex::Regex;

const ENGLISH_FILLERS: &[&str] = &["um", "uh", "uhm", "umm", "er", "ah", "hmm"];
const VIETNAMESE_FILLERS: &[&str] = &["ờ", "ờm", "ừm", "ưm", "ơ", "hừm"];

/// One alternation, longest first, with a trailing comma/period and the
/// spaces around it, so "So, um, yeah" becomes "So, yeah".
static FILLER_PATTERN: Lazy<Regex> = Lazy::new(|| {
    let mut words: Vec<&str> = ENGLISH_FILLERS
        .iter()
        .chain(VIETNAMESE_FILLERS)
        .copied()
        .collect();
    words.sort_by_key(|w| std::cmp::Reverse(w.chars().count()));
    let alternation = words
        .iter()
        .map(|w| regex::escape(w))
        .collect::<Vec<_>>()
        .join("|");
    Regex::new(&format!(r"(?i)\b(?:{alternation})\b[,.]?")).unwrap()
});

static MULTI_SPACE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[ \t]{2,}").unwrap());
static SPACE_BEFORE_PUNCT: Lazy<Regex> = Lazy::new(|| Regex::new(r"[ \t]+([,.!?;:])").unwrap());
static REPEATED_COMMA: Lazy<Regex> = Lazy::new(|| Regex::new(r",(\s*,)+").unwrap());
static LEADING_PUNCT: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[\s,.;:]+").unwrap());

pub fn remove_filler_words(text: &str) -> String {
    let without_fillers = FILLER_PATTERN.replace_all(text, "");
    let collapsed = collapse_stutters(&without_fillers);
    tidy(&collapsed)
}

/// A word repeated three or more times in a row collapses to one: "I I I think"
/// becomes "I think". Two repetitions are kept ("very very").
pub fn collapse_stutters(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut result: Vec<&str> = Vec::with_capacity(words.len());
    let mut i = 0;
    while i < words.len() {
        let word = words[i];
        let lower = word.to_lowercase();
        let mut count = 1;
        if lower.chars().all(char::is_alphabetic) {
            while i + count < words.len() && words[i + count].to_lowercase() == lower {
                count += 1;
            }
        }
        result.push(word);
        i += if count >= 3 { count } else { 1 };
    }
    result.join(" ")
}

fn tidy(text: &str) -> String {
    let text = REPEATED_COMMA.replace_all(text, ",");
    let text = SPACE_BEFORE_PUNCT.replace_all(&text, "$1");
    let text = MULTI_SPACE.replace_all(&text, " ");
    let text = LEADING_PUNCT.replace(&text, "");
    capitalize_first(text.trim())
}

/// Removing a leading filler ("Um, so we…") can leave a lowercase start.
fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) if first.is_lowercase() => first.to_uppercase().chain(chars).collect(),
        _ => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_english_fillers() {
        assert_eq!(
            remove_filler_words("Um, I think, uh, we should ship it."),
            "I think, we should ship it."
        );
        assert_eq!(remove_filler_words("So, um, yeah"), "So, yeah");
        assert_eq!(remove_filler_words("hmm let me er check"), "Let me check");
    }

    #[test]
    fn removes_vietnamese_fillers() {
        assert_eq!(
            remove_filler_words("Ờ, em nghĩ là ừm mình nên làm vậy."),
            "Em nghĩ là mình nên làm vậy."
        );
        assert_eq!(remove_filler_words("ơ anh ơi"), "Anh ơi");
        assert_eq!(remove_filler_words("hừm để em xem"), "Để em xem");
    }

    #[test]
    fn keeps_vietnamese_particles_a_and_u() {
        assert_eq!(remove_filler_words("Anh đi rồi à?"), "Anh đi rồi à?");
        assert_eq!(remove_filler_words("Ừ, em biết rồi."), "Ừ, em biết rồi.");
    }

    #[test]
    fn does_not_touch_words_containing_fillers() {
        assert_eq!(
            remove_filler_words("The umbrella is here, err no."),
            "The umbrella is here, err no."
        );
        assert_eq!(remove_filler_words("thơ mới"), "Thơ mới");
    }

    #[test]
    fn collapses_three_or_more_repeats() {
        assert_eq!(collapse_stutters("I I I think so"), "I think so");
        assert_eq!(collapse_stutters("em em em em đi"), "em đi");
        assert_eq!(collapse_stutters("very very good"), "very very good");
    }

    #[test]
    fn stutter_collapse_ignores_case() {
        assert_eq!(collapse_stutters("The the THE end"), "The end");
    }

    #[test]
    fn filler_removal_also_collapses_stutters() {
        assert_eq!(remove_filler_words("um so so so yes"), "So yes");
    }
}
