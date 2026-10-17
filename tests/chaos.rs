//! Chaos tests: random interaction sequences checked against a model, plus
//! the edge cases they and targeted probes turned up.
//!
//! Each proptest case runs on a fresh `TestAppContext` seeded like
//! `#[gpui::test(seed)]`, so a failure shrinks to a short, replayable list of
//! steps. `PROPTEST_CASES` raises the case count.
use gpui::{
    Context, ElementId, FocusHandle, InteractiveElement as _, IntoElement, ParentElement as _,
    Render, Role, StatefulInteractiveElement as _, Styled as _, TestAppContext, TestDispatcher,
    Window, div, prelude::FluentBuilder as _, px, size, text,
};
use gpui_testing_library::{
    Queries, RenderOptions, RenderResult, TestableExt, TestableTextExt, TextMatch, by, expect,
    render_with,
};
use proptest::prelude::*;

const ROW: f32 = 30.;
const VIEWPORT: f32 = 100.;

struct Chaos {
    items: Vec<(usize, bool)>,
    next_id: usize,
    field: String,
    field_focus: FocusHandle,
    other_focus: FocusHandle,
    hidden: bool,
    offset: f32,
}

impl Chaos {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            items: Vec::new(),
            next_id: 0,
            field: String::new(),
            field_focus: cx.focus_handle().tab_stop(true),
            other_focus: cx.focus_handle().tab_stop(true),
            hidden: false,
            offset: 0.,
        }
    }
}

fn item_id(id: usize) -> ElementId {
    ("item", id).into()
}

impl Render for Chaos {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .id("add")
                    .role(Role::Button)
                    .aria_label("Add")
                    .testable()
                    .h(px(20.))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.items.push((this.next_id, false));
                        this.next_id += 1;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("remove")
                    .role(Role::Button)
                    .aria_label("Remove first")
                    .testable()
                    .h(px(20.))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if !this.items.is_empty() {
                            this.items.remove(0);
                        }
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("field")
                    .role(Role::TextInput)
                    .aria_label("Field")
                    .aria_value(self.field.clone())
                    .testable()
                    .track_focus(&self.field_focus)
                    .h(px(20.))
                    .on_click(cx.listener(|this, _, window, cx| {
                        window.focus(&this.field_focus, cx);
                    }))
                    .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                        if event.keystroke.key == "backspace" {
                            this.field.pop();
                        } else if let Some(text) = &event.keystroke.key_char
                            && !text.chars().any(char::is_control)
                        {
                            // Tab and Enter carry "\t" and "\n" as GPUI simulates them.
                            this.field.push_str(text);
                        }
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("other")
                    .role(Role::TextInput)
                    .aria_label("Other")
                    .testable()
                    .track_focus(&self.other_focus)
                    .h(px(20.)),
            )
            .child(
                div()
                    .id("viewport")
                    .role(Role::List)
                    .aria_label("Items")
                    .testable()
                    .h(px(VIEWPORT))
                    .overflow_hidden()
                    .when(self.hidden, |this| this.invisible())
                    .child(
                        div()
                            .relative()
                            .top(px(-self.offset))
                            .children(self.items.iter().map(|&(id, checked)| {
                                div()
                                    .id(item_id(id))
                                    .role(Role::CheckBox)
                                    .aria_label(format!("Item {id}"))
                                    .aria_toggled(checked.into())
                                    .testable()
                                    .h(px(ROW))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if let Some(item) =
                                            this.items.iter_mut().find(|item| item.0 == id)
                                        {
                                            item.1 = !item.1;
                                        }
                                        cx.notify();
                                    }))
                            })),
                    ),
            )
    }
}

#[derive(Clone, Debug)]
enum Op {
    Add,
    RemoveFirst,
    /// Clicks the nth visible item, wrapping.
    ClickItem(usize),
    Type(String),
    Backspace,
    Tab,
    TabBackward,
    HoldShift,
    ReleaseShift,
    Scroll(u8),
    ToggleHidden,
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        3 => Just(Op::Add),
        1 => Just(Op::RemoveFirst),
        4 => any::<usize>().prop_map(Op::ClickItem),
        2 => "[a-z ]{0,4}".prop_map(Op::Type),
        1 => Just(Op::Backspace),
        1 => Just(Op::Tab),
        1 => Just(Op::TabBackward),
        1 => Just(Op::HoldShift),
        1 => Just(Op::ReleaseShift),
        2 => any::<u8>().prop_map(Op::Scroll),
        1 => Just(Op::ToggleHidden),
    ]
}

#[derive(Default)]
struct Model {
    items: Vec<(usize, bool)>,
    next_id: usize,
    field: String,
    hidden: bool,
    offset: f32,
}

impl Model {
    /// Items whose row overlaps the clipped viewport.
    fn visible_items(&self) -> Vec<usize> {
        if self.hidden {
            return Vec::new();
        }
        self.items
            .iter()
            .enumerate()
            .filter(|(row, _)| {
                let top = *row as f32 * ROW - self.offset;
                top < VIEWPORT && top + ROW > 0.
            })
            .map(|(_, item)| item.0)
            .collect()
    }
}

