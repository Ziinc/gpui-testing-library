//! Rendered elements as tests see them.
use crate::screen::Screen;
use gpui::{Bounds, Element, ElementId, Pixels, Role, SharedString, accesskit};
use std::fmt;

/// Where a snapshot was observed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// An element marked with [`crate::TestableExt::testable`].
    Testable,
    /// An element marked with gpui-kit's `.test_support()`.
    Kit,
}

/// Owned accessibility facts from the last paint of an observed element.
#[derive(Clone)]
pub struct Snapshot {
    pub(crate) source: Source,
    pub(crate) path: Vec<ElementId>,
    pub(crate) role: Option<Role>,
    pub(crate) name: Option<SharedString>,
    pub(crate) description: Option<SharedString>,
    pub(crate) value: Option<SharedString>,
    pub(crate) placeholder: Option<SharedString>,
    pub(crate) test_id: Option<SharedString>,
    pub(crate) keyboard_shortcut: Option<SharedString>,
    pub(crate) toggled: Option<accesskit::Toggled>,
    pub(crate) selected: Option<bool>,
    pub(crate) expanded: Option<bool>,
    pub(crate) disabled: bool,
    pub(crate) required: bool,
    pub(crate) read_only: bool,
    pub(crate) invalid: bool,
    pub(crate) level: Option<usize>,
    pub(crate) numeric_value: Option<f64>,
    pub(crate) min_numeric_value: Option<f64>,
    pub(crate) max_numeric_value: Option<f64>,
    pub(crate) bounds: Bounds<Pixels>,
    pub(crate) visible: bool,
    pub(crate) focused: Option<bool>,
    pub(crate) focusable: bool,
    pub(crate) clickable: bool,
}

impl Snapshot {
    pub(crate) fn capture<E: Element>(
        element: &E,
        path: Vec<ElementId>,
        bounds: Bounds<Pixels>,
    ) -> Self {
        let role = element.a11y_role();
        let mut node = accesskit::Node::new(role.unwrap_or(Role::Unknown));
        element.write_a11y_info(&mut node);
        let string = |value: Option<&str>| value.map(|value| SharedString::from(value.to_owned()));
        Self {
            source: Source::Testable,
            path,
            role,
            name: string(node.label()),
            description: string(node.description()),
            value: string(node.value()),
            placeholder: string(node.placeholder()),
            test_id: string(node.author_id()),
            keyboard_shortcut: string(node.keyboard_shortcut()),
            toggled: node.toggled(),
            selected: node.is_selected(),
            expanded: node.is_expanded(),
            disabled: node.is_disabled(),
            required: node.is_required(),
            read_only: node.is_read_only(),
            invalid: node.invalid().is_some(),
            level: node.level(),
            numeric_value: node.numeric_value(),
            min_numeric_value: node.min_numeric_value(),
            max_numeric_value: node.max_numeric_value(),
            bounds,
            // Set during paint; anything that is never painted stays hidden.
            visible: false,
            focused: None,
            focusable: node.supports_action(accesskit::Action::Focus),
            clickable: node.supports_action(accesskit::Action::Click),
        }
    }

