//! Testing Library queries: `by::*` matchers and `get`/`query`/`find`
//! variants, available on [`Screen`](crate::Screen) and [`Within`](crate::Within).
//!
//! | Variant      | No match      | One match | Many matches | Waits |
//! |--------------|---------------|-----------|--------------|-------|
//! | `get_*`      | panic         | `Node`    | panic        | no    |
//! | `query_*`    | `None`        | `Node`    | panic        | no    |
//! | `find_*`     | panic (timeout)| `Node`   | panic        | yes   |
//! | `get_all_*`  | panic         | `Vec`     | `Vec`        | no    |
//! | `query_all_*`| empty `Vec`   | `Vec`     | `Vec`        | no    |
//! | `find_all_*` | panic (timeout)| `Vec`    | `Vec`        | yes   |
use crate::{
    config,
    node::{Node, Snapshot},
    screen::Screen,
    text_match::TextMatch,
};
use gpui::{ElementId, Role, accesskit::Toggled};
use std::fmt::Write as _;

/// A predicate over observed elements.
pub trait Query {
    fn matches(&self, snapshot: &Snapshot) -> bool;
    /// Whether elements that are not visible can match. Only role queries
    /// exclude them by default, as in Testing Library.
    fn includes_hidden(&self) -> bool {
        true
    }
    /// Describes the query for error messages, e.g. `the role Button`.
    fn describe(&self) -> String;
}

impl<Q: Query + ?Sized> Query for &Q {
    fn matches(&self, snapshot: &Snapshot) -> bool {
        (**self).matches(snapshot)
    }
    fn includes_hidden(&self) -> bool {
        (**self).includes_hidden()
    }
    fn describe(&self) -> String {
        (**self).describe()
    }
}

/// Query constructors, named after Testing Library's `*By*` suffixes.
pub mod by {
    use super::*;

    /// `ByRole`: the accessibility role, optionally with name and state.
    pub fn role(role: Role) -> ByRole {
        ByRole::new(role)
    }
    /// `ByText`: what the user reads (labels, `text!` strings, names).
    pub fn text(text: impl Into<TextMatch>) -> ByText {
        ByText(text.into())
    }
    /// `ByLabelText`: the accessible name (`aria_label`).
    pub fn label_text(text: impl Into<TextMatch>) -> ByLabelText {
        ByLabelText {
            text: text.into(),
            role: None,
        }
    }
    /// `ByPlaceholderText`: `aria_placeholder`.
    pub fn placeholder_text(text: impl Into<TextMatch>) -> ByPlaceholderText {
        ByPlaceholderText(text.into())
    }
    /// `ByDisplayValue`: a form control's current value.
    pub fn display_value(text: impl Into<TextMatch>) -> ByDisplayValue {
        ByDisplayValue(text.into())
    }
    /// `ByTitle`: the accessible description (`aria_description`), GPUI's
    /// closest equivalent of the `title` attribute.
    pub fn title(text: impl Into<TextMatch>) -> ByTitle {
        ByTitle(text.into())
    }
    /// `ByAltText`: the accessible name of an image.
    pub fn alt_text(text: impl Into<TextMatch>) -> ByAltText {
        ByAltText(text.into())
    }
    /// `ByTestId`: the accessibility id set with `.accessibility_id(...)`.
    pub fn test_id(id: impl Into<TextMatch>) -> ByTestId {
        ByTestId(id.into())
    }
    /// The last GPUI `ElementId` in the element's path, as gpui-kit's
    /// `window.find(id)` matches.
    pub fn element_id(id: impl Into<ElementId>) -> ByElementId {
        ByElementId(id.into())
    }
}

/// See [`by::role`].
#[derive(Clone, Debug)]
pub struct ByRole {
    role: Role,
    name: Option<TextMatch>,
    description: Option<TextMatch>,
    checked: Option<bool>,
    pressed: Option<bool>,
    selected: Option<bool>,
    expanded: Option<bool>,
    disabled: Option<bool>,
    level: Option<usize>,
    value_now: Option<f64>,
    value_min: Option<f64>,
    value_max: Option<f64>,
    value_text: Option<TextMatch>,
    hidden: Option<bool>,
}