fn item_number(name: impl AsRef<str>) -> usize {
    name.as_ref().trim_start_matches("Item ").parse().unwrap()
}

fn render_chaos(cx: &mut TestAppContext) -> RenderResult<Chaos> {
    render_with(
        cx,
        RenderOptions::new().with_size(size(px(300.), px(300.))),
        |_, cx| Chaos::new(cx),
    )
}

fn run(seed: u64, ops: &[Op]) -> Result<(), TestCaseError> {
    let mut cx = TestAppContext::build(TestDispatcher::new(seed), None);
    let result = drive(&mut cx, ops);
    cx.quit();
    result
}

fn drive(cx: &mut TestAppContext, ops: &[Op]) -> Result<(), TestCaseError> {
    let screen = render_chaos(cx);
    let mut model = Model::default();
    for (step, op) in ops.iter().enumerate() {
        // A fresh `UserEvent` per step, as tests usually write it.
        let user = screen.user();
        match op {
            Op::Add => {
                user.click(&screen.get_by_role(by::role(Role::Button).name("Add")));
                model.items.push((model.next_id, false));
                model.next_id += 1;
            }
            Op::RemoveFirst => {
                user.click(&screen.get_by_role(by::role(Role::Button).name("Remove first")));
                if !model.items.is_empty() {
                    model.items.remove(0);
                }
            }
            Op::ClickItem(index) => {
                let visible = screen.query_all_by_role(Role::CheckBox);
                if let Some(node) = visible.get(index % visible.len().max(1)) {
                    let id = item_number(node.name().unwrap());
                    user.click(node);
                    let item = model.items.iter_mut().find(|item| item.0 == id).unwrap();
                    item.1 = !item.1;
                    prop_assert_eq!(
                        node.snapshot().checked(),
                        Some(item.1),
                        "step {}: {:?}",
                        step,
                        op
                    );
                }
            }
            Op::Type(text) => {
                user.type_text(&screen.get_by_label_text("Field"), text);
                model.field.push_str(text);
            }
            Op::Backspace => {
                user.type_text(&screen.get_by_label_text("Field"), "{Backspace}");
                model.field.pop();
            }
            Op::Tab => user.tab(),
            Op::TabBackward => user.tab_backward(),
            Op::HoldShift => user.keyboard("{Shift>}"),
            Op::ReleaseShift => user.keyboard("{/Shift}"),
            Op::Scroll(offset) => {
                model.offset = f32::from(*offset);
                screen.rerender(|view, _, cx| {
                    view.offset = model.offset;
                    cx.notify();
                });
            }
            Op::ToggleHidden => {
                model.hidden = !model.hidden;
                screen.rerender(|view, _, cx| {
                    view.hidden = model.hidden;
                    cx.notify();
                });
            }
        }
        check(&screen, &model).map_err(|error| {
            TestCaseError::fail(format!(
                "after step {step} ({op:?}): {error}\n\n{}",
                screen.pretty()
            ))
        })?;
    }
    Ok(())
}

/// The library's contract, checked after every step.
fn check(screen: &RenderResult<Chaos>, model: &Model) -> Result<(), String> {
    let ensure = |ok: bool, message: String| if ok { Ok(()) } else { Err(message) };

    let every = screen.query_all_by_role(by::role(Role::CheckBox).hidden(true));
    let observed: Vec<_> = every
        .iter()
        .map(|node| {
            (
                item_number(node.name().unwrap()),
                node.snapshot().checked() == Some(true),
            )
        })
        .collect();
    ensure(
        observed == model.items,
        format!("items {observed:?} != model {:?}", model.items),
    )?;

    let visible: Vec<_> = screen
        .query_all_by_role(Role::CheckBox)
        .iter()
        .map(|node| item_number(node.name().unwrap()))
        .collect();
    ensure(
        visible == model.visible_items(),
        format!("visible {visible:?} != model {:?}", model.visible_items()),
    )?;

    let field = screen.get_by_label_text("Field");
    ensure(
        field.snapshot().display_value().unwrap_or_default() == model.field,
        format!(
            "field {:?} != model {:?}",
            field.snapshot().display_value(),
            model.field
        ),
    )?;

    let elements = screen.elements();
    let focused = elements.iter().filter(|node| node.is_focused()).count();
    ensure(focused <= 1, format!("{focused} elements report focus"))?;

    let snapshots: Vec<_> = elements.iter().map(|node| node.snapshot()).collect();
    for (index, snapshot) in snapshots.iter().enumerate() {
        ensure(
            !snapshots[..index]
                .iter()
                .any(|other| other.path() == snapshot.path()),
            format!("duplicate path {:?}", snapshot.path()),
        )?;
        if let Some(previous) = index.checked_sub(1).map(|index| &snapshots[index]) {
            let order =
                |s: &gpui_testing_library::Snapshot| (s.bounds().origin.y, s.bounds().origin.x);
            ensure(
                order(previous) <= order(snapshot),
                format!("{previous:?} is listed before {snapshot:?}"),
            )?;
        }
        let clip = snapshot.visible_bounds();
        ensure(
            snapshot.is_visible() == !clip.is_empty()
                && (clip.is_empty() || snapshot.bounds().intersect(&clip) == clip),
            format!("visible bounds {clip:?} disagree with {snapshot:?}"),
        )?;
        ensure(
            screen
                .get_by_element_id(snapshot.element_id().clone())
                .snapshot()
                .path()
                == snapshot.path(),
            format!("element id of {snapshot:?} does not resolve to it"),
        )?;
    }

    let lines = screen.pretty().lines().count();
    ensure(
        lines == elements.len(),
        format!(
            "pretty() printed {lines} lines for {} elements",
            elements.len()
        ),
    )?;
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    #[test]
    fn random_interactions_match_the_model(
        seed in any::<u64>(),
        ops in proptest::collection::vec(op(), 1..40),
    ) {
        run(seed, &ops)?;
    }
}

