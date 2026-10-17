//! Failure messages, less common query forms, and the lifecycle edges chaos
//! testing exercises: nodes outliving their window, re-parenting, timeouts.
use gpui::{
    Context, ElementId, FocusHandle, InteractiveElement as _, IntoElement, ParentElement as _,
    Render, Role, StatefulInteractiveElement as _, Styled as _, TestAppContext, Window, div,
    prelude::FluentBuilder as _, px,
};
use gpui_testing_library::{
    PointerAction, Queries, RenderOptions, RenderResult, Source, TestableExt, WaitOptions, by,
    configure, expect, get_config, render, render_with, wait_for, wait_for_element_to_be_removed,
    within,
};
use std::time::Duration;

struct Panel {
    focus: FocusHandle,
    keys: Vec<String>,
    wrapped: bool,
    picked: Option<&'static str>,
}

impl Panel {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
            keys: Vec::new(),
            wrapped: false,
            picked: None,
        }
    }
}

impl Render for Panel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let movable = div()
            .id("movable")
            .role(Role::Button)
            .aria_label("Movable")
            .testable()
            .h(px(20.));
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(div().id("untitled").testable().h(px(20.)))
            .child(
                div()
                    .id(ElementId::NamedInteger("row".into(), 7))
                    .role(Role::Button)
                    .aria_label("Row")
                    .testable()
                    .h(px(20.)),
            )
            .child(
                div()
                    .id("name")
                    .role(Role::TextInput)
                    .aria_label("Name")
                    .aria_description("Full name")
                    .aria_placeholder("Your name")
                    .aria_value("Ada")
                    .accessibility_id("name-id")
                    .testable()
                    .track_focus(&self.focus)
                    .h(px(20.))
                    .on_click(cx.listener(|this, _, window, cx| window.focus(&this.focus, cx)))
                    .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                        this.keys.push(event.keystroke.to_string());
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("logo")
                    .role(Role::Image)
                    .aria_label("Logo")
                    .testable()
                    .h(px(20.)),
            )
            .child(
                div()
                    .id("tree")
                    .role(Role::TreeItem)
                    .aria_label("Node")
                    .aria_expanded(true)
                    .testable()
                    .h(px(20.)),
            )
            .child(
                div()
                    .id("colors")
                    .role(Role::ListBox)
                    .aria_label("Colors")
                    .testable()
                    .child(
                        div()
                            .id("red")
                            .role(Role::ListBoxOption)
                            .aria_label("Red")
                            .aria_selected(self.picked == Some("Red"))
                            .testable()
                            .h(px(20.))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.picked = Some("Red");
                                cx.notify();
                            })),
                    ),
            )
            .map(|this| {
                if self.wrapped {
                    this.child(div().id("wrapper").child(movable))
                } else {
                    this.child(movable)
                }
            })
    }
}

fn render_panel(cx: &mut TestAppContext) -> RenderResult<Panel> {
    render_with(cx, RenderOptions::new(), |_, cx| Panel::new(cx))
}

#[gpui::test]
fn every_text_query_constructor_finds_its_element(cx: &mut TestAppContext) {
    let screen = render_panel(cx);
    let name = screen.get_by_label_text("Name");
    assert_eq!(screen.get(by::placeholder_text("Your name")), name);
    assert_eq!(screen.get(by::display_value("Ada")), name);
    assert_eq!(screen.get(by::title("Full name")), name);
    assert_eq!(screen.get(by::test_id("name-id")), name);
    assert_eq!(
        screen.get(by::label_text("Name").role(Role::TextInput)),
        name
    );
    assert_eq!(
        screen.get(by::role(Role::TextInput).description("Full name")),
        name
    );
    assert_eq!(
        screen.get(by::alt_text("Logo")),
        screen.get(by::element_id("logo"))
    );
    expect(screen.get(by::role(Role::TreeItem).expanded(true))).to_be_expanded();
    assert!(
        screen
            .query(by::label_text("Name").role(Role::Button))
            .is_none()
    );
}

#[gpui::test]
fn snapshot_and_node_accessors(cx: &mut TestAppContext) {
    let screen = render_panel(cx);
    let name = screen.get_by_label_text("Name");
    let snapshot = name.snapshot();
    assert_eq!(snapshot.source(), Source::Testable);
    assert_eq!(snapshot.keyboard_shortcut(), None);
    assert!(snapshot.is_focusable());
    let logo = screen.get_by_alt_text("Logo").snapshot();
    assert!(!logo.is_focusable());
    assert!(!logo.is_clickable());
    assert_eq!(name.path(), snapshot.path());
    assert!(!name.is_focused());
    screen.user().click(&name);
    assert!(name.is_focused());
    assert_eq!(snapshot.visible_bounds(), snapshot.bounds());
    assert_eq!(screen.view().entity_id(), screen.view().clone().entity_id());
}

