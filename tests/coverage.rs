//! Every public interaction, query and matcher against plain GPUI elements.
use gpui::{
    App, Bounds, ClipboardItem, Context, Div, Element, ElementId, FocusHandle, GlobalElementId,
    Hitbox, InspectorElementId, InteractiveElement, Interactivity, IntoElement, KeyBinding,
    LayoutId, Modifiers, MouseButton, ParentElement as _, PathPromptOptions, Pixels, PlatformInput,
    Render, Role, ScrollDelta, Stateful, StatefulInteractiveElement as _, Styled as _,
    TestAppContext, UTF16Selection, Window, accesskit, actions, div, prelude::FluentBuilder as _,
    px, size, text,
};
use gpui_testing_library::{
    PointerAction, Queries, RenderOptions, RenderResult, Screen, TestableExt, TestableTextExt,
    TextMatch, TypeOptions, UserEvent, UserEventOptions, WaitOptions, act, by, cleanup, configure,
    expect, get_config, render_with, wait_for, within,
};
use std::{ops::Range, path::PathBuf, time::Duration};

actions!(coverage, [Bump, Copy, Paste]);

/// A div that also publishes the AccessKit flags GPUI's div has no setters for.
struct Flags {
    inner: Stateful<Div>,
    required: bool,
    invalid: bool,
    disabled: bool,
    read_only: bool,
}

fn flags(id: &'static str) -> Flags {
    Flags {
        inner: div().id(id),
        required: false,
        invalid: false,
        disabled: false,
        read_only: false,
    }
}

impl Flags {
    fn required(mut self) -> Self {
        self.required = true;
        self
    }
    fn invalid(mut self) -> Self {
        self.invalid = true;
        self
    }
    fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
    fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }
}

impl IntoElement for Flags {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl InteractiveElement for Flags {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.inner.interactivity()
    }
}
impl gpui::StatefulInteractiveElement for Flags {}
impl gpui::Styled for Flags {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.inner.style()
    }
}

impl Element for Flags {
    type RequestLayoutState = <Stateful<Div> as Element>::RequestLayoutState;
    type PrepaintState = Option<Hitbox>;
    fn id(&self) -> Option<ElementId> {
        Element::id(&self.inner)
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
    ) -> Option<Hitbox> {
        self.inner
            .prepaint(id, inspector, bounds, layout, window, cx)
    }
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        prepaint: &mut Option<Hitbox>,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.inner
            .paint(id, inspector, bounds, layout, prepaint, window, cx)
    }
    fn a11y_role(&self) -> Option<accesskit::Role> {
        self.inner.a11y_role()
    }
    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.inner.write_a11y_info(node);
        if self.required {
            node.set_required();
        }
        if self.invalid {
            node.set_invalid(accesskit::Invalid::True);
        }
        if self.disabled {
            node.set_disabled();
        }
        if self.read_only {
            node.set_read_only();
        }
    }
}

#[derive(Default)]
struct Log {
    events: Vec<String>,
    hovered: bool,
    clicks: Vec<usize>,
    color: Option<&'static str>,
    open: bool,
    uploaded: Option<Vec<PathBuf>>,
    pasted: Option<String>,
    debounced: bool,
}

struct Playground {
    log: Log,
    focus: FocusHandle,
}

impl Playground {
    fn push(&mut self, event: impl Into<String>, cx: &mut Context<Self>) {
        self.log.events.push(event.into());
        cx.notify();
    }
}