    pub fn source(&self) -> Source {
        self.source
    }
    /// The GPUI global element path, outermost id first.
    pub fn path(&self) -> &[ElementId] {
        &self.path
    }
    /// The last id in [`Self::path`].
    pub fn element_id(&self) -> &ElementId {
        self.path.last().expect("observed elements have an id")
    }
    pub fn role(&self) -> Option<Role> {
        self.role
    }
    /// The accessible name (`aria_label`).
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
    /// The accessibility value: a text field's contents, a `text!`'s string.
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }
    pub fn placeholder(&self) -> Option<&str> {
        self.placeholder.as_deref()
    }
    /// The accessibility id set with `.accessibility_id(...)`.
    pub fn test_id(&self) -> Option<&str> {
        self.test_id.as_deref()
    }
    pub fn keyboard_shortcut(&self) -> Option<&str> {
        self.keyboard_shortcut.as_deref()
    }
    /// The text a user reads on this element: a label's string, otherwise the
    /// accessible name. Text inputs have none; use [`Self::display_value`].
    pub fn text(&self) -> Option<&str> {
        match self.role {
            Some(role) if is_text_field(role) => None,
            Some(Role::Label | Role::TextRun) => self.value().or(self.name()),
            _ => self.name(),
        }
    }
    /// The current value of a form control.
    pub fn display_value(&self) -> Option<&str> {
        match self.role {
            Some(Role::Label | Role::TextRun) => None,
            _ => self.value(),
        }
    }
    pub fn toggled(&self) -> Option<accesskit::Toggled> {
        self.toggled
    }
    /// `Some(true)` when toggled on, `None` when the element is not checkable.
    pub fn checked(&self) -> Option<bool> {
        self.toggled
            .map(|toggled| toggled == accesskit::Toggled::True)
    }
    pub fn indeterminate(&self) -> bool {
        self.toggled == Some(accesskit::Toggled::Mixed)
    }
    pub fn selected(&self) -> Option<bool> {
        self.selected
    }
    pub fn expanded(&self) -> Option<bool> {
        self.expanded
    }
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
    pub fn is_required(&self) -> bool {
        self.required
    }
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }
    pub fn is_invalid(&self) -> bool {
        self.invalid
    }
    /// Heading level.
    pub fn level(&self) -> Option<usize> {
        self.level
    }
    pub fn numeric_value(&self) -> Option<f64> {
        self.numeric_value
    }
    pub fn min_numeric_value(&self) -> Option<f64> {
        self.min_numeric_value
    }
    pub fn max_numeric_value(&self) -> Option<f64> {
        self.max_numeric_value
    }
    /// Window-local bounds from the last paint.
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }
    /// Painted with a non-empty, unclipped area, and no ancestor hid it.
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    /// Whether the tracked focus handle contains keyboard focus. `None` when no
    /// focus handle was observed (call `.testable()` before `.track_focus`).
    pub fn focused(&self) -> Option<bool> {
        self.focused
    }
    pub fn is_focusable(&self) -> bool {
        self.focusable
    }
    pub fn is_clickable(&self) -> bool {
        self.clickable
    }
    /// Whether `other` is a strict descendant of this element.
    pub fn contains(&self, other: &Snapshot) -> bool {
        other.path.len() > self.path.len() && other.path.starts_with(&self.path)
    }
}

pub(crate) fn is_text_field(role: Role) -> bool {
    matches!(
        role,
        Role::TextInput
            | Role::MultilineTextInput
            | Role::SearchInput
            | Role::PasswordInput
            | Role::EmailInput
            | Role::NumberInput
            | Role::PhoneNumberInput
            | Role::UrlInput
            | Role::DateInput
            | Role::DateTimeInput
            | Role::WeekInput
            | Role::MonthInput
            | Role::TimeInput
            | Role::SpinButton
    )
}

impl fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&crate::pretty::describe(self))
    }
}

/// A live handle to a rendered element, like a DOM element in Testing Library.
///
/// Accessors and matchers re-read the element from the window's latest
/// frame, so state changes after a click are visible without querying again.
/// When the element is gone, they report its last known state and
/// [`Node::is_connected`] returns false.
#[derive(Clone)]
pub struct Node {
    screen: Screen,
    last: Snapshot,
}

impl Node {
    pub(crate) fn new(screen: Screen, last: Snapshot) -> Self {
        Self { screen, last }
    }

    /// The screen this element was found on.
    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    /// Re-reads the element after drawing a fresh frame.
    pub fn snapshot(&self) -> Snapshot {
        self.screen
            .resolve(&self.last.path)
            .unwrap_or_else(|| self.last.clone())
    }

    /// The state captured when the element was queried, without redrawing.
    pub fn initial_snapshot(&self) -> &Snapshot {
        &self.last
    }

    /// Whether the element was painted in the latest frame.
    pub fn is_connected(&self) -> bool {
        self.screen.resolve(&self.last.path).is_some()
    }

    pub fn path(&self) -> &[ElementId] {
        &self.last.path
    }
    pub fn element_id(&self) -> &ElementId {
        self.last.element_id()
    }
    pub fn role(&self) -> Option<Role> {
        self.snapshot().role()
    }
    pub fn name(&self) -> Option<String> {
        self.snapshot().name().map(str::to_owned)
    }
    pub fn description(&self) -> Option<String> {
        self.snapshot().description().map(str::to_owned)
    }
    pub fn value(&self) -> Option<String> {
        self.snapshot().value().map(str::to_owned)
    }
    pub fn text(&self) -> Option<String> {
        self.snapshot().text().map(str::to_owned)
    }
    pub fn test_id(&self) -> Option<String> {
        self.snapshot().test_id().map(str::to_owned)
    }
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.snapshot().bounds()
    }
    pub fn is_visible(&self) -> bool {
        self.is_connected() && self.snapshot().is_visible()
    }
    pub fn is_focused(&self) -> bool {
        self.snapshot().focused() == Some(true)
    }
    /// Whether `other` is a strict descendant of this element.
    pub fn contains(&self, other: &Node) -> bool {
        self.last.contains(&other.last)
    }
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.last, f)
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.screen.window() == other.screen.window() && self.last.path == other.last.path
    }
}
