//! Testing Library APIs against plain GPUI views.
use gpui::{
    Context, FocusHandle, InteractiveElement as _, IntoElement, ParentElement as _, Render, Role,
    StatefulInteractiveElement as _, Styled as _, TestAppContext, Window, div,
    prelude::FluentBuilder as _, px, size, text,
};
use gpui_testing_library::{
    Queries, RenderOptions, TestableExt, TestableTextExt, TextMatch, by, expect, render,
    render_with, wait_for_element_to_be_removed, within,
};
use std::time::Duration;

struct Form {
    count: usize,
    agreed: bool,
    show_details: bool,
    keys: Vec<String>,
    name_focus: FocusHandle,
    email_focus: FocusHandle,
}

impl Form {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            count: 0,
            agreed: false,
            show_details: false,
            keys: Vec::new(),
            name_focus: cx.focus_handle().tab_stop(true),
            email_focus: cx.focus_handle().tab_stop(true),
        }
    }
}

impl Render for Form {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("form")
            .role(Role::Form)
            .aria_label("Sign up")
            .testable()
            .flex()
            .flex_col()
            .size_full()
            .child(text!(format!("Clicked {} times", self.count)).testable())
            .child(
                div()
                    .id("increment")
                    .role(Role::Button)
                    .aria_label("Increment")
                    .aria_description("Adds one")
                    .accessibility_id("increment-button")
                    .testable()
                    .h(px(30.))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.count += 1;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("agree")
                    .role(Role::CheckBox)
                    .aria_label("I agree")
                    .aria_toggled(self.agreed.into())
                    .testable()
                    .h(px(30.))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.agreed = !this.agreed;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("name")
                    .role(Role::TextInput)
                    .aria_label("Name")
                    .aria_placeholder("Your name")
                    .aria_value(self.keys.concat())
                    .testable()
                    .track_focus(&self.name_focus)
                    .h(px(30.))
                    .on_click(cx.listener(|this, _, window, cx| {
                        window.focus(&this.name_focus, cx);
                    }))
                    .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                        if let Some(text) = &event.keystroke.key_char {
                            this.keys.push(text.clone());
                        } else {
                            this.keys.push(format!("<{}>", event.keystroke.key));
                        }
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("email")
                    .role(Role::EmailInput)
                    .aria_label("Email")
                    .testable()
                    .track_focus(&self.email_focus)
                    .h(px(30.)),
            )
            .child(
                div()
                    .id("toggle-details")
                    .role(Role::Button)
                    .aria_label("Details")
                    .aria_expanded(self.show_details)
                    .testable()
                    .h(px(30.))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_details = !this.show_details;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("details")
                    .role(Role::Region)
                    .aria_label("More")
                    .testable()
                    .when_hidden(!self.show_details)
                    .child(
                        div()
                            .id("details-ok")
                            .role(Role::Button)
                            .aria_label("Ok")
                            .testable()
                            .h(px(20.)),
                    ),
            )
            .when(self.count < 100, |this| {
                this.child(
                    div()
                        .id("load")
                        .role(Role::Button)
                        .aria_label("Load")
                        .testable()
                        .h(px(30.))
                        .on_click(cx.listener(|_, _, _, cx| {
                            cx.spawn(async move |this, cx| {
                                cx.background_executor()
                                    .timer(Duration::from_millis(300))
                                    .await;
                                this.update(cx, |this, cx| {
                                    this.count = 100;
                                    cx.notify();
                                })
                                .ok();
                            })
                            .detach();
                        })),
                )
            })
    }
}

trait WhenHidden: gpui::Styled + Sized {
    fn when_hidden(self, hidden: bool) -> Self {
        if hidden { self.invisible() } else { self }
    }
}
impl<T: gpui::Styled> WhenHidden for T {}

fn render_form(cx: &mut TestAppContext) -> gpui_testing_library::RenderResult<Form> {
    render_with(
        cx,
        RenderOptions::new().with_size(size(px(400.), px(600.))),
        |_, cx| Form::new(cx),
    )
}

#[gpui::test]
fn queries_by_role_text_label_and_test_id(cx: &mut TestAppContext) {
    let screen = render_form(cx);

    let button = screen.get_by_role(by::role(Role::Button).name("Increment"));
    expect(&button).to_be_visible();
    expect(&button).to_have_accessible_description("Adds one");
    assert_eq!(screen.get_by_test_id("increment-button"), button);
    assert_eq!(screen.get_by_title("Adds one"), button);
    assert_eq!(screen.get_by_label_text("Increment"), button);
    assert_eq!(screen.get_by_element_id("increment"), button);

    expect(screen.get_by_text("Clicked 0 times")).to_be_in_the_document();
    expect(screen.get_by_text(TextMatch::contains("clicked 0"))).to_be_visible();
    expect(screen.get_by_text(TextMatch::regex(r"^Clicked \d+ times$"))).to_be_visible();
    expect(screen.query_by_text("Clicked"))
        .not()
        .to_be_in_the_document();

    expect(screen.get_by_placeholder_text("Your name")).to_have_role(Role::TextInput);
    assert_eq!(screen.get_all_by_role(Role::Button).len(), 3);
    assert!(screen.query_all_by_role(Role::Slider).is_empty());
}

#[gpui::test]
fn role_queries_skip_hidden_elements_unless_asked(cx: &mut TestAppContext) {
    let screen = render_form(cx);

    assert!(
        screen
            .query_by_role(by::role(Role::Button).name("Ok"))
            .is_none()
    );
    let ok = screen.get_by_role(by::role(Role::Button).name("Ok").hidden(true));
    expect(&ok).not().to_be_visible();
    // Non-role queries include hidden elements, as in Testing Library.
    assert_eq!(screen.get_by_label_text("Ok"), ok);

    let details = screen.get_by_role(by::role(Role::Button).name("Details"));
    expect(&details).to_be_collapsed();
    screen.user().click(&details);
    expect(&details).to_be_expanded();
    expect(&ok).to_be_visible();
}