impl Render for Playground {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Taller than small test windows, so later children can be off-screen.
        div()
            .w_full()
            .h(px(800.))
            .flex()
            .flex_col()
            .child(
                div()
                    .id("pad")
                    .role(Role::Canvas)
                    .aria_label("Pad")
                    .testable()
                    .track_focus(&self.focus)
                    .key_context("Pad")
                    .h(px(40.))
                    .on_click(cx.listener(|this, event: &gpui::ClickEvent, window, cx| {
                        window.focus(&this.focus, cx);
                        this.log.clicks.push(event.click_count());
                        cx.notify();
                    }))
                    .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                        this.log.hovered = *hovered;
                        cx.notify();
                    }))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, _, _, cx| this.push("right-down", cx)),
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| this.push("left-down", cx)),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| this.push("left-up", cx)),
                    )
                    .on_scroll_wheel(cx.listener(|this, event: &gpui::ScrollWheelEvent, _, cx| {
                        let delta = event.delta.pixel_delta(px(10.));
                        this.push(format!("scroll {}", f32::from(delta.y)), cx);
                    }))
                    .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                        this.push(format!("down {}", event.keystroke.key), cx);
                    }))
                    .on_key_up(cx.listener(|this, event: &gpui::KeyUpEvent, _, cx| {
                        this.push(format!("up {}", event.keystroke.key), cx);
                    }))
                    .on_modifiers_changed(cx.listener(
                        |this, event: &gpui::ModifiersChangedEvent, _, cx| {
                            this.push(format!("modifiers shift={}", event.modifiers.shift), cx);
                        },
                    ))
                    .on_action(cx.listener(|this, _: &Bump, _, cx| this.push("bump", cx)))
                    .on_action(cx.listener(|this, _: &Copy, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string("copied".into()));
                        this.push("copy", cx);
                    }))
                    .on_action(cx.listener(|this, _: &Paste, _, cx| {
                        this.log.pasted = cx.read_from_clipboard().and_then(|item| item.text());
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("target")
                    .role(Role::Region)
                    .aria_label("Drop target")
                    .testable()
                    .h(px(40.))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| this.push("dropped", cx)),
                    ),
            )
            .child(
                div()
                    .id("color")
                    .role(Role::ComboBox)
                    .aria_label("Color")
                    .aria_expanded(self.log.open)
                    .aria_value(self.log.color.unwrap_or(""))
                    .testable()
                    .h(px(20.))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.log.open = !this.log.open;
                        cx.notify();
                    })),
            )
            .when(self.log.open, |this| {
                this.child(
                    div()
                        .id("colors")
                        .role(Role::ListBox)
                        .aria_label("Colors")
                        .testable()
                        .children(["Red", "Green"].map(|color| {
                            div()
                                .id(color)
                                .role(Role::ListBoxOption)
                                .aria_label(color)
                                .aria_selected(self.log.color == Some(color))
                                .testable()
                                .h(px(20.))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.log.color = Some(color);
                                    this.log.open = false;
                                    cx.notify();
                                }))
                        })),
                )
            })
            .child(
                div()
                    .id("upload")
                    .role(Role::Button)
                    .aria_label("Upload")
                    .testable()
                    .h(px(20.))
                    .on_click(cx.listener(|_, _, _, cx| {
                        let paths = cx.prompt_for_paths(PathPromptOptions {
                            files: true,
                            directories: false,
                            multiple: true,
                            prompt: None,
                        });
                        cx.spawn(async move |this, cx| {
                            if let Ok(Ok(paths)) = paths.await {
                                this.update(cx, |this, cx| {
                                    this.log.uploaded = paths;
                                    cx.notify();
                                })
                                .ok();
                            }
                        })
                        .detach();
                    })),
            )
            .child(
                div()
                    .id("debounce")
                    .role(Role::Button)
                    .aria_label("Debounce")
                    .testable()
                    .h(px(20.))
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.spawn(async move |this, cx| {
                            cx.background_executor()
                                .timer(Duration::from_millis(100))
                                .await;
                            this.update(cx, |this, cx| {
                                this.log.debounced = true;
                                cx.notify();
                            })
                            .ok();
                        })
                        .detach();
                    })),
            )
            .child(
                flags("email")
                    .required()
                    .invalid()
                    .role(Role::EmailInput)
                    .aria_label("Email")
                    .aria_placeholder("you@example.com")
                    .aria_value("ada@")
                    .testable()
                    .h(px(20.)),
            )
            .child(
                flags("locked")
                    .read_only()
                    .role(Role::TextInput)
                    .aria_label("Locked")
                    .testable()
                    .h(px(20.)),
            )
            .child(
                flags("off")
                    .disabled()
                    .role(Role::Button)
                    .aria_label("Off")
                    .testable()
                    .h(px(20.)),
            )
            .child(
                div()
                    .id("mixed")
                    .role(Role::CheckBox)
                    .aria_label("Some")
                    .aria_toggled(accesskit::Toggled::Mixed)
                    .testable()
                    .h(px(20.)),
            )
            .child(
                div()
                    .id("bold")
                    .role(Role::Button)
                    .aria_label("Bold")
                    .aria_toggled(accesskit::Toggled::True)
                    .accessibility_id("bold-toggle")
                    .testable()
                    .h(px(20.)),
            )
            .child(
                div()
                    .id("heading")
                    .role(Role::Heading)
                    .aria_label("Settings")
                    .aria_level(2)
                    .testable()
                    .h(px(20.)),
            )
            .child(
                div()
                    .id("volume")
                    .role(Role::Slider)
                    .aria_label("Volume")
                    .aria_numeric_value(30.)
                    .aria_min_numeric_value(0.)
                    .aria_max_numeric_value(100.)
                    .testable()
                    .h(px(20.)),
            )
            .child(
                div()
                    .id("logo")
                    .role(Role::Image)
                    .aria_label("Company logo")
                    .testable()
                    .h(px(20.)),
            )
            .child(text!("  Spaced   out  ").testable())
    }
}

