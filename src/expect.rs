//! jest-dom style matchers: `expect(&node).to_be_visible()`.
//!
//! Matchers re-read the element from the latest frame. Every matcher can be
//! negated with [`Expect::not`].
use crate::{
    node::{Node, Snapshot, is_text_field},
    text_match::TextMatch,
};
use gpui::{Role, accesskit::Toggled};

/// What [`expect`] accepts: a [`Node`], or the `Option<Node>` a `query_*`
/// returns, for `to_be_in_the_document`.
pub trait Subject {
    fn into_node(self) -> Option<Node>;
}
impl Subject for Node {
    fn into_node(self) -> Option<Node> {
        Some(self)
    }
}
impl Subject for &Node {
    fn into_node(self) -> Option<Node> {
        Some(self.clone())
    }
}
impl Subject for Option<Node> {
    fn into_node(self) -> Option<Node> {
        self
    }
}
impl Subject for &Option<Node> {
    fn into_node(self) -> Option<Node> {
        self.clone()
    }
}

/// Starts an assertion.
pub fn expect(subject: impl Subject) -> Expect {
    Expect {
        node: subject.into_node(),
        negated: false,
    }
}

/// An assertion about one element. Methods panic when it fails.
#[must_use = "call a matcher such as .to_be_visible()"]
pub struct Expect {
    node: Option<Node>,
    negated: bool,
}

impl Expect {
    /// Inverts the next matcher, like `expect(x).not`.
    #[allow(clippy::should_implement_trait)]
    pub fn not(mut self) -> Self {
        self.negated = !self.negated;
        self
    }

    #[track_caller]
    fn current(&self, matcher: &str) -> Snapshot {
        match &self.node {
            Some(node) => node.snapshot(),
            None => panic!("expect(...).{matcher}(): received None; the element was not found"),
        }
    }

    #[track_caller]
    fn check(&self, matcher: &str, pass: bool, detail: impl FnOnce() -> String) {
        if pass == self.negated {
            let not = if self.negated { ".not()" } else { "" };
            let element = self.node.as_ref().map_or_else(
                || "None".to_owned(),
                |node| format!("{:?}", node.snapshot()),
            );
            panic!(
                "expect(element){not}.{matcher}() failed\n\n{}\n\nElement: {element}",
                detail()
            );
        }
    }

    #[track_caller]
    fn flag(&self, matcher: &str, test: impl FnOnce(&Snapshot) -> bool) {
        let snapshot = self.current(matcher);
        let pass = test(&snapshot);
        self.check(matcher, pass, || format!("received {pass}"));
    }

    #[track_caller]
    fn text(
        &self,
        matcher: &str,
        expected: TextMatch,
        read: impl FnOnce(&Snapshot) -> Option<String>,
    ) {
        let snapshot = self.current(matcher);
        let actual = read(&snapshot);
        let pass = expected.matches_option(actual.as_deref(), &snapshot);
        self.check(matcher, pass, || {
            format!("expected: {expected:?}\nreceived: {actual:?}")
        });
    }

    /// The element is rendered in the latest frame.
    #[track_caller]
    pub fn to_be_in_the_document(self) {
        let pass = self.node.as_ref().is_some_and(Node::is_connected);
        self.check("to_be_in_the_document", pass, || {
            "the element is not rendered".into()
        });
    }

    /// Rendered, painted with a visible area, and not hidden by an ancestor.
    #[track_caller]
    pub fn to_be_visible(self) {
        let pass = self.node.as_ref().is_some_and(Node::is_visible);
        self.check("to_be_visible", pass, || {
            format!("received visible: {}", !pass)
        });
    }

    #[track_caller]
    pub fn to_be_disabled(self) {
        self.flag("to_be_disabled", Snapshot::is_disabled);
    }

    #[track_caller]
    pub fn to_be_enabled(self) {
        self.flag("to_be_enabled", |s| !s.is_disabled());
    }

    #[track_caller]
    pub fn to_be_required(self) {
        self.flag("to_be_required", Snapshot::is_required);
    }

    #[track_caller]
    pub fn to_be_invalid(self) {
        self.flag("to_be_invalid", Snapshot::is_invalid);
    }

    #[track_caller]
    pub fn to_be_valid(self) {
        self.flag("to_be_valid", |s| !s.is_invalid());
    }

    #[track_caller]
    pub fn to_be_checked(self) {
        self.flag("to_be_checked", |s| s.toggled() == Some(Toggled::True));
    }

