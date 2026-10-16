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
