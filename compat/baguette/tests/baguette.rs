//! Baguette's Elixir view trees, rendered by `baguette_view` without the host
//! or the BEAM, queried by role and text and clicked like any GPUI view.
use std::cell::RefCell;
use std::rc::Rc;

use baguette_view::{Event, Frame, Root};
use gpui::{AppContext as _, Role, TestAppContext};
use gpui_testing_library::{Queries as _, by, expect, kit, render};

/// The tree `Baguette.View` sends for a counter at `count`.
fn counter(count: u32) -> Frame {
    serde_json::from_str(&format!(
        r##"{{
          "theme": {{"primary": "#2563eb"}},
          "root": {{"t": "div", "a": {{"class": "size-full flex-col gap-2 p-4"}}, "c": [
            {{"t": "text", "c": ["Count: {count}"]}},
            {{"t": "button", "a": {{"phx-click": "inc", "phx-value-by": "1"}}, "c": ["+"]}},
            {{"t": "button", "a": {{"aria-label": "Reset", "phx-click": "reset", "disabled": ""}}, "c": ["0"]}}
          ]}}
        }}"##
    ))
    .unwrap()
}

type Events = Rc<RefCell<Vec<Event>>>;

fn sink(events: &Events) -> impl Fn(Event) + 'static {
    let events = events.clone();
    move |event| events.borrow_mut().push(event)
}

fn inc() -> Event {
    Event::Click {
        window: 0,
        event: "inc".into(),
        values: [("by".into(), "1".into())].into(),
    }
}

#[gpui::test]
fn elixir_trees_are_queryable(cx: &mut TestAppContext) {
    let events = Events::default();
    let screen = render(cx, {
        let sink = sink(&events);
        move |_, _| Root::new(0, sink).with_frame(counter(0))
    });

    expect(screen.get_by_text("Count: 0")).to_be_visible();
    // A button without `aria-label` is named by its text.
    let plus = screen.get_by_role(by::role(Role::Button).name("+"));
    let reset = screen.get_by_role(by::role(Role::Button).name("Reset"));
    // `disabled` stops clicks but is not reported: baguette renders plain
    // divs, and GPUI's div has no AccessKit disabled setter.
    screen.user().click(&reset);
    screen.user().click(&plus);
    assert_eq!(*events.borrow(), [inc()]);
}

/// The test plays the server: every event comes back as a new frame.
#[gpui::test]
fn events_round_trip_through_a_fake_server(cx: &mut TestAppContext) {
    let events = Events::default();
    let screen = render(cx, {
        let sink = sink(&events);
        move |_, _| Root::new(0, sink).with_frame(counter(0))
    });
    let mut count = 0;
    for _ in 0..3 {
        screen
            .user()
            .click(&screen.get_by_role(by::role(Role::Button).name("+")));
        for event in events.borrow_mut().drain(..) {
            assert_eq!(event, inc());
            count += 1;
        }
        screen.rerender(|root, _, cx| {
            root.set_frame(counter(count));
            cx.notify();
        });
    }
    expect(screen.get_by_text("Count: 3")).to_be_visible();
    assert!(screen.query_by_text("Count: 0").is_none());
}

/// Frames that arrive later, as they do from the BEAM, are awaited with `find_by_*`.
#[gpui::test]
async fn find_by_waits_for_a_late_frame(cx: &mut TestAppContext) {
    let screen = render(cx, |_, _| Root::new(0, |_| {}));
    assert!(screen.query_by_text("Count: 5").is_none());
    let window = screen.window();
    screen.update_app(|cx| {
        cx.spawn(async move |cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(200))
                .await;
            window
                .update(cx, |view, _, cx| {
                    view.downcast::<Root>().unwrap().update(cx, |root, cx| {
                        root.set_frame(counter(5));
                        cx.notify();
                    })
                })
                .ok();
        })
        .detach()
    });
    expect(&screen.find_by_text("Count: 5").await).to_be_visible();
}

/// tauri-gpui's `attach_gpui_view` shows the view inside gpui-base's Root.
#[gpui::test]
fn trees_render_inside_the_kit_root(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let events = Events::default();
    let screen = kit::render(cx, {
        let sink = sink(&events);
        move |_, cx| cx.new(|_| Root::new(0, sink).with_frame(counter(0)))
    });
    screen
        .user()
        .click(&screen.get_by_role(by::role(Role::Button).name("+")));
    assert_eq!(*events.borrow(), [inc()]);
    expect(screen.get_by_text("Count: 0")).to_be_visible();
}