fn render_playground(cx: &mut TestAppContext) -> RenderResult<Playground> {
    cx.update(|cx| {
        cx.bind_keys([
            KeyBinding::new("ctrl-b", Bump, Some("Pad")),
            KeyBinding::new("cmd-c", Copy, Some("Pad")),
            KeyBinding::new("ctrl-c", Copy, Some("Pad")),
            KeyBinding::new("cmd-v", Paste, Some("Pad")),
            KeyBinding::new("ctrl-v", Paste, Some("Pad")),
        ])
    });
    render_with(
        cx,
        RenderOptions::new().with_size(size(px(400.), px(800.))),
        |_, cx| Playground {
            log: Log::default(),
            focus: cx.focus_handle(),
        },
    )
}

fn events(screen: &RenderResult<Playground>) -> Vec<String> {
    screen.read(|this, _| this.log.events.clone())
}

fn pad(screen: &Screen) -> gpui_testing_library::Node {
    screen.get_by_role(by::role(Role::Canvas).name("Pad"))
}

#[gpui::test]
fn user_pointer_interactions(cx: &mut TestAppContext) {
    let screen = render_playground(cx);
    let user = screen.user();
    let pad = pad(&screen);

    user.hover(&pad);
    assert!(screen.read(|this, _| this.log.hovered));
    user.unhover(&pad);
    assert!(!screen.read(|this, _| this.log.hovered));

    user.click(&pad);
    user.dbl_click(&pad);
    user.triple_click(&pad);
    assert_eq!(
        screen.read(|this, _| this.log.clicks.clone()),
        [1, 1, 2, 1, 2, 3]
    );

    user.right_click(&pad);
    assert!(events(&screen).contains(&"right-down".into()));

    user.pointer([
        PointerAction::MoveTo(Box::new(pad.clone())),
        PointerAction::Press(MouseButton::Left),
        PointerAction::Release(MouseButton::Left),
    ]);
    assert!(events(&screen).ends_with(&["left-down".into(), "left-up".into()]));

    let target = screen.get_by_label_text("Drop target");
    user.drag_and_drop(&pad, &target);
    assert_eq!(events(&screen).last().map(String::as_str), Some("dropped"));
}