// Regressions the chaos tests found.

#[gpui::test]
fn clicks_land_on_the_visible_part_of_a_clipped_element(cx: &mut TestAppContext) {
    let screen = render_chaos(cx);
    let add = screen.get_by_role(by::role(Role::Button).name("Add"));
    for _ in 0..4 {
        screen.user().click(&add);
    }
    // Item 1 spans 10..40 of the viewport's 0..100 once scrolled by 20, so
    // the center of its full bounds would hit item 0 above it.
    screen.rerender(|view, _, cx| {
        view.offset = 20.;
        cx.notify();
    });
    let first = screen.get_by_role(by::role(Role::CheckBox).name("Item 0"));
    let clip = first.snapshot().visible_bounds();
    assert_eq!(clip.size.height, px(10.));
    screen.user().click(&first);
    expect(&first).to_be_checked();
    expect(screen.get_by_role(by::role(Role::CheckBox).name("Item 1")))
        .not()
        .to_be_checked();
}

#[gpui::test]
fn a_new_user_event_inherits_held_modifiers(cx: &mut TestAppContext) {
    let screen = render_chaos(cx);
    let field = screen.get_by_label_text("Field");
    screen.user().keyboard("{Control>}");
    // Ctrl is still held in the window, so this types no character, as the
    // clicks of any later UserEvent already behaved.
    screen.user().type_text(&field, "a");
    expect(&field).to_have_display_value(TextMatch::exact("").without_normalization());
    screen.user().keyboard("{/Control}");
    screen.user().type_text(&field, "a");
    expect(&field).to_have_display_value("a");
}

#[gpui::test]
#[should_panic(expected = "missing key name in {>2}")]
fn repeat_without_a_key_name_is_rejected(cx: &mut TestAppContext) {
    render_chaos(cx).user().keyboard("{>2}");
}

#[gpui::test]
#[should_panic(expected = "missing key name in {/}")]
fn release_without_a_key_name_is_rejected(cx: &mut TestAppContext) {
    render_chaos(cx).user().keyboard("{/}");
}

// Limitations the chaos tests found. They follow from GPUI and are documented
// in the README; these tests pin the current behavior.

struct Edges {
    words: bool,
}

impl Render for Edges {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .id("same")
                    .role(Role::Button)
                    .aria_label("First")
                    .testable()
                    .h(px(20.)),
            )
            .child(
                div()
                    .id("same")
                    .role(Role::Button)
                    .aria_label("Second")
                    .testable()
                    .h(px(20.)),
            )
            .when(self.words, |this| {
                this.children(["one", "two"].map(|word| text!(word).testable()))
                    .children(["three", "four"].map(|word| text!(id = word, word).testable()))
            })
            .child(
                div().opacity(0.).child(
                    div()
                        .id("faded")
                        .role(Role::Button)
                        .aria_label("Faded")
                        .testable()
                        .h(px(20.)),
                ),
            )
    }
}

#[gpui::test]
fn siblings_with_one_id_are_observed_once(cx: &mut TestAppContext) {
    let screen = render_with(cx, RenderOptions::new(), |_, _| Edges { words: true });
    // GPUI keys element state by id path, so the last sibling wins.
    let same = screen.get_all_by_element_id("same");
    assert_eq!(same.len(), 1);
    expect(&same[0]).to_have_accessible_name("Second");
    // `text!` in a loop reuses one call-site id; give each its own id instead.
    assert_eq!(
        screen
            .query_all_by_text(TextMatch::regex("^(one|two)$"))
            .len(),
        1
    );
    assert_eq!(
        screen
            .get_all_by_text(TextMatch::regex("^(three|four)$"))
            .len(),
        2
    );
}

#[gpui::test]
fn ancestor_opacity_does_not_hide_descendants(cx: &mut TestAppContext) {
    let screen = render_with(cx, RenderOptions::new(), |_, _| Edges { words: false });
    // GPUI keeps the inherited opacity private, so only an element's own
    // opacity is detected.
    expect(screen.get_by_label_text("Faded")).to_be_visible();
}
