//! Opt-in observation of rendered elements.
//!
//! GPUI's test platform never activates AccessKit, so the accessibility tree a
//! screen reader would see is not available inside a `TestAppContext`. Instead,
//! elements marked with [`TestableExt::testable`] record the same accessibility
//! facts (role, name, value, states) every frame, plus their painted bounds,
//! visibility and focus. The wrapper adds no layout node and forwards every
//! Element, Styled and interactive method, so it can stay in production code.
use crate::node::Snapshot;
use gpui::{
    App, Bounds, Element, ElementId, FocusHandle, GlobalElementId, Hitbox, InspectorElementId,
    InteractiveElement, IntoElement, LayoutId, Pixels, Text, Visibility, Window, accesskit, px,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
    sync::Arc,
};

/// Identifies a window by its text system, which each `Window` owns uniquely,
/// so equal `WindowId`s from different test apps never collide.
pub(crate) type WindowKey = usize;

pub(crate) fn window_key(window: &Window) -> WindowKey {
    Arc::as_ptr(window.text_system()) as *const () as usize
}

type Registry = HashMap<WindowKey, HashMap<GlobalElementId, Weak<Registration>>>;
thread_local! { static REGISTRY: RefCell<Registry> = RefCell::new(HashMap::new()); }

#[doc(hidden)]
pub struct Registration {
    window: WindowKey,
    global_id: GlobalElementId,
    snapshot: RefCell<Snapshot>,
}

impl Drop for Registration {
    fn drop(&mut self) {
        let _ = REGISTRY.try_with(|registry| {
            if let Ok(mut registry) = registry.try_borrow_mut()
                && let Some(entries) = registry.get_mut(&self.window)
            {
                entries.remove(&self.global_id);
                if entries.is_empty() {
                    registry.remove(&self.window);
                }
            }
        });
    }
}