impl ByRole {
    pub fn new(role: Role) -> Self {
        Self {
            role,
            name: None,
            description: None,
            checked: None,
            pressed: None,
            selected: None,
            expanded: None,
            disabled: None,
            level: None,
            value_now: None,
            value_min: None,
            value_max: None,
            value_text: None,
            hidden: None,
        }
    }
    /// Filters by accessible name.
    pub fn name(mut self, name: impl Into<TextMatch>) -> Self {
        self.name = Some(name.into());
        self
    }
    /// Filters by accessible description.
    pub fn description(mut self, description: impl Into<TextMatch>) -> Self {
        self.description = Some(description.into());
        self
    }
    /// Checkboxes, switches and radios: toggled on (`true`) or off.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }
    /// Toggle buttons: pressed (`true`) or not.
    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = Some(pressed);
        self
    }
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = Some(expanded);
        self
    }
    /// Not in Testing Library's `ByRole`, which has no disabled filter;
    /// provided because GPUI exposes the flag directly.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }
    /// Heading level.
    pub fn level(mut self, level: usize) -> Self {
        self.level = Some(level);
        self
    }
    /// Range widgets: `value: { now }`.
    pub fn value_now(mut self, value: f64) -> Self {
        self.value_now = Some(value);
        self
    }
    pub fn value_min(mut self, value: f64) -> Self {
        self.value_min = Some(value);
        self
    }
    pub fn value_max(mut self, value: f64) -> Self {
        self.value_max = Some(value);
        self
    }
    /// Range widgets: `value: { text }`, the accessibility value string.
    pub fn value_text(mut self, text: impl Into<TextMatch>) -> Self {
        self.value_text = Some(text.into());
        self
    }
    /// Includes elements that are not visible. Defaults to
    /// [`Config::default_hidden`](crate::Config::default_hidden).
    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = Some(hidden);
        self
    }
}

impl From<Role> for ByRole {
    fn from(role: Role) -> Self {
        Self::new(role)
    }
}

impl Query for ByRole {
    fn matches(&self, s: &Snapshot) -> bool {
        s.role() == Some(self.role)
            && self
                .name
                .as_ref()
                .is_none_or(|name| name.matches_option(s.name(), s))
            && self
                .description
                .as_ref()
                .is_none_or(|description| description.matches_option(s.description(), s))
            && self
                .checked
                .is_none_or(|checked| s.checked() == Some(checked) && !s.indeterminate())
            && self.pressed.is_none_or(|pressed| {
                s.toggled()
                    .is_some_and(|toggled| (toggled == Toggled::True) == pressed)
            })
            && self
                .selected
                .is_none_or(|selected| s.selected().unwrap_or(false) == selected)
            && self
                .expanded
                .is_none_or(|expanded| s.expanded() == Some(expanded))
            && self
                .disabled
                .is_none_or(|disabled| s.is_disabled() == disabled)
            && self.level.is_none_or(|level| s.level() == Some(level))
            && self
                .value_now
                .is_none_or(|value| s.numeric_value() == Some(value))
            && self
                .value_min
                .is_none_or(|value| s.min_numeric_value() == Some(value))
            && self
                .value_max
                .is_none_or(|value| s.max_numeric_value() == Some(value))
            && self
                .value_text
                .as_ref()
                .is_none_or(|text| text.matches_option(s.value(), s))
    }
    fn includes_hidden(&self) -> bool {
        self.hidden
            .unwrap_or_else(|| config::get().default_hidden())
    }
    fn describe(&self) -> String {
        let mut text = format!("the role {:?}", self.role);
        if let Some(name) = &self.name {
            let _ = write!(text, " and name {name:?}");
        }
        if let Some(description) = &self.description {
            let _ = write!(text, " and description {description:?}");
        }
        for (key, value) in [
            ("checked", self.checked),
            ("pressed", self.pressed),
            ("selected", self.selected),
            ("expanded", self.expanded),
            ("disabled", self.disabled),
        ] {
            if let Some(value) = value {
                let _ = write!(text, " and {key}: {value}");
            }
        }
        if let Some(level) = self.level {
            let _ = write!(text, " and level {level}");
        }
        text
    }
}