    /// Checkbox in the mixed state.
    #[track_caller]
    pub fn to_be_partially_checked(self) {
        self.flag("to_be_partially_checked", Snapshot::indeterminate);
    }

    /// Toggle button pressed.
    #[track_caller]
    pub fn to_be_pressed(self) {
        self.flag("to_be_pressed", |s| s.toggled() == Some(Toggled::True));
    }

    #[track_caller]
    pub fn to_be_selected(self) {
        self.flag("to_be_selected", |s| s.selected() == Some(true));
    }

    #[track_caller]
    pub fn to_be_expanded(self) {
        self.flag("to_be_expanded", |s| s.expanded() == Some(true));
    }

    #[track_caller]
    pub fn to_be_collapsed(self) {
        self.flag("to_be_collapsed", |s| s.expanded() == Some(false));
    }

    /// The element's tracked focus handle contains keyboard focus. Requires
    /// `.testable()` before `.track_focus(&handle)`.
    #[track_caller]
    pub fn to_have_focus(self) {
        let snapshot = self.current("to_have_focus");
        assert!(
            snapshot.focused().is_some(),
            "to_have_focus: no focus handle was observed for {snapshot:?}; call .testable() before .track_focus(&handle)"
        );
        let pass = snapshot.focused() == Some(true);
        self.check("to_have_focus", pass, || {
            format!("received focused: {pass}")
        });
    }

    /// A text field that is not disabled or read-only.
    #[track_caller]
    pub fn to_be_editable(self) {
        self.flag("to_be_editable", |s| {
            s.role().is_some_and(is_text_field) && !s.is_disabled() && !s.is_read_only()
        });
    }

    /// No observed descendants.
    #[track_caller]
    pub fn to_be_empty_dom_element(self) {
        self.current("to_be_empty_dom_element");
        let node = self.node.as_ref().expect("checked by current");
        let children = crate::Queries::elements(&crate::within(node)).len();
        self.check("to_be_empty_dom_element", children == 0, || {
            format!("received {children} observed descendants")
        });
    }

    #[track_caller]
    pub fn to_contain_element(self, other: &Node) {
        let snapshot = self.current("to_contain_element");
        let pass = other.is_connected() && snapshot.contains(&other.snapshot());
        self.check("to_contain_element", pass, || {
            format!("other element: {:?}", other.snapshot())
        });
    }

    #[track_caller]
    pub fn to_have_role(self, role: Role) {
        let snapshot = self.current("to_have_role");
        let pass = snapshot.role() == Some(role);
        self.check("to_have_role", pass, || {
            format!("expected: {role:?}\nreceived: {:?}", snapshot.role())
        });
    }

    #[track_caller]
    pub fn to_have_accessible_name(self, name: impl Into<TextMatch>) {
        self.text("to_have_accessible_name", name.into(), |s| {
            s.name().map(str::to_owned)
        });
    }

    #[track_caller]
    pub fn to_have_accessible_description(self, description: impl Into<TextMatch>) {
        self.text("to_have_accessible_description", description.into(), |s| {
            s.description().map(str::to_owned)
        });
    }

    /// The text a user reads on the element, [`Snapshot::text`].
    #[track_caller]
    pub fn to_have_text_content(self, text: impl Into<TextMatch>) {
        self.text("to_have_text_content", text.into(), |s| {
            s.text().map(str::to_owned)
        });
    }

    /// A form control's value, or a range widget's numeric value.
    #[track_caller]
    pub fn to_have_value(self, value: impl Into<TextMatch>) {
        self.text("to_have_value", value.into(), |s| {
            s.value()
                .map(str::to_owned)
                .or_else(|| s.numeric_value().map(|value| value.to_string()))
        });
    }

    #[track_caller]
    pub fn to_have_display_value(self, value: impl Into<TextMatch>) {
        self.text("to_have_display_value", value.into(), |s| {
            s.display_value().map(str::to_owned)
        });
    }

    #[track_caller]
    pub fn to_have_placeholder(self, placeholder: impl Into<TextMatch>) {
        self.text("to_have_placeholder", placeholder.into(), |s| {
            s.placeholder().map(str::to_owned)
        });
    }

    /// GPUI's analog of `toHaveAttribute('data-testid', ...)`.
    #[track_caller]
    pub fn to_have_test_id(self, id: impl Into<TextMatch>) {
        self.text("to_have_test_id", id.into(), |s| {
            s.test_id().map(str::to_owned)
        });
    }
}
