//! gpui-kit integration (feature `gpui-kit`).
//!
//! gpui-kit components register themselves with `.test_support()`, and
//! applications built on gpui-kit do the same for their own elements. Those
//! observations are merged into every query, so a Kit `Button` is found by
//! `get_by_role(Role::Button)` without marking it `.testable()`. When an
//! element is observed both ways, the `.testable()` snapshot wins.
use crate::{
    node::{Snapshot, Source},
    screen::{RenderOptions, RenderResult, Screen},
};
use gpui::{
    App, Bounds, Entity, Render, TestAppContext, Window, WindowBounds, WindowOptions,
    accesskit::Toggled,
};
use gpui_kit::base::test_support;

pub(crate) fn snapshots(window: &Window) -> Vec<Snapshot> {
    test_support::snapshots(window)
        .into_iter()
        .map(|kit| {
            // `focused()` panics when a focusable element's focus binding was
            // missed; its Debug output reports that case without panicking.
            let debug = format!("{kit:?}");
            let binding_missed = debug.contains("<binding missed>");
            Snapshot {
                source: Source::Kit,
                path: kit.path().to_vec(),
                role: kit.role(),
                name: kit.label().map(|label| label.to_owned().into()),
                description: None,
                value: kit.value().map(|value| value.to_owned().into()),
                placeholder: None,
                test_id: None,
                keyboard_shortcut: None,
                toggled: match (kit.checked(), kit.indeterminate()) {
                    (_, Some(true)) => Some(Toggled::Mixed),
                    (Some(true), _) => Some(Toggled::True),
                    (Some(false), _) => Some(Toggled::False),
                    _ => None,
                },
                selected: kit.selected(),
                expanded: kit.expanded(),
                disabled: kit.disabled() == Some(true),
                required: false,
                read_only: false,
                invalid: false,
                level: None,
                numeric_value: None,
                min_numeric_value: None,
                max_numeric_value: None,
                bounds: kit.bounds(),
                visible: kit.visible(),
                focused: if binding_missed { None } else { kit.focused() },
                focusable: debug.contains("focus_action: true"),
                clickable: false,
            }
        })
        .collect()
}

/// Opens `build`'s content through `gpui_kit::open_window`, inside Kit's Root,
/// exactly as an application does. Call `gpui_kit::init(cx)` first.
///
/// The returned screen's queries include gpui-kit's `.test_support()`
/// observations, and `Screen` works with gpui-kit's own `TestWindowExt`
/// helpers on the same window.
pub fn render<V: Render>(
    cx: &mut TestAppContext,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> RenderResult<V> {
    render_with(cx, RenderOptions::default(), build)
}

/// [`render`] with options.
pub fn render_with<V: Render>(
    cx: &mut TestAppContext,
    options: RenderOptions,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> RenderResult<V> {
    let (window, view) = cx.update(|cx| {
        let bounds = options
            .size()
            .map(|size| Bounds::new(Default::default(), size))
            .unwrap_or_else(|| Bounds::maximized(None, cx));
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            cx,
            build,
        )
        .expect("open a gpui-kit test window")
    });
    let screen = Screen::new(cx, window);
    screen.redraw();
    RenderResult::new(screen, view)
}
