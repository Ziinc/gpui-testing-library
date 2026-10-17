//! gpui-testing-library inside gpui-proptest: elements marked for both,
//! queries in invariants, and testable elements as generated targets.
use gpui::{
    App, AppContext as _, Context, InteractiveElement as _, IntoElement, ParentElement as _,
    Render, Role, StatefulInteractiveElement as _, Styled as _, Window, div, px, text,
};
use gpui_kit::component::button::Button;
use gpui_proptest::{Config, Failure, Harness, ProptestTargetExt, TargetInfo, TargetSource};
use gpui_testing_library::{
    Queries as _, TestableExt, TestableTextExt, TextMatch, by, observed, observed_matching,
};

#[derive(Default)]
struct Counter {
    count: usize,
}

impl Render for Counter {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(text!(id = "count", format!("Count {}", self.count)).testable())
            .child(
                div()
                    .id("increment")
                    .role(Role::Button)
                    .aria_label("Increment")
                    // `.testable()` first: proptest targets wrap any element,
                    // but only interactive elements can be made testable.
                    .testable()
                    .h(px(30.))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.count += 1;
                        cx.notify();
                    }))
                    .proptest_target(),
            )
            .child(
                div()
                    .id("reset")
                    .role(Role::Button)
                    .aria_label("Reset")
                    .testable()
                    .h(px(30.))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.count = 0;
                        cx.notify();
                    }))
                    .proptest_target(),
            )
    }
}

fn config() -> Config {
    Config::default().with_cases(32).with_persist_failures(false)
}

fn fail(message: String) -> Result<(), Failure> {
    Err(Failure::new(message))
}

#[test]
fn queries_run_inside_invariants() {
    Harness::new(|_, cx| cx.new(|_| Counter::default()))
        .invariant("the label shows the count", |counter, _, window, cx| {
            let expected = format!("Count {}", counter.read(cx).count);
            let labels = observed_matching(window, by::text(TextMatch::regex("^Count")));
            match labels.as_slice() {
                [label] if label.text() == Some(expected.as_str()) => Ok(()),
                _ => fail(format!("expected one {expected:?}, observed {labels:?}")),
            }
        })
        .invariant("both buttons stay visible", |_, _, window, _| {
            let buttons = observed_matching(window, by::role(Role::Button));
            if buttons.len() == 2 {
                Ok(())
            } else {
                fail(format!("{} visible buttons", buttons.len()))
            }
        })
        .check(config());
}

/// Two independent observers of the same frame must agree.
#[test]
fn testable_and_target_observations_agree() {
    Harness::new(|_, cx| cx.new(|_| Counter::default()))
        .invariant("same paths, bounds and visibility", |_, targets, window, _| {
            let snapshots = observed(window);
            for target in targets {
                let Some(snapshot) = snapshots.iter().find(|s| s.path() == target.path()) else {
                    return fail(format!("{target:?} is not observed by testing-library"));
                };
                if snapshot.bounds() != target.bounds()
                    || snapshot.is_visible() != target.is_visible()
                {
                    return fail(format!("{target:?} disagrees with {snapshot:?}"));
                }
            }
            Ok(())
        })
        .check(config());
}

/// Elements marked only `.testable()` as gpui-proptest targets.
struct TestableTargets;

impl TargetSource for TestableTargets {
    fn targets(&self, window: &Window, _: &App) -> Vec<TargetInfo> {
        observed(window)
            .into_iter()
            .filter(|snapshot| snapshot.role() == Some(Role::Button))
            .map(|snapshot| {
                TargetInfo::new(snapshot.path().to_vec(), snapshot.visible_bounds())
                    .with_visible(snapshot.is_visible())
                    .with_disabled(snapshot.is_disabled())
                    .with_focused(snapshot.focused())
                    .with_role(snapshot.role())
                    .with_label(snapshot.name().map(str::to_owned))
            })
            .collect()
    }
}

struct Plain {
    clicks: usize,
}

impl Render for Plain {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            div()
                .id("plain")
                .role(Role::Button)
                .aria_label("Plain")
                .testable()
                .h(px(30.))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.clicks += 1;
                    cx.notify();
                })),
        )
    }
}

#[test]
fn testable_elements_can_be_targets() {
    let report = Harness::new(|_, cx| cx.new(|_| Plain { clicks: 0 }))
        .with_target_source(TestableTargets)
        .invariant("never clicked", |plain, _, _, cx| {
            if plain.read(cx).clicks == 0 {
                Ok(())
            } else {
                fail("clicked".into())
            }
        })
        .search(config())
        .expect_err("generated clicks reach a `.testable()`-only element");
    assert!(report.failure().message().contains("clicked"), "{report}");
}

struct Form;

impl Render for Form {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(Button::new("save").label("Save"))
    }
}

#[test]
fn kit_harness_and_kit_snapshots() {
    gpui_proptest::kit::harness(|_, cx| cx.new(|_| Form))
        .invariant("the kit button is queryable", |_, _, window, _| {
            let save = observed_matching(window, by::role(Role::Button).name("Save"));
            if save.len() == 1 {
                Ok(())
            } else {
                fail(format!("observed {save:?}"))
            }
        })
        .check(config());
}

#[gpui::test]
fn the_same_view_works_under_render(cx: &mut gpui::TestAppContext) {
    let screen = gpui_testing_library::render(cx, |_, _| Counter::default());
    screen
        .user()
        .click(&screen.get_by_role(by::role(Role::Button).name("Increment")));
    screen.get_by_text("Count 1");
}
