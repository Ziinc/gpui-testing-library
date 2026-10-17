//! Views written against tauri-gpui's GPUI re-export are the views
//! gpui-testing-library renders: one GPUI crate, no Tauri runtime needed.
//!
//! tauri-gpui shows a view with `attach_gpui_view`, inside gpui-base's Root;
//! `kit::render` opens it inside the same Root through `gpui_kit::open_window`.
use gpui_kit::component::{button::Button, checkbox::Checkbox, v_flex};
use gpui_testing_library::{Queries as _, TestableExt, TestableTextExt, by, expect, kit, render};
use tauri_plugin_gpui::gpui::{
    AppContext as _, Context, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    Role, StatefulInteractiveElement as _, Styled as _, TestAppContext, Window, div, px, text,
};

#[derive(Default)]
struct Status {
    online: bool,
}

impl Render for Status {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(text!(id = "state", if self.online { "Online" } else { "Offline" }).testable())
            .child(
                div()
                    .id("toggle")
                    .role(Role::Switch)
                    .aria_label("Connected")
                    .aria_toggled(self.online.into())
                    .testable()
                    .h(px(30.))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.online = !this.online;
                        cx.notify();
                    })),
            )
    }
}

#[tauri_plugin_gpui::gpui::test]
fn plain_views_render_without_tauri(cx: &mut TestAppContext) {
    let screen = render(cx, |_, _| Status::default());
    let toggle = screen.get_by_role(by::role(Role::Switch).name("Connected"));
    screen.user().click(&toggle);
    expect(&toggle).to_be_checked();
    expect(screen.get_by_text("Online")).to_be_visible();
}

struct Settings {
    sync: bool,
    saved: bool,
}

impl Render for Settings {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                Checkbox::new("sync")
                    .label("Sync")
                    .checked(self.sync)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.sync = *checked;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("save")
                    .label("Save")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.saved = true;
                        cx.notify();
                    })),
            )
    }
}

#[tauri_plugin_gpui::gpui::test]
fn kit_views_render_inside_root(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let screen = kit::render(cx, |_, cx| {
        cx.new(|_| Settings {
            sync: false,
            saved: false,
        })
    });
    let sync = screen.get_by_role(by::role(Role::CheckBox).name("Sync"));
    screen.user().click(&sync);
    expect(&sync).to_be_checked();
    screen
        .user()
        .click(&screen.get_by_role(by::role(Role::Button).name("Save")));
    assert!(screen.read(|settings, _| settings.saved));
}