#[gpui::test]
fn user_click_updates_live_nodes(cx: &mut TestAppContext) {
    let screen = render_form(cx);
    let user = screen.user();

    let checkbox = screen.get_by_role(by::role(Role::CheckBox).name("I agree"));
    expect(&checkbox).not().to_be_checked();
    user.click(&checkbox);
    expect(&checkbox).to_be_checked();
    assert_eq!(
        screen.get_by_role(by::role(Role::CheckBox).checked(true)),
        checkbox
    );

    user.dbl_click(&screen.get_by_role(by::role(Role::Button).name("Increment")));
    expect(screen.get_by_text("Clicked 2 times")).to_be_visible();
    assert_eq!(screen.read(|form, _| form.count), 2);
}

#[gpui::test]
fn user_typing_focuses_and_sends_keys(cx: &mut TestAppContext) {
    let screen = render_form(cx);
    let user = screen.user();
    let name = screen.get_by_label_text("Name");
    expect(&name).not().to_have_focus();

    user.type_text(&name, "Hi {Enter}{ArrowLeft}");

    expect(&name).to_have_focus();
    assert_eq!(
        screen.read(|form, _| form.keys.clone()),
        ["H", "i", " ", "\n", "<left>"]
    );
    expect(&name).to_have_value(TextMatch::exact("Hi \n<left>").without_normalization());
    expect(&name).to_have_display_value(TextMatch::exact("Hi \n<left>").without_normalization());

    user.keyboard("{Shift>}a{/Shift}");
    assert_eq!(
        screen.read(|form, _| form.keys.last().cloned()),
        Some("a".into())
    );
}

#[gpui::test]
fn tab_moves_focus_between_tab_stops(cx: &mut TestAppContext) {
    let screen = render_form(cx);
    let user = screen.user();
    let name = screen.get_by_label_text("Name");
    let email = screen.get_by_label_text("Email");

    user.click(&name);
    user.tab();
    expect(&email).to_have_focus();
    user.tab_backward();
    expect(&name).to_have_focus();
}

#[gpui::test]
fn within_limits_queries_to_descendants(cx: &mut TestAppContext) {
    let screen = render_form(cx);
    let form = screen.get_by_role(by::role(Role::Form).name("Sign up"));
    let details = screen.get_by_role(by::role(Role::Region).hidden(true));

    let scoped = within(&details);
    assert_eq!(scoped.get_all_by_label_text("Ok").len(), 1);
    assert!(scoped.query_by_label_text("Increment").is_none());
    expect(&form).to_contain_element(&scoped.get_by_label_text("Ok"));
    expect(scoped.get_by_label_text("Ok")).to_be_empty_dom_element();
}

#[gpui::test]
async fn find_by_waits_on_the_test_clock(cx: &mut TestAppContext) {
    let screen = render_form(cx);
    screen
        .user()
        .click(&screen.get_by_role(by::role(Role::Button).name("Load")));

    assert!(screen.query_by_text("Clicked 100 times").is_none());
    let loaded = screen.find_by_text("Clicked 100 times").await;
    expect(&loaded).to_be_visible();
}

#[gpui::test]
async fn wait_for_and_removal(cx: &mut TestAppContext) {
    let screen = render_form(cx);
    let load = screen.get_by_role(by::role(Role::Button).name("Load"));
    screen.user().click(&load);

    wait_for_element_to_be_removed(&screen, [load.clone()]).await;
    assert!(!load.is_connected());
    let count = screen
        .wait_for(|| (screen.read(|form, _| form.count) == 100).then_some(100))
        .await;
    assert_eq!(count, 100);
}

#[gpui::test]
#[should_panic(expected = "Unable to find an element with the role Button and name \"Missing\"")]
fn get_reports_missing_elements_with_the_tree(cx: &mut TestAppContext) {
    let screen = render_form(cx);
    screen.get_by_role(by::role(Role::Button).name("Missing"));
}

#[gpui::test]
#[should_panic(expected = "Found multiple elements with the role Button")]
fn get_rejects_ambiguous_matches(cx: &mut TestAppContext) {
    let screen = render_form(cx);
    screen.get_by_role(Role::Button);
}

#[gpui::test]
fn pretty_prints_the_observed_tree(cx: &mut TestAppContext) {
    let screen = render_form(cx);
    let pretty = screen.pretty();
    assert!(pretty.starts_with("Form \"Sign up\""), "{pretty}");
    assert!(pretty.contains("\n  Button \"Increment\""), "{pretty}");
    assert!(
        pretty.contains("CheckBox \"I agree\" #agree [unchecked]"),
        "{pretty}"
    );
}

#[gpui::test]
fn fire_event_dispatches_single_events(cx: &mut TestAppContext) {
    let screen = render(cx, |_, cx| Form::new(cx));
    let increment = screen.get_by_role(by::role(Role::Button).name("Increment"));
    // A press without a release is not a click.
    screen.fire_event().mouse_move(&increment);
    screen.fire_event().mouse_down(&increment);
    assert_eq!(screen.read(|form, _| form.count), 0);
    screen.fire_event().mouse_up(&increment);
    assert_eq!(screen.read(|form, _| form.count), 1);
    screen.fire_event().click(&increment);
    assert_eq!(screen.read(|form, _| form.count), 2);
}

#[gpui::test]
fn unmount_removes_everything(cx: &mut TestAppContext) {
    let screen = render(cx, |_, cx| Form::new(cx));
    screen.get_by_label_text("Increment");
    screen.unmount();
    assert!(cx.windows().is_empty());
}
