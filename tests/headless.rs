//! A `Screen` over a `HeadlessAppContext`, the app gpui-visual-test renders
//! pixels with, and observations read while holding the window.
use gpui::{
    AppContext as _, Context, HeadlessAppContext, InteractiveElement as _, IntoElement,
    NoopTextSystem, ParentElement as _, Render, Role, StatefulInteractiveElement as _, Styled as _,
    TestAppContext, Window, div, px, size,
};
use gpui_testing_library::{
    Queries, RenderOptions, Screen, TestableExt, by, expect, observed, observed_matching,
    render_with,
};
use std::{sync::Arc, time::Duration};

#[derive(Default)]
struct Toggle {
    on: bool,
}

impl Render for Toggle {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .id("toggle")
                    .role(Role::Switch)
                    .aria_label("Power")
                    .aria_toggled(self.on.into())
                    .testable()
                    .h(px(30.))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.on = !this.on;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("hidden")
                    .role(Role::Button)
                    .aria_label("Hidden")
                    .testable()
                    .invisible()
                    .h(px(30.)),
            )
    }
}

fn headless() -> (HeadlessAppContext, Screen, gpui::WindowHandle<Toggle>) {
    let mut cx = HeadlessAppContext::new(Arc::new(NoopTextSystem));
    let handle = cx
        .open_window(size(px(200.), px(200.)), |_, cx| {
            cx.new(|_| Toggle::default())
        })
        .unwrap();
    let screen = Screen::headless(&cx, handle);
    (cx, screen, handle)
}

#[test]
fn headless_screens_query_and_interact() {
    let (_cx, screen, handle) = headless();
    let power = screen.get_by_role(by::role(Role::Switch).name("Power"));
    screen.user().click(&power);
    expect(&power).to_be_checked();
    assert!(screen.read_app(|cx| handle.read(cx).unwrap().on));
    screen.update_app(|cx| {
        handle
            .update(cx, |toggle, _, cx| {
                toggle.on = false;
                cx.notify();
            })
            .unwrap()
    });
    expect(&power).not().to_be_checked();

    screen.user().paste("clip");
    assert_eq!(
        screen
            .read_app(|cx| cx.read_from_clipboard())
            .and_then(|item| item.text()),
        Some("clip".into())
    );
    screen.executor().advance_clock(Duration::from_millis(10));
    screen.run_until_parked();
    screen.unmount();
    expect(&power).not().to_be_in_the_document();
}

#[test]
#[should_panic(expected = "drives a HeadlessAppContext")]
fn headless_screens_have_no_test_context() {
    let (_cx, screen, _) = headless();
    screen.cx();
}

#[gpui::test]
fn observed_reads_the_last_frame_while_holding_the_window(cx: &mut TestAppContext) {
    let screen = render_with(
        cx,
        RenderOptions::new().with_size(size(px(200.), px(200.))),
        |_, _| Toggle::default(),
    );
    let (all, visible_buttons, hidden_buttons) = screen.update(|window, _| {
        (
            observed(window).len(),
            observed_matching(window, by::role(Role::Button)).len(),
            observed_matching(window, by::role(Role::Button).hidden(true)).len(),
        )
    });
    assert_eq!((all, visible_buttons, hidden_buttons), (2, 0, 1));
    assert_eq!(screen.elements().len(), all);
}