#[gpui::test]
#[should_panic(expected = "move the pointer to a target")]
fn pointer_press_needs_a_position(cx: &mut TestAppContext) {
    let screen = render_playground(cx);
    screen
        .user()
        .pointer([PointerAction::Press(MouseButton::Left)]);
}

#[gpui::test]
fn user_keyboard_shortcuts_and_clipboard(cx: &mut TestAppContext) {
    let screen = render_playground(cx);
    let user = screen.user();
    user.click(&pad(&screen));

    user.keyboard("{Control>}b{/Control}");
    assert!(events(&screen).contains(&"bump".into()));
    user.keyboard("{x>2}{Escape}");
    let log = events(&screen);
    assert_eq!(log.iter().filter(|event| *event == "down x").count(), 2);
    assert!(log.contains(&"down escape".into()));
    user.keyboard("{Shift>}{/Shift}");
    assert!(events(&screen).contains(&"modifiers shift=true".into()));
    user.keyboard("{Shift}");
    user.keyboard("{q>}{/q}");
    assert!(events(&screen).ends_with(&["down q".into(), "up q".into()]));

    assert_eq!(user.copy().as_deref(), Some("copied"));
    assert_eq!(user.cut().as_deref(), Some("copied"));
    user.paste("from the clipboard");
    assert_eq!(
        screen.read(|this, _| this.log.pasted.clone()).as_deref(),
        Some("from the clipboard")
    );

    user.type_with(&pad(&screen), "z", TypeOptions::new().with_skip_click(true));
    assert!(events(&screen).contains(&"down z".into()));
}

#[gpui::test]
fn user_selects_options_and_uploads(cx: &mut TestAppContext) {
    let screen = render_playground(cx);
    let user = screen.user();
    let color = screen.get_by_role(by::role(Role::ComboBox).name("Color"));

    user.select_options(&color, &["Green"]);
    expect(&color).to_have_value("Green");
    user.select_options(&color, &["Red"]);
    expect(&color).to_have_value("Red");
    user.deselect_options(&color, &["Green"]);
    expect(&color).to_have_display_value("Green");

    user.upload(&screen.get_by_label_text("Upload"), ["a.txt", "b.txt"]);
    assert_eq!(
        screen.read(|this, _| this.log.uploaded.clone()),
        Some(vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")])
    );
}

#[gpui::test]
#[should_panic(expected = "no option named \"Blue\"")]
fn select_options_reports_missing_options(cx: &mut TestAppContext) {
    let screen = render_playground(cx);
    let color = screen.get_by_role(Role::ComboBox);
    screen.user().select_options(&color, &["Blue"]);
}

#[gpui::test]
#[should_panic(expected = "did not open a file prompt")]
fn upload_requires_a_prompt(cx: &mut TestAppContext) {
    let screen = render_playground(cx);
    screen.user().upload(&pad(&screen), ["a.txt"]);
}

#[gpui::test]
#[should_panic(expected = "only supported on text fields")]
fn clear_rejects_non_text_fields(cx: &mut TestAppContext) {
    let screen = render_playground(cx);
    screen.user().clear(&pad(&screen));
}

#[gpui::test]
#[should_panic(expected = "disabled or read-only")]
fn clear_rejects_read_only_fields(cx: &mut TestAppContext) {
    let screen = render_playground(cx);
    screen.user().clear(&screen.get_by_label_text("Locked"));
}

#[gpui::test]
fn user_delay_advances_the_test_clock(cx: &mut TestAppContext) {
    let screen = render_playground(cx);
    let debounce = screen.get_by_label_text("Debounce");

    screen.user().click(&debounce);
    assert!(!screen.read(|this, _| this.log.debounced));

    let user = UserEvent::with_options(
        screen.screen().clone(),
        UserEventOptions::new()
            .with_delay(Duration::from_millis(200))
            .with_skip_hover(true),
    );
    assert_eq!(user.screen().window(), screen.window());
    user.click(&debounce);
    assert!(screen.read(|this, _| this.log.debounced));
}

struct Recorder(std::rc::Rc<std::cell::RefCell<String>>);

impl gpui::InputHandler for Recorder {
    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut App,
    ) -> Option<UTF16Selection> {
        None
    }
    fn marked_text_range(&mut self, _: &mut Window, _: &mut App) -> Option<Range<usize>> {
        None
    }
    fn text_for_range(
        &mut self,
        _: Range<usize>,
        _: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut App,
    ) -> Option<String> {
        None
    }
    fn replace_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        _: &mut App,
    ) {
        self.0.borrow_mut().push_str(text);
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        _: &str,
        _: Option<Range<usize>>,
        _: &mut Window,
        _: &mut App,
    ) {
    }
    fn unmark_text(&mut self, _: &mut Window, _: &mut App) {}
    fn bounds_for_range(
        &mut self,
        _: Range<usize>,
        _: &mut Window,
        _: &mut App,
    ) -> Option<Bounds<Pixels>> {
        None
    }
    fn character_index_for_point(
        &mut self,
        _: gpui::Point<Pixels>,
        _: &mut Window,
        _: &mut App,
    ) -> Option<usize> {
        None
    }
}

