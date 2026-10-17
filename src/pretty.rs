//! Readable dumps of observed elements, for `debug()` and error messages.
use crate::node::Snapshot;
use gpui::ElementId;
use std::fmt::Write as _;

/// One line describing an element: role, name and state.
pub(crate) fn describe(snapshot: &Snapshot) -> String {
    let mut line = match snapshot.role() {
        Some(role) => format!("{role:?}"),
        None => "<no role>".to_owned(),
    };
    if let Some(name) = snapshot.name() {
        let _ = write!(line, " {name:?}");
    }
    match snapshot.element_id() {
        ElementId::Name(name) => {
            let _ = write!(line, " #{name}");
        }
        ElementId::Integer(id) => {
            let _ = write!(line, " #{id}");
        }
        id => {
            let _ = write!(line, " #{id:?}");
        }
    }
    let mut fields = Vec::new();
    if let Some(value) = snapshot.value() {
        fields.push(format!("value={value:?}"));
    }
    if let Some(description) = snapshot.description() {
        fields.push(format!("description={description:?}"));
    }
    if let Some(placeholder) = snapshot.placeholder() {
        fields.push(format!("placeholder={placeholder:?}"));
    }
    if let Some(test_id) = snapshot.test_id() {
        fields.push(format!("test-id={test_id:?}"));
    }
    if let Some(level) = snapshot.level() {
        fields.push(format!("level={level}"));
    }
    if let Some(value) = snapshot.numeric_value() {
        fields.push(format!("value-now={value}"));
    }
    if snapshot.indeterminate() {
        fields.push("mixed".into());
    } else if let Some(checked) = snapshot.checked() {
        fields.push(if checked { "checked" } else { "unchecked" }.into());
    }
    for (flag, on) in [
        ("selected", snapshot.selected() == Some(true)),
        ("expanded", snapshot.expanded() == Some(true)),
        ("collapsed", snapshot.expanded() == Some(false)),
        ("focused", snapshot.focused() == Some(true)),
        ("disabled", snapshot.is_disabled()),
        ("required", snapshot.is_required()),
        ("readonly", snapshot.is_read_only()),
        ("invalid", snapshot.is_invalid()),
        ("hidden", !snapshot.is_visible()),
    ] {
        if on {
            fields.push(flag.into());
        }
    }
    if !fields.is_empty() {
        let _ = write!(line, " [{}]", fields.join(", "));
    }
    line
}

/// Elements indented under their nearest observed ancestor.
pub(crate) fn tree(snapshots: &[Snapshot]) -> String {
    if snapshots.is_empty() {
        return "<no observed elements: mark elements with .testable() and draw a frame>".into();
    }
    let mut text = String::new();
    for snapshot in snapshots {
        let depth = snapshots
            .iter()
            .filter(|other| other.contains(snapshot))
            .count();
        let _ = writeln!(text, "{}{}", "  ".repeat(depth), describe(snapshot));
    }
    text
}

/// Elements grouped by role, like Testing Library's `logRoles`.
pub(crate) fn roles(snapshots: &[Snapshot]) -> String {
    let mut groups: Vec<(String, Vec<&Snapshot>)> = Vec::new();
    for snapshot in snapshots {
        let role = snapshot
            .role()
            .map_or_else(|| "<no role>".to_owned(), |role| format!("{role:?}"));
        match groups.iter_mut().find(|(name, _)| *name == role) {
            Some((_, group)) => group.push(snapshot),
            None => groups.push((role, vec![snapshot])),
        }
    }
    let mut text = String::new();
    for (role, group) in groups {
        let _ = writeln!(text, "{role}:");
        for snapshot in group {
            let _ = writeln!(text, "  {}", describe(snapshot));
        }
        text.push('\n');
    }
    text
}