#[gpui::test]
fn pretty_prints_elements_without_a_role_or_a_plain_id(cx: &mut TestAppContext) {
    let screen = render_panel(cx);
    let pretty = screen.pretty();
    assert!(pretty.starts_with("<no role> #untitled"), "{pretty}");
    assert!(pretty.contains("Button \"Row\" #NamedInteger("), "{pretty}");
}

#[gpui::test]
fn pretty_explains_an_empty_window(cx: &mut TestAppContext) {
    struct Empty;
    impl Render for Empty {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }
    let screen = render(cx, |_, _| Empty);
    assert!(screen.pretty().starts_with("<no observed elements"));
}

#[gpui::test]
fn keyboard_types_control_characters_and_every_modifier(cx: &mut TestAppContext) {
    let screen = render_panel(cx);
    let user = screen.user();
    user.click(&screen.get_by_label_text("Name"));
    user.keyboard("\n\t{Alt>}{Meta>}{Fn>}x{/Fn}{/Meta}{/Alt}");
    let keys = screen.read(|panel, _| panel.keys.clone());
    assert_eq!(keys[..2], ["enter", "tab"]);
    let last = keys.last().unwrap();
    assert!(
        last.contains("alt-") && last.to_lowercase().ends_with('x'),
        "{keys:?}"
    );
}

#[gpui::test]
fn pointer_moves_to_nodes(cx: &mut TestAppContext) {
    let screen = render_panel(cx);
    let red = screen.get_by_role(by::role(Role::ListBoxOption).name("Red"));
    screen.user().pointer([
        PointerAction::MoveTo(Box::new(red.clone())),
        PointerAction::Press(gpui::MouseButton::Left),
        PointerAction::Release(gpui::MouseButton::Left),
    ]);
    expect(&red).to_be_selected();
}

#[gpui::test]
fn select_options_clicks_an_option_that_is_already_shown(cx: &mut TestAppContext) {
    let screen = render_panel(cx);
    let colors = screen.get_by_role(Role::ListBox);
    screen.user().select_options(&colors, &["Red"]);
    assert_eq!(screen.read(|panel, _| panel.picked), Some("Red"));
}

#[gpui::test]
fn configure_sets_the_polling_interval(cx: &mut TestAppContext) {
    let _ = cx;
    let before = get_config();
    configure(|config| config.with_async_util_interval(Duration::from_millis(5)));
    assert_eq!(get_config().async_util_interval(), Duration::from_millis(5));
    configure(|_| before);
}

#[gpui::test]
async fn wait_for_accepts_booleans(cx: &mut TestAppContext) {
    let screen = render_panel(cx);
    wait_for(&screen, WaitOptions::new(), || true).await;
}

#[gpui::test]
#[should_panic(expected = "wait_for timed out")]
async fn wait_for_times_out_with_the_tree(cx: &mut TestAppContext) {
    let screen = render_panel(cx);
    let options = WaitOptions::new().with_timeout(Duration::from_millis(100));
    wait_for(&screen, options, || false).await;
}

#[gpui::test]
#[should_panic(expected = "Timed out waiting for elements to be removed")]
async fn removal_times_out_while_the_element_stays(cx: &mut TestAppContext) {
    let screen = render_panel(cx);
    configure(|config| config.with_async_util_timeout(Duration::from_millis(100)));
    wait_for_element_to_be_removed(&screen, [screen.get_by_label_text("Name")]).await;
}

// Lifecycle edges.

#[gpui::test]
#[should_panic(expected = "it is no longer rendered")]
fn interacting_after_unmount_names_the_element(cx: &mut TestAppContext) {
    let screen = render_panel(cx);
    let name = screen.get_by_label_text("Name");
    screen.unmount();
    expect(&name).not().to_be_in_the_document();
    assert!(screen.elements().is_empty());
    screen.user().click(&name);
}

#[gpui::test]
#[should_panic(expected = "Unable to find an element with the label text \"Name\"")]
async fn find_by_after_unmount_times_out(cx: &mut TestAppContext) {
    let screen = render_panel(cx);
    screen.unmount();
    screen.find_by_label_text("Name").await;
}

