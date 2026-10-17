//! Testing Library's `TextMatch`: strings, substrings, regexes and functions.
use crate::node::Snapshot;
use std::{fmt, rc::Rc};

/// How a query compares text. Strings match exactly after normalization
/// (trimmed, whitespace collapsed), like Testing Library's default.
#[derive(Clone)]
pub struct TextMatch {
    matcher: Matcher,
    normalize: bool,
}

type MatchFn = Rc<dyn Fn(&str, &Snapshot) -> bool>;

#[derive(Clone)]
enum Matcher {
    Exact(String),
    /// Case-insensitive substring, Testing Library's `{ exact: false }`.
    Contains(String),
    #[cfg(feature = "regex")]
    Regex(regex::Regex),
    Function(MatchFn),
}

impl TextMatch {
    /// Matches the whole normalized string.
    pub fn exact(text: impl Into<String>) -> Self {
        Self {
            matcher: Matcher::Exact(text.into()),
            normalize: true,
        }
    }
    /// Matches a case-insensitive substring (`{ exact: false }`).
    pub fn contains(text: impl Into<String>) -> Self {
        Self {
            matcher: Matcher::Contains(text.into()),
            normalize: true,
        }
    }
    /// Matches a regular expression anywhere in the normalized string.
    /// Panics if `pattern` is invalid.
    #[cfg(feature = "regex")]
    pub fn regex(pattern: &str) -> Self {
        regex::Regex::new(pattern)
            .unwrap_or_else(|error| panic!("invalid TextMatch regex {pattern:?}: {error}"))
            .into()
    }
    /// Matches with a function of the text and the element it belongs to.
    pub fn function(matcher: impl Fn(&str, &Snapshot) -> bool + 'static) -> Self {
        Self {
            matcher: Matcher::Function(Rc::new(matcher)),
            normalize: true,
        }
    }
    /// Compares raw text, without trimming or collapsing whitespace.
    pub fn without_normalization(mut self) -> Self {
        self.normalize = false;
        self
    }

    pub fn matches(&self, text: &str, snapshot: &Snapshot) -> bool {
        let normalized;
        let text = if self.normalize {
            normalized = normalize(text);
            normalized.as_str()
        } else {
            text
        };
        match &self.matcher {
            Matcher::Exact(expected) => text == expected,
            Matcher::Contains(expected) => text.to_lowercase().contains(&expected.to_lowercase()),
            #[cfg(feature = "regex")]
            Matcher::Regex(regex) => regex.is_match(text),
            Matcher::Function(matcher) => matcher(text, snapshot),
        }
    }

    pub(crate) fn matches_option(&self, text: Option<&str>, snapshot: &Snapshot) -> bool {
        text.is_some_and(|text| self.matches(text, snapshot))
    }
}

/// Trims and collapses runs of whitespace into single spaces.
pub fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl fmt::Debug for TextMatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.matcher {
            Matcher::Exact(text) => write!(f, "{text:?}"),
            Matcher::Contains(text) => write!(f, "{text:?} (substring, ignoring case)"),
            #[cfg(feature = "regex")]
            Matcher::Regex(regex) => write!(f, "/{}/", regex.as_str()),
            Matcher::Function(_) => f.write_str("<function>"),
        }
    }
}

impl From<&str> for TextMatch {
    fn from(text: &str) -> Self {
        Self::exact(text)
    }
}
impl From<String> for TextMatch {
    fn from(text: String) -> Self {
        Self::exact(text)
    }
}
impl From<&String> for TextMatch {
    fn from(text: &String) -> Self {
        Self::exact(text.clone())
    }
}
#[cfg(feature = "regex")]
impl From<regex::Regex> for TextMatch {
    fn from(regex: regex::Regex) -> Self {
        Self {
            matcher: Matcher::Regex(regex),
            normalize: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Bounds, IntoElement as _, div, prelude::*};
    use proptest::prelude::*;

    fn snapshot() -> Snapshot {
        Snapshot::capture(&div().id("x").into_element(), Vec::new(), Bounds::default())
    }

    #[test]
    fn debug_names_each_matcher() {
        assert_eq!(format!("{:?}", TextMatch::exact("a")), "\"a\"");
        assert_eq!(
            format!("{:?}", TextMatch::contains("a")),
            "\"a\" (substring, ignoring case)"
        );
        assert_eq!(format!("{:?}", TextMatch::regex("^a$")), "/^a$/");
        assert_eq!(
            format!("{:?}", TextMatch::function(|_, _| true)),
            "<function>"
        );
        assert_eq!(format!("{:?}", TextMatch::from(String::from("b"))), "\"b\"");
        assert_eq!(
            format!("{:?}", TextMatch::from(&String::from("c"))),
            "\"c\""
        );
    }

    #[test]
    fn contains_lowercases_rather_than_case_folds() {
        // Like JavaScript's toLowerCase, which Testing Library uses.
        assert!(!TextMatch::contains("STRASSE").matches("Straße", &snapshot()));
        assert!(TextMatch::contains("STRASSE").matches("Strasse", &snapshot()));
    }

    proptest! {
        #[test]
        fn normalize_is_idempotent(text in "\\PC*") {
            let once = normalize(&text);
            prop_assert_eq!(normalize(&once), once);
        }

        #[test]
        fn exact_matches_text_up_to_whitespace(text in "\\PC*") {
            prop_assert!(TextMatch::exact(normalize(&text)).matches(&text, &snapshot()));
        }

        #[test]
        fn contains_finds_any_substring_in_any_case(
            before in "\\PC{0,4}",
            needle in "[^\\s]{0,4}",
            after in "\\PC{0,4}",
        ) {
            // Lowercasing is not case folding: "ß" uppercases to "SS".
            prop_assume!(needle.to_uppercase().to_lowercase() == needle.to_lowercase());
            let text = format!("{before} {needle} {after}");
            prop_assert!(TextMatch::contains(needle.to_uppercase()).matches(&text, &snapshot()));
        }

        #[test]
        fn without_normalization_compares_raw_text(text in "\\PC*") {
            let raw = TextMatch::exact(text.clone()).without_normalization();
            prop_assert!(raw.matches(&text, &snapshot()));
        }
    }
}