#[gpui::test]
fn fire_event_single_events(cx: &mut TestAppContext) {
    let screen = render_playground(cx);
    let fire = screen.fire_event();
    let pad = pad(&screen);

    fire.mouse_enter(&pad);
    assert!(screen.read(|this, _| this.log.hovered));
    fire.mouse_leave();
    assert!(!screen.read(|this, _| this.log.hovered));

    fire.double_click(&pad);
    assert_eq!(screen.read(|this, _| this.log.clicks.clone()), [1, 2]);
    fire.context_menu(&pad);
    assert!(events(&screen).contains(&"right-down".into()));

    fire.scroll(&pad, ScrollDelta::Pixels(gpui::point(px(0.), px(-5.))));
    assert!(events(&screen).contains(&"scroll -5".into()));

    fire.key_down("k");
    fire.key_up("k");
    fire.key_press("j");
    let log = events(&screen);
    assert!(
        log.contains(&"down k".into()) && log.contains(&"up k".into()),
        "{log:?}"
    );
    assert!(log.ends_with(&["down j".into(), "up j".into()]));

    fire.modifiers_changed(Modifiers::shift());
    assert!(events(&screen).contains(&"modifiers shift=true".into()));
    fire.modifiers_changed(Modifiers::default());

    fire.action(&Bump);
    assert_eq!(events(&screen).last().map(String::as_str), Some("bump"));

    fire.dispatch(PlatformInput::KeyDown(gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse("m").unwrap(),
        is_held: false,
        prefer_character_input: false,
    }));
    assert_eq!(events(&screen).last().map(String::as_str), Some("down m"));

    let text = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
    fire.input(Recorder(text.clone()), "こんにちは");
    assert_eq!(*text.borrow(), "こんにちは");
}

#[gpui::test]
#[should_panic(expected = "is not visible")]
fn interacting_with_hidden_elements_panics(cx: &mut TestAppContext) {
    let screen = render_with(
        cx,
        RenderOptions::new().with_size(size(px(100.), px(100.))),
        |_, cx| Playground {
            log: Log::default(),
            focus: cx.focus_handle(),
        },
    );
    // Off-screen below the 100px window.
    let logo = screen.get_by_role(by::role(Role::Image).hidden(true));
    screen.fire_event().click(&logo);
}

