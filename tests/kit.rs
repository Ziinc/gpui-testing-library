//! gpui-kit components through gpui-kit's own window entry point and its
//! `.test_support()` observations, without `.testable()`.
use gpui_kit::{
    AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, Role, Styled as _,
    TestAppContext, Window,
    component::{
        button::Button,
        checkbox::Checkbox,
        input::{Input, InputState},
        v_flex,
    },
    test::TestWindowExt as _,
};
use gpui_testing_library::{Queries, Screen, by, expect, kit};

struct Signup {
    name: Entity<InputState>,
    agreed: bool,
    saved: Option<String>,
}

impl Render for Signup {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_2()
            .child(Input::new(&self.name))
            .child(
                Checkbox::new("agree")
                    .label("I agree")
                    .checked(self.agreed)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.agreed = *checked;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("save")
                    .label("Save")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.saved = Some(this.name.read(cx).value().to_string());
                        cx.notify();
                    })),
            )
    }
}

fn render_signup(cx: &mut TestAppContext) -> gpui_testing_library::RenderResult<Signup> {
    cx.update(gpui_kit::init);
    kit::render(cx, |window, cx| {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Your name"));
        cx.new(|_| Signup {
            name,
            agreed: false,
            saved: None,
        })
    })
}

#[gpui_kit::test]
fn kit_components_are_queryable_by_role(cx: &mut TestAppContext) {
    let screen = render_signup(cx);

    let save = screen.get_by_role(by::role(Role::Button).name("Save"));
    expect(&save).to_be_visible();
    expect(&save).to_be_enabled();
    let agree = screen.get_by_role(by::role(Role::CheckBox).name("I agree"));
    expect(&agree).not().to_be_checked();
}

#[gpui_kit::test]
fn user_event_drives_kit_controls(cx: &mut TestAppContext) {
    let screen = render_signup(cx);
    let user = screen.user();

    let agree = screen.get_by_role(by::role(Role::CheckBox).name("I agree"));
    user.click(&agree);
    expect(&agree).to_be_checked();
    assert!(screen.read(|signup, _| signup.agreed));

    // Kit names an unlabeled input after its placeholder.
    let name = screen.get_by_role(by::role(Role::TextInput).name("Your name"));
    user.type_text(&name, "Ada");
    expect(&name).to_have_display_value("Ada");
    user.clear(&name);
    expect(&name).not().to_have_display_value("Ada");
    user.type_text(&name, "Grace");

    user.click(&screen.get_by_role(by::role(Role::Button).name("Save")));
    assert_eq!(
        screen.read(|signup, _| signup.saved.clone()),
        Some("Grace".into())
    );
}

#[gpui_kit::test]
fn screen_and_kit_harness_share_one_window(cx: &mut TestAppContext) {
    let result = render_signup(cx);
    // Wrap the same window again, as a test that opened it itself would.
    let screen = Screen::new(cx, result.window());

    // gpui-kit's harness clicks by ElementId; Testing Library sees the result.
    cx.update_window(screen.window(), |_, window, cx| window.click("agree", cx))
        .unwrap();
    expect(screen.get_by_role(by::role(Role::CheckBox).name("I agree"))).to_be_checked();
    assert_eq!(
        screen.get_by_element_id("agree"),
        screen.get_by_role(Role::CheckBox)
    );
}