/// Every element observed in the window's last completed frame.
pub(crate) fn snapshots(window: &Window) -> Vec<Snapshot> {
    REGISTRY.with(|registry| {
        registry
            .borrow()
            .get(&window_key(window))
            .map(|entries| {
                entries
                    .values()
                    .filter_map(Weak::upgrade)
                    .map(|entry| entry.snapshot.borrow().clone())
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// How an observed element decides whether it was visible when painted.
#[doc(hidden)]
pub trait Probe: Element {
    fn probe_visible(
        &mut self,
        id: Option<&GlobalElementId>,
        prepaint: &Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) -> bool;
}

impl<E: Element<PrepaintState = Option<Hitbox>> + InteractiveElement> Probe for E {
    fn probe_visible(
        &mut self,
        id: Option<&GlobalElementId>,
        hitbox: &Option<Hitbox>,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        let style = self
            .interactivity()
            .compute_style(id, hitbox.as_ref(), window, cx);
        style.visibility != Visibility::Hidden && style.opacity.unwrap_or(1.) > 0.
    }
}

/// A [`Text`] that can be observed. Created by [`TestableTextExt::testable`].
#[doc(hidden)]
pub struct ObservableText(Text);

impl IntoElement for ObservableText {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for ObservableText {
    type RequestLayoutState = <Text as Element>::RequestLayoutState;
    type PrepaintState = <Text as Element>::PrepaintState;
    fn id(&self) -> Option<ElementId> {
        self.0.id().cloned()
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.0.source_location()
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.0.request_layout(id, inspector, window, cx)
    }
    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.0.prepaint(id, inspector, bounds, layout, window, cx)
    }
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.0
            .paint(id, inspector, bounds, layout, prepaint, window, cx)
    }
    fn a11y_role(&self) -> Option<accesskit::Role> {
        self.0.a11y_role()
    }
    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.0.write_a11y_info(node)
    }
}

impl Probe for ObservableText {
    fn probe_visible(
        &mut self,
        _: Option<&GlobalElementId>,
        _: &(),
        _: &mut Window,
        _: &mut App,
    ) -> bool {
        true
    }
}

/// Observes an element so queries can find it.
///
/// The element must have an `ElementId`. Call `.testable()` before
/// `.track_focus(&handle)` so focus can be reported.
pub trait TestableExt: Probe + InteractiveElement + Sized {
    fn testable(self) -> Testable<Self> {
        Testable::new(self)
    }
}
impl<E: Probe + InteractiveElement> TestableExt for E {}

/// Observes a [`Text`] created with `text!`, so `get_by_text` can find it.
pub trait TestableTextExt {
    fn testable(self) -> Testable<ObservableText>;
}
impl TestableTextExt for Text {
    fn testable(self) -> Testable<ObservableText> {
        Testable::new(ObservableText(self))
    }
}

/// A transparent element wrapper that records accessibility facts each frame.
pub struct Testable<E> {
    inner: E,
    focus: Option<FocusHandle>,
}

impl<E: Element> Testable<E> {
    fn new(inner: E) -> Self {
        assert!(
            inner.id().is_some(),
            "testable() requires an element with an id: call .id(...) first"
        );
        Self { inner, focus: None }
    }
}

impl<E: Probe> IntoElement for Testable<E> {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl<E: Probe> Element for Testable<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = (E::PrepaintState, Rc<Registration>);

    fn id(&self) -> Option<ElementId> {
        self.inner.id()
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.inner.source_location()
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.inner.request_layout(id, inspector, window, cx)
    }
    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let global_id = id.expect("observed elements have an id");
        let snapshot = Snapshot::capture(&self.inner, global_id.to_vec(), bounds);
        let registration =
            window.with_element_state(global_id, |state: Option<Rc<Registration>>, window| {
                let registration = state.unwrap_or_else(|| {
                    Rc::new(Registration {
                        window: window_key(window),
                        global_id: global_id.clone(),
                        snapshot: RefCell::new(snapshot.clone()),
                    })
                });
                *registration.snapshot.borrow_mut() = snapshot;
                REGISTRY.with(|registry| {
                    registry
                        .borrow_mut()
                        .entry(registration.window)
                        .or_default()
                        .insert(global_id.clone(), Rc::downgrade(&registration));
                });
                (registration.clone(), registration)
            });
        let prepaint = self
            .inner
            .prepaint(id, inspector, bounds, layout, window, cx);
        (prepaint, registration)
    }
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        // Painting is only reached when every ancestor is visible, so an
        // element hidden by an ancestor stays invisible from prepaint.
        let styled_visible = self.inner.probe_visible(id, &prepaint.0, window, cx);
        let clipped = bounds
            .intersect(&window.content_mask().bounds)
            .intersect(&Bounds::new(Default::default(), window.viewport_size()));
        {
            let mut snapshot = prepaint.1.snapshot.borrow_mut();
            snapshot.visible =
                styled_visible && clipped.size.width > px(0.) && clipped.size.height > px(0.);
            snapshot.visible_bounds = if snapshot.visible {
                clipped
            } else {
                Bounds::default()
            };
            snapshot.focused = self
                .focus
                .as_ref()
                .map(|focus| focus.contains_focused(window, cx));
        }
        self.inner
            .paint(id, inspector, bounds, layout, &mut prepaint.0, window, cx);
    }
    fn a11y_role(&self) -> Option<accesskit::Role> {
        self.inner.a11y_role()
    }
    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.inner.write_a11y_info(node);
    }
    fn a11y_synthetic_children(
        &mut self,
        prepaint: &mut Self::PrepaintState,
        builder: &mut gpui::A11ySubtreeBuilder,
    ) {
        self.inner.a11y_synthetic_children(&mut prepaint.0, builder);
    }
}

impl<E: Probe + gpui::Styled> gpui::Styled for Testable<E> {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.inner.style()
    }
}

impl<E: Probe + InteractiveElement> InteractiveElement for Testable<E> {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.inner.interactivity()
    }
    fn track_focus(mut self, focus: &FocusHandle) -> Self {
        self.focus = Some(focus.clone());
        self.inner = self.inner.track_focus(focus);
        self
    }
}

impl<E: Probe + InteractiveElement + gpui::ParentElement> gpui::ParentElement for Testable<E> {
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
        self.inner.extend(elements);
    }
}

impl<E: Probe + gpui::StatefulInteractiveElement> gpui::StatefulInteractiveElement for Testable<E> {}