#[gpui::test]
fn matchers(cx: &mut TestAppContext) {
    let screen = render_playground(cx);

    let email = screen.get_by_role(Role::EmailInput);
    expect(&email).to_be_required();
    expect(&email).to_be_invalid();
    expect(&email).not().to_be_valid();
    expect(&email).to_be_editable();
    expect(&email).to_be_enabled();
    expect(&email).to_have_placeholder("you@example.com");
    expect(&email).to_have_accessible_name(TextMatch::contains("mai"));
    expect(&email).to_have_display_value("ada@");
    expect(&email).not().to_have_text_content("never");

    let locked = screen.get_by_label_text("Locked");
    expect(&locked).not().to_be_editable();
    expect(&locked).to_be_valid();

    let off = screen.get_by_role(by::role(Role::Button).disabled(true));
    expect(&off).to_be_disabled();
    expect(&off).not().to_be_enabled();
    assert_eq!(off.name().as_deref(), Some("Off"));

    let mixed = screen.get_by_role(Role::CheckBox);
    expect(&mixed).to_be_partially_checked();
    expect(&mixed).not().to_be_checked();
    assert!(
        screen
            .query_by_role(by::role(Role::CheckBox).checked(false))
            .is_none()
    );

    let bold = screen.get_by_role(by::role(Role::Button).pressed(true));
    expect(&bold).to_be_pressed();
    expect(&bold).to_have_test_id("bold-toggle");
    assert_eq!(bold.test_id().as_deref(), Some("bold-toggle"));
    assert!(
        screen
            .query_by_role(by::role(Role::Button).name("Bold").pressed(false))
            .is_none()
    );

    expect(screen.get_by_role(by::role(Role::Heading).level(2))).to_have_text_content("Settings");
    let volume = screen.get_by_role(
        by::role(Role::Slider)
            .value_now(30.)
            .value_min(0.)
            .value_max(100.),
    );
    expect(&volume).to_have_value("30");
    assert!(
        screen
            .query_by_role(by::role(Role::Slider).value_text("30"))
            .is_none()
    );
    expect(screen.get_by_alt_text("Company logo")).to_have_role(Role::Image);
    expect(screen.get_by_display_value("ada@")).to_have_role(Role::EmailInput);

    // Text is normalized before matching unless asked otherwise.
    let spaced = screen.get_by_text("Spaced out");
    assert_eq!(spaced.text().as_deref(), Some("  Spaced   out  "));
    assert!(
        screen
            .query_by_text(TextMatch::exact("Spaced out").without_normalization())
            .is_none()
    );
    assert_eq!(
        screen.get_by_text(TextMatch::function(|text, snapshot| {
            text.ends_with("out") && snapshot.role() == Some(Role::Label)
        })),
        spaced
    );
}

#[gpui::test]
fn selection_states_and_containment(cx: &mut TestAppContext) {
    let screen = render_playground(cx);
    let user = screen.user();
    let color = screen.get_by_role(by::role(Role::ComboBox).name("Color"));

    user.click(&color);
    let listbox = screen.get_by_role(Role::ListBox);
    expect(&listbox).not().to_be_empty_dom_element();
    let options = within(&listbox).get_all_by_role(Role::ListBoxOption);
    assert_eq!(options.len(), 2);
    expect(&options[0]).not().to_be_selected();
    expect(&listbox).to_contain_element(&options[1]);
    expect(&options[1]).not().to_contain_element(&listbox);
    assert!(listbox.contains(&options[0]));

    user.click(&options[0]);
    expect(&listbox).not().to_be_in_the_document();
    expect(screen.query_by_role(Role::ListBox))
        .not()
        .to_be_in_the_document();
    user.click(&color);
    expect(screen.get_by_role(by::role(Role::ListBoxOption).selected(true)))
        .to_have_accessible_name("Red");
    let scoped = within(&screen.get_by_role(Role::ListBox));
    assert!(scoped.pretty().contains("ListBoxOption \"Red\""));
    scoped.debug();
}

