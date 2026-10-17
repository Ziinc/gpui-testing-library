//! # GPUI Testing Library
//!
//! [Testing Library](https://testing-library.com) for [GPUI](https://gpui.rs):
//! render a view in GPUI's headless test platform, find elements the way a
//! user perceives them (role, name, text, value), interact through real
//! pointer and keyboard dispatch, and assert with jest-dom style matchers.
//!
//! ```no_run
//! use gpui::{prelude::*, *};
//! use gpui_testing_library::prelude::*;
//!
//! struct Counter { count: usize }
//!
//! impl Render for Counter {
//!     fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
//!         div()
//!             .child(text!(format!("Count: {}", self.count)).testable())
//!             .child(
//!                 div()
//!                     .id("increment")
//!                     .role(Role::Button)
//!                     .aria_label("Increment")
//!                     .testable()
//!                     .on_click(cx.listener(|this, _, _, cx| {
//!                         this.count += 1;
//!                         cx.notify();
//!                     })),
//!             )
//!     }
//! }
//!
//! // Annotate with #[gpui::test] (or #[gpui_kit::test]).
//! fn increments(cx: &mut TestAppContext) {
//!     let screen = render(cx, |_, _| Counter { count: 0 });
//!     let user = screen.user();
//!
//!     user.click(&screen.get_by_role(by::role(Role::Button).name("Increment")));
//!
//!     expect(screen.get_by_text("Count: 1")).to_be_visible();
//! }
//! ```
//!
//! ## Observing elements
//!
//! GPUI's test platform does not activate AccessKit, so the accessibility tree
//! is not available in tests. Mark the elements tests should see with
//! [`TestableExt::testable`]. The wrapper records the element's accessibility
//! facts (role, `aria_label`, `aria_value`, toggled, selected...) each frame
//! with no layout node or behavior change, so it can stay in production code.
//! With the `gpui-kit` feature, everything gpui-kit marks `.test_support()` is
//! queryable too (see [`kit`]).
//!
//! ## Testing Library mapping
//!
//! | Testing Library | Here |
//! |---|---|
//! | `render(ui)` | [`render`], [`render_with`], [`kit::render`] |
//! | `screen` | [`Screen`], returned by `render` or [`Screen::new`] |
//! | `getBy*`/`queryBy*`/`findBy*` (+`All`) | [`Queries`], e.g. `get_by_text`, `query_all_by_role`, `find_by_label_text` |
//! | `ByRole`, `ByLabelText`, `ByPlaceholderText`, `ByText`, `ByDisplayValue`, `ByAltText`, `ByTitle`, `ByTestId` | [`by`] |
//! | `TextMatch`, `exact: false`, normalizer | [`TextMatch`] |
//! | `within(el)` | [`within`] |
//! | `fireEvent` | [`Screen::fire_event`] / [`FireEvent`] |
//! | `userEvent.setup()` | [`Screen::user`] / [`UserEvent`] |
//! | `waitFor`, `waitForElementToBeRemoved` | [`Screen::wait_for`], [`wait_for`], [`wait_for_element_to_be_removed`] |
//! | `act`, `cleanup`, `unmount`, `rerender` | [`act`], [`cleanup`], [`Screen::unmount`], [`RenderResult::rerender`] |
//! | `screen.debug`, `prettyDOM`, `logRoles` | [`Screen::debug`], [`Screen::pretty`], [`Screen::log_roles`] |
//! | `configure` | [`configure`] |
//! | jest-dom matchers | [`expect()`] |
//!
//! See the README for gaps where GPUI has no equivalent.

mod config;
mod events;
mod expect;
mod fire_event;
mod keyboard;
mod node;
mod observe;
mod pretty;
mod query;
mod screen;
mod text_match;
mod user_event;
mod wait;

#[cfg(feature = "gpui-kit")]
pub mod kit;

pub use config::{Config, configure, get_config};
pub use expect::{Expect, Subject, expect};
pub use fire_event::FireEvent;
pub use node::{Node, Snapshot, Source};
pub use observe::{ObservableText, Probe, Testable, TestableExt, TestableTextExt};
pub use query::{
    ByAltText, ByDisplayValue, ByElementId, ByLabelText, ByPlaceholderText, ByRole, ByTestId,
    ByText, ByTitle, Queries, Query, by,
};
pub use screen::{
    RenderOptions, RenderResult, Screen, Within, act, cleanup, observed, observed_matching, render,
    render_with, within,
};
pub use text_match::{TextMatch, normalize};
pub use user_event::{PointerAction, TypeOptions, UserEvent, UserEventOptions};
pub use wait::{Ready, WaitOptions, wait_for, wait_for_element_to_be_removed};

/// Everything a test needs: `use gpui_testing_library::prelude::*;`
pub mod prelude {
    pub use crate::{
        Queries, TestableExt, TestableTextExt, TextMatch, by, expect, render, render_with, within,
    };
}