macro_rules! text_query {
    ($(#[$doc:meta])* $name:ident, $what:literal, |$s:ident| $field:expr) => {
        $(#[$doc])*
        #[derive(Clone, Debug)]
        pub struct $name(TextMatch);
        impl<T: Into<TextMatch>> From<T> for $name {
            fn from(text: T) -> Self {
                Self(text.into())
            }
        }
        impl Query for $name {
            fn matches(&self, $s: &Snapshot) -> bool {
                self.0.matches_option($field, $s)
            }
            fn describe(&self) -> String {
                format!(concat!($what, " {:?}"), self.0)
            }
        }
    };
}

text_query!(
    /// See [`by::text`].
    ByText, "the text", |s| s.text()
);
text_query!(
    /// See [`by::placeholder_text`].
    ByPlaceholderText, "the placeholder text", |s| s.placeholder()
);
text_query!(
    /// See [`by::display_value`].
    ByDisplayValue, "the display value", |s| s.display_value()
);
text_query!(
    /// See [`by::title`].
    ByTitle, "the title", |s| s.description()
);
text_query!(
    /// See [`by::alt_text`].
    ByAltText, "the alt text", |s| (s.role() == Some(Role::Image)).then(|| s.name()).flatten()
);
text_query!(
    /// See [`by::test_id`].
    ByTestId, "the test id", |s| s.test_id()
);

/// See [`by::label_text`].
#[derive(Clone, Debug)]
pub struct ByLabelText {
    text: TextMatch,
    role: Option<Role>,
}

impl ByLabelText {
    /// Testing Library's `selector` option: only elements with this role.
    pub fn role(mut self, role: Role) -> Self {
        self.role = Some(role);
        self
    }
}

impl<T: Into<TextMatch>> From<T> for ByLabelText {
    fn from(text: T) -> Self {
        by::label_text(text)
    }
}

impl Query for ByLabelText {
    fn matches(&self, s: &Snapshot) -> bool {
        // A `text!` label's name is its text, not a label for a control.
        !matches!(s.role(), Some(Role::Label | Role::TextRun))
            && self.role.is_none_or(|role| s.role() == Some(role))
            && self.text.matches_option(s.name(), s)
    }
    fn describe(&self) -> String {
        format!("the label text {:?}", self.text)
    }
}

/// See [`by::element_id`].
#[derive(Clone, Debug)]
pub struct ByElementId(ElementId);

impl Query for ByElementId {
    fn matches(&self, s: &Snapshot) -> bool {
        s.path().last() == Some(&self.0)
    }
    fn describe(&self) -> String {
        format!("the element id {:?}", self.0)
    }
}

impl<T: Into<ElementId>> From<T> for ByElementId {
    fn from(id: T) -> Self {
        Self(id.into())
    }
}

macro_rules! named_queries {
    ($($ty:ty => $get:ident, $get_all:ident, $query:ident, $query_all:ident, $find:ident, $find_all:ident;)*) => {
        $(
            #[track_caller]
            fn $get(&self, query: impl Into<$ty>) -> Node {
                self.get(query.into())
            }
            #[track_caller]
            fn $get_all(&self, query: impl Into<$ty>) -> Vec<Node> {
                self.get_all(query.into())
            }
            #[track_caller]
            fn $query(&self, query: impl Into<$ty>) -> Option<Node> {
                self.query(query.into())
            }
            fn $query_all(&self, query: impl Into<$ty>) -> Vec<Node> {
                self.query_all(query.into())
            }
            fn $find(&self, query: impl Into<$ty>) -> impl Future<Output = Node> {
                self.find(query.into())
            }
            fn $find_all(&self, query: impl Into<$ty>) -> impl Future<Output = Vec<Node>> {
                self.find_all(query.into())
            }
        )*
    };
}

/// The query methods shared by [`Screen`] and [`Within`](crate::Within).
pub trait Queries {
    fn screen(&self) -> &Screen;
    /// The element queries are limited to, or `None` for the whole window.
    fn container(&self) -> Option<&Snapshot>;

    /// Every observed element in this container, in reading order.
    fn elements(&self) -> Vec<Node> {
        let screen = self.screen();
        screen
            .snapshots_in(self.container())
            .into_iter()
            .map(|snapshot| Node::new(screen.clone(), snapshot))
            .collect()
    }

    fn query_all(&self, query: impl Query) -> Vec<Node> {
        let screen = self.screen();
        screen
            .snapshots_in(self.container())
            .into_iter()
            .filter(|snapshot| {
                (query.includes_hidden() || snapshot.is_visible()) && query.matches(snapshot)
            })
            .map(|snapshot| Node::new(screen.clone(), snapshot))
            .collect()
    }

    #[track_caller]
    fn query(&self, query: impl Query) -> Option<Node> {
        let mut nodes = self.query_all(&query);
        if nodes.len() > 1 {
            panic!("{}", self.multiple_error(&query, &nodes));
        }
        nodes.pop()
    }

    #[track_caller]
    fn get(&self, query: impl Query) -> Node {
        self.query(&query)
            .unwrap_or_else(|| panic!("{}", self.missing_error(&query)))
    }

    #[track_caller]
    fn get_all(&self, query: impl Query) -> Vec<Node> {
        let nodes = self.query_all(&query);
        if nodes.is_empty() {
            panic!("{}", self.missing_error(&query));
        }
        nodes
    }

    /// Waits until exactly one element matches, like `findBy*`.
    fn find(&self, query: impl Query) -> impl Future<Output = Node> {
        async move {
            let screen = self.screen().clone();
            let result = crate::wait::poll(&screen, None, || {
                let nodes = self.query_all(&query);
                match nodes.len() {
                    1 => Ok(nodes.into_iter().next()),
                    0 => Ok(None),
                    _ => Err(self.multiple_error(&query, &nodes)),
                }
            })
            .await;
            match result {
                Ok(node) => node,
                Err(error) => panic!("{error}"),
            }
            .unwrap_or_else(|| panic!("{}", self.missing_error(&query)))
        }
    }

    /// Waits until at least one element matches, like `findAllBy*`.
    fn find_all(&self, query: impl Query) -> impl Future<Output = Vec<Node>> {
        async move {
            let screen = self.screen().clone();
            let result = crate::wait::poll(&screen, None, || {
                let nodes = self.query_all(&query);
                Ok::<_, String>((!nodes.is_empty()).then_some(nodes))
            })
            .await;
            result
                .expect("find_all never fails early")
                .unwrap_or_else(|| panic!("{}", self.missing_error(&query)))
        }
    }

    #[doc(hidden)]
    fn missing_error(&self, query: &dyn Query) -> String {
        let hidden = if query.includes_hidden() {
            ""
        } else {
            " (hidden elements excluded; use .hidden(true) to include them)"
        };
        format!(
            "Unable to find an element with {}{hidden}.\n\n{}",
            query.describe(),
            self.screen().pretty_in(self.container())
        )
    }

    #[doc(hidden)]
    fn multiple_error(&self, query: &dyn Query, nodes: &[Node]) -> String {
        let mut text = format!(
            "Found multiple elements with {}. Use a *_all_* query, a more specific query, or within(...).\n\nMatches:\n",
            query.describe()
        );
        for node in nodes {
            let _ = writeln!(text, "  {:?}", node.initial_snapshot());
        }
        text
    }

    named_queries! {
        ByRole => get_by_role, get_all_by_role, query_by_role, query_all_by_role, find_by_role, find_all_by_role;
        ByText => get_by_text, get_all_by_text, query_by_text, query_all_by_text, find_by_text, find_all_by_text;
        ByLabelText => get_by_label_text, get_all_by_label_text, query_by_label_text, query_all_by_label_text, find_by_label_text, find_all_by_label_text;
        ByPlaceholderText => get_by_placeholder_text, get_all_by_placeholder_text, query_by_placeholder_text, query_all_by_placeholder_text, find_by_placeholder_text, find_all_by_placeholder_text;
        ByDisplayValue => get_by_display_value, get_all_by_display_value, query_by_display_value, query_all_by_display_value, find_by_display_value, find_all_by_display_value;
        ByTitle => get_by_title, get_all_by_title, query_by_title, query_all_by_title, find_by_title, find_all_by_title;
        ByAltText => get_by_alt_text, get_all_by_alt_text, query_by_alt_text, query_all_by_alt_text, find_by_alt_text, find_all_by_alt_text;
        ByTestId => get_by_test_id, get_all_by_test_id, query_by_test_id, query_all_by_test_id, find_by_test_id, find_all_by_test_id;
        ByElementId => get_by_element_id, get_all_by_element_id, query_by_element_id, query_all_by_element_id, find_by_element_id, find_all_by_element_id;
    }
}
