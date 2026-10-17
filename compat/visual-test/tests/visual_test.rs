//! gpui-testing-library and gpui-visual-test on one window: interact through
//! a `Screen` on gpui-visual-test's headless app, then capture real pixels.
//! Needs GPUI's headless renderer (Vulkan on Linux, lavapipe in CI).
use gpui::{
    AppContext as _, Context, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    Role, StatefulInteractiveElement as _, Styled as _, Window, div, px, rgb, text,
};
use gpui_testing_library::{Queries as _, Screen, TestableExt, TestableTextExt, by, expect};
use gpui_visual_test::capture;

#[derive(Default)]
struct Counter {
    count: usize,
}

impl Render for Counter {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0xffffff))
            .flex()
            .flex_col()
            .child(text!(id = "count", format!("Count {}", self.count)).testable())
            .child(
                div()
                    .id("increment")
                    .role(Role::Button)
                    .aria_label("Increment")
                    .testable()
                    .w(px(120.))
                    .h(px(40.))
                    .bg(rgb(0x2563eb))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.count += 1;
                        cx.notify();
                    })),
            )
    }
}

#[test]
fn interact_then_capture_the_same_window() {
    let mut cx = capture::headless_app(None).expect("a headless renderer");
    let handle = cx
        .update(|cx| {
            cx.open_window(capture::window_options(300., 200.), |_, cx| {
                cx.new(|_| Counter::default())
            })
        })
        .unwrap();
    let screen = Screen::headless(&cx, handle);

    let increment = screen.get_by_role(by::role(Role::Button).name("Increment"));
    screen.user().click(&increment);
    screen.user().dbl_click(&increment);
    expect(screen.get_by_text("Count 3")).to_be_visible();
    assert_eq!(screen.read_app(|cx| handle.read(cx).unwrap().count), 3);

    // Node bounds become named regions of the screenshot.
    let bounds = increment.snapshot().visible_bounds();
    let shot = capture::capture_window(&mut cx, handle.into(), "compat/counter")
        .unwrap()
        .with_region(
            "increment",
            bounds.origin.x.into(),
            bounds.origin.y.into(),
            bounds.size.width.into(),
            bounds.size.height.into(),
        );
    assert_eq!(shot.logical_size(), (300., 200.));
    shot.validate().unwrap();
    let scale = shot.scale();
    let center = |value: f32| (value * scale) as u32;
    let pixel = shot.image().get_pixel(
        center(f32::from(bounds.center().x)),
        center(f32::from(bounds.center().y)),
    );
    assert_eq!(pixel.0[..3], [0x25, 0x63, 0xeb], "the button is painted");
}

#[test]
fn headless_screens_wait_on_the_test_clock() {
    let mut cx = capture::headless_app(None).expect("a headless renderer");
    let handle = cx
        .update(|cx| {
            cx.open_window(capture::window_options(300., 200.), |_, cx| {
                cx.new(|_| Counter::default())
            })
        })
        .unwrap();
    let screen = Screen::headless(&cx, handle);
    screen.update_app(|cx| {
        cx.spawn(async move |cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(500))
                .await;
            handle
                .update(cx, |counter, _, cx| {
                    counter.count = 9;
                    cx.notify();
                })
                .ok();
        })
        .detach()
    });
    let found = cx
        .foreground_executor()
        .block_test(screen.find_by_text("Count 9"));
    expect(&found).to_be_visible();
    screen.unmount();
    expect(&found).not().to_be_in_the_document();
}

#[test]
#[should_panic(expected = "needs a TestAppContext")]
fn upload_explains_it_needs_the_test_platform() {
    let mut cx = capture::headless_app(None).expect("a headless renderer");
    let handle = cx
        .update(|cx| {
            cx.open_window(capture::window_options(300., 200.), |_, cx| {
                cx.new(|_| Counter::default())
            })
        })
        .unwrap();
    let screen = Screen::headless(&cx, handle);
    let increment = screen.get_by_label_text("Increment");
    screen.user().upload(&increment, ["a.txt"]);
}