#[gpui::test]
async fn find_all_and_wait_for(cx: &mut TestAppContext) {
    let screen = render_playground(cx);
    assert_eq!(screen.find_all_by_role(Role::Button).await.len(), 4);
    assert_eq!(
        screen.find_by_alt_text("Company logo").await.role(),
        Some(Role::Image)
    );

    screen.user().click(&screen.get_by_label_text("Debounce"));
    let debounced = wait_for(
        &screen,
        WaitOptions::new().with_interval(Duration::from_millis(10)),
        || screen.read(|this, _| this.log.debounced).then_some("done"),
    )
    .await;
    assert_eq!(debounced, "done");
    screen
        .wait_for(|| screen.query_by_label_text("Pad").ok_or("not yet"))
        .await;
}

#[gpui::test]
#[should_panic(expected = "wait_for timed out: \"still loading\"")]
async fn wait_for_reports_the_last_error(cx: &mut TestAppContext) {
    let screen = render_playground(cx);
    wait_for(
        &screen,
        WaitOptions::new().with_timeout(Duration::from_millis(30)),
        || Err::<(), _>("still loading"),
    )
    .await;
}

#[gpui::test]
#[should_panic(expected = "Unable to find an element with the text \"Never\"")]
async fn find_times_out(cx: &mut TestAppContext) {
    let screen = render_playground(cx);
    configure(|config| config.with_async_util_timeout(Duration::from_millis(20)));
    screen.find_by_text("Never").await;
}

#[gpui::test]
#[should_panic(expected = "Found multiple elements")]
async fn find_rejects_ambiguous_matches(cx: &mut TestAppContext) {
    let screen = render_playground(cx);
    screen.find_by_role(Role::Button).await;
}

#[gpui::test]
fn configure_default_hidden(cx: &mut TestAppContext) {
    let screen = render_with(
        cx,
        RenderOptions::new().with_size(size(px(100.), px(100.))),
        |_, cx| Playground {
            log: Log::default(),
            focus: cx.focus_handle(),
        },
    );
    assert!(screen.query_by_role(Role::Image).is_none());
    configure(|config| config.with_default_hidden(true));
    assert!(get_config().default_hidden());
    assert!(screen.query_by_role(Role::Image).is_some());
    configure(|config| config.with_default_hidden(false));
}

#[gpui::test]
fn act_rerender_cleanup_and_debug_output(cx: &mut TestAppContext) {
    let screen = render_playground(cx);

    let value = act(cx, |_| 7);
    assert_eq!(value, 7);
    screen.act(|_, cx| cx.refresh_windows());
    screen.rerender(|this, _, cx| {
        this.log.color = Some("Green");
        cx.notify();
    });
    expect(screen.get_by_role(Role::ComboBox)).to_have_value("Green");

    screen.debug();
    screen.log_roles();
    assert!(screen.pretty().contains("[value=\"Green\", collapsed]"));

    let node = screen.get_by_role(Role::Slider);
    assert_eq!(node.description(), None);
    assert_eq!(node.value(), None);
    assert!(node.bounds().size.height > px(0.));
    assert_eq!(node.element_id(), &ElementId::from("volume"));
    assert_eq!(node.screen().window(), screen.window());
    assert!(format!("{node:?}").contains("value-now=30"));

    cleanup(cx);
    assert!(cx.windows().is_empty());
    assert!(!node.is_connected());
    assert!(!node.is_visible());
    assert_eq!(screen.cx().windows().len(), 0);
}

#[gpui::test]
#[should_panic(expected = "received None")]
fn matchers_reject_missing_elements(cx: &mut TestAppContext) {
    let screen = render_playground(cx);
    expect(screen.query_by_text("Nothing")).to_be_checked();
}

#[gpui::test]
#[should_panic(expected = "expect(element).not().to_be_visible() failed")]
fn negated_matchers_report_failure(cx: &mut TestAppContext) {
    let screen = render_playground(cx);
    expect(pad(&screen)).not().to_be_visible();
}

#[gpui::test]
#[should_panic(expected = "no focus handle was observed")]
fn to_have_focus_requires_a_tracked_handle(cx: &mut TestAppContext) {
    let screen = render_playground(cx);
    expect(screen.get_by_role(Role::Slider)).to_have_focus();
}