#[gpui::test]
fn reparenting_disconnects_a_node(cx: &mut TestAppContext) {
    let screen = render_panel(cx);
    let movable = screen.get_by_label_text("Movable");
    screen.rerender(|panel, _, cx| {
        panel.wrapped = true;
        cx.notify();
    });
    // Nodes are identified by id path, so moving an element under a new
    // parent id makes it a different node, unlike a moved DOM element.
    assert!(!movable.is_connected());
    let moved = screen.get_by_label_text("Movable");
    assert_ne!(moved, movable);
    screen.rerender(|panel, _, cx| {
        panel.wrapped = false;
        cx.notify();
    });
    // Rendering the old path again reconnects the old node.
    assert!(movable.is_connected());
    assert!(
        within(&screen.get_by_role(Role::ListBox))
            .query_by_label_text("Movable")
            .is_none()
    );
}

// Failure messages.

#[gpui::test]
#[should_panic(
    expected = "the role Button and name \"x\" and description \"d\" and checked: true \
    and pressed: false and selected: true and expanded: true and disabled: false and level 2"
)]
fn role_errors_list_every_option(cx: &mut TestAppContext) {
    render_panel(cx).get(
        by::role(Role::Button)
            .name("x")
            .description("d")
            .checked(true)
            .pressed(false)
            .selected(true)
            .expanded(true)
            .disabled(false)
            .level(2),
    );
}

#[gpui::test]
#[should_panic(expected = "Unable to find an element with the label text \"Nope\"")]
fn label_text_errors_name_the_text(cx: &mut TestAppContext) {
    render_panel(cx).get_by_label_text("Nope");
}

#[gpui::test]
#[should_panic(expected = "Unable to find an element with the element id")]
fn element_id_errors_name_the_id(cx: &mut TestAppContext) {
    render_panel(cx).get_by_element_id("nope");
}

#[gpui::test]
#[should_panic(expected = "Unable to find an element with the text")]
fn get_all_panics_when_nothing_matches(cx: &mut TestAppContext) {
    render_panel(cx).get_all_by_text("nothing");
}

#[gpui::test]
#[should_panic(expected = "expected: Button\nreceived: Some(TextInput)")]
fn to_have_role_reports_both_roles(cx: &mut TestAppContext) {
    let screen = render_panel(cx);
    expect(screen.get_by_label_text("Name")).to_have_role(Role::Button);
}

#[gpui::test]
#[should_panic(expected = "expected: \"Bob\"\nreceived: Some(\"Name\")")]
fn text_matchers_report_both_values(cx: &mut TestAppContext) {
    let screen = render_panel(cx);
    expect(screen.get_by_label_text("Name")).to_have_accessible_name("Bob");
}

#[gpui::test]
#[should_panic(expected = "other element:")]
fn to_contain_element_reports_the_other_element(cx: &mut TestAppContext) {
    let screen = render_panel(cx);
    expect(screen.get_by_label_text("Name")).to_contain_element(&screen.get_by_alt_text("Logo"));
}

#[gpui::test]
#[should_panic(expected = "received 1 observed descendants")]
fn to_be_empty_dom_element_counts_descendants(cx: &mut TestAppContext) {
    let screen = render_panel(cx);
    expect(screen.get_by_role(Role::ListBox)).to_be_empty_dom_element();
}

#[gpui::test]
#[should_panic(expected = "received focused: false")]
fn to_have_focus_reports_focus(cx: &mut TestAppContext) {
    let screen = render_panel(cx);
    expect(screen.get_by_label_text("Name")).to_have_focus();
}

#[gpui::test]
#[should_panic(expected = "the element is not rendered\n\nElement: None")]
fn to_be_in_the_document_reports_a_missing_element(cx: &mut TestAppContext) {
    let screen = render_panel(cx);
    let missing = screen.query_by_label_text("Nope");
    expect(&missing).to_be_in_the_document();
}

#[gpui::test]
#[should_panic(expected = "received visible: false")]
fn not_to_be_visible_reports_visibility(cx: &mut TestAppContext) {
    let screen = render_panel(cx);
    expect(screen.get_by_label_text("Name"))
        .not()
        .to_be_visible();
}

#[gpui::test]
fn negated_matchers_pass_on_absent_state(cx: &mut TestAppContext) {
    let screen = render_panel(cx);
    let name = screen.get_by_label_text("Name");
    expect(&name).not().to_have_role(Role::Button);
    expect(&name)
        .not()
        .to_contain_element(&screen.get_by_alt_text("Logo"));
    expect(screen.get_by_role(Role::ListBox))
        .not()
        .to_be_empty_dom_element();
    expect(&name).not().to_have_focus();
}
