# gpui-testing-library

[Testing Library](https://testing-library.com) for [GPUI](https://gpui.rs).
Render a view in GPUI's headless test platform, find elements the way a user
perceives them, interact through GPUI's real pointer and keyboard dispatch,
and assert with jest-dom style matchers.

> The more your tests resemble the way your software is used, the more
> confidence they can give you.

```rust
use gpui::{prelude::*, *};
use gpui_testing_library::prelude::*;

#[gpui::test]
async fn saves_the_name(cx: &mut TestAppContext) {
    let screen = render(cx, |window, cx| SignupForm::new(window, cx));
    let user = screen.user();

    user.type_text(&screen.get_by_role(by::role(Role::TextInput).name("Name")), "Ada");
    user.click(&screen.get_by_role(by::role(Role::Button).name("Save")));

    expect(screen.find_by_text("Saved Ada").await).to_be_visible();
}
```

## Install

```toml
[dev-dependencies]
gpui-testing-library = { git = "https://github.com/Ziinc/gpui-testing-library" }
# For gpui-kit applications:
# gpui-testing-library = { git = "...", features = ["gpui-kit"] }
```

It depends on `gpui-pre = "=0.3.9"` with `test-support`, the snapshot
`gpui-kit 0.7.3` uses. Your application must use the same GPUI crate, since
windows and elements are shared types.

On Linux, GPUI needs the packages in gpui-kit's `script/install-linux.sh` (see `.github/workflows/ci.yml`).

## Making elements visible to tests

GPUI's test platform never activates AccessKit, so the accessibility tree a
screen reader sees is not available in a `TestAppContext`. Mark the elements
your tests should find with `.testable()`:

```rust
div()
    .id("save")
    .role(Role::Button)
    .aria_label("Save")
    .testable() // before .track_focus, so focus is observed
    .track_focus(&self.focus)
    .on_click(...)

text!(format!("Saved {}", self.name)).testable()
```

`.testable()` reads the same facts AccessKit would publish (role,
`aria_label`, `aria_description`, `aria_value`, `aria_placeholder`,
`accessibility_id`, toggled, selected, expanded, level, numeric value), plus
painted bounds, visibility and focus, every frame. It adds no layout node and
changes no behavior, so it can stay in production code.

### gpui-kit

With the `gpui-kit` feature, everything gpui-kit marks with `.test_support()`
is queryable too, which includes Kit's buttons, checkboxes, switches, inputs,
selects, menus, tabs, dialogs and tables. No `.testable()` is needed for them.

```rust
#[gpui_kit::test]
fn checks_the_box(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let screen = gpui_testing_library::kit::render(cx, |window, cx| {
        cx.new(|cx| Signup::new(window, cx))
    });
    let agree = screen.get_by_role(by::role(Role::CheckBox).name("I agree"));
    screen.user().click(&agree);
    expect(&agree).to_be_checked();
}
```

`kit::render` opens the window through `gpui_kit::open_window`, inside Kit's
`Root`, exactly as an application does. A window you opened yourself works
with `Screen::new(cx, window)`, and gpui-kit's own `TestWindowExt` helpers
(`window.click("agree", cx)`) keep working on the same window.

## API

| Testing Library | gpui-testing-library |
|---|---|
| `render(ui)` | `render(cx, \|window, cx\| View)`, `render_with(cx, RenderOptions, ..)`, `kit::render` |
| `screen` | `Screen` (returned by `render`, or `Screen::new(cx, window)`) |
| `getBy*` `queryBy*` `findBy*` `getAllBy*` `queryAllBy*` `findAllBy*` | `get_by_role`, `query_all_by_text`, `find_by_label_text`, ... on the `Queries` trait; or `get(by::role(..))` |
| `ByRole` (`name`, `description`, `checked`, `pressed`, `selected`, `expanded`, `level`, `value`, `hidden`) | `by::role(Role::Button).name("Save").checked(true)` |
| `ByLabelText`, `ByPlaceholderText`, `ByText`, `ByDisplayValue`, `ByAltText`, `ByTitle`, `ByTestId` | `by::label_text`, `by::placeholder_text`, `by::text`, `by::display_value`, `by::alt_text`, `by::title`, `by::test_id` |
| `TextMatch` (string, regex, function), `exact: false`, normalizer | `TextMatch::exact`, `::regex`, `::function`, `::contains`, `.without_normalization()` |
| `within(el)` | `within(&node)` |
| `fireEvent.*` | `screen.fire_event()`: `click`, `mouse_down`, `mouse_up`, `mouse_move`, `mouse_enter`, `mouse_leave`, `double_click`, `context_menu`, `scroll`, `key_down`, `key_up`, `key_press`, `modifiers_changed`, `input`, `action`, `dispatch` |
| `userEvent.setup()` | `screen.user()`, `UserEvent::with_options(screen, UserEventOptions)` |
| `click`, `dblClick`, `tripleClick`, `hover`, `unhover`, `pointer` | `user.click`, `dbl_click`, `triple_click`, `right_click`, `hover`, `unhover`, `pointer([PointerAction::..])`, `drag_and_drop` |
| `type`, `keyboard`, `clear`, `tab` | `user.type_text`, `type_with`, `keyboard("{Shift>}a{/Shift}{Enter}")`, `clear`, `tab`, `tab_backward` |
| `copy`, `cut`, `paste` | `user.copy()`, `cut()`, `paste("text")` (GPUI test clipboard) |
| `selectOptions`, `deselectOptions` | `user.select_options(&trigger, &["Red"])`, `deselect_options` |
| `upload` | `user.upload(&button, ["a.txt"])` (answers GPUI's "Open" prompt) |
| `waitFor`, `waitForElementToBeRemoved` | `screen.wait_for(\|\| ..).await`, `wait_for(&screen, WaitOptions, ..)`, `wait_for_element_to_be_removed` |
| `act`, `cleanup`, `unmount`, `rerender` | `act(cx, ..)`, `screen.act(..)`, `cleanup(cx)`, `screen.unmount()`, `result.rerender(\|view, window, cx\| ..)` |
| `screen.debug()`, `prettyDOM`, `logRoles` | `screen.debug()`, `screen.pretty()`, `screen.log_roles()` |
| `configure` | `configure(\|c\| c.with_async_util_timeout(..))` |
| jest-dom | `expect(&node)`: `to_be_in_the_document`, `to_be_visible`, `to_be_disabled`, `to_be_enabled`, `to_be_checked`, `to_be_partially_checked`, `to_be_pressed`, `to_be_selected`, `to_be_expanded`, `to_be_collapsed`, `to_have_focus`, `to_be_required`, `to_be_invalid`, `to_be_valid`, `to_be_editable`, `to_be_empty_dom_element`, `to_contain_element`, `to_have_role`, `to_have_accessible_name`, `to_have_accessible_description`, `to_have_text_content`, `to_have_value`, `to_have_display_value`, `to_have_placeholder`, `to_have_test_id`, and `.not()` |

Nodes are live handles, like DOM elements: after `user.click(&checkbox)`,
`expect(&checkbox).to_be_checked()` reads the new state without querying
again. Queries return elements in reading order (top to bottom, left to
right, ancestors first). `find_*` and `wait_for` poll on GPUI's deterministic
test clock, so a 300 ms timer in the app resolves instantly in the test.

## Differences from Testing Library

These follow from GPUI, not from missing work:

- **Opt-in observation.** Only `.testable()` elements (and Kit's
  `.test_support()` elements) are visible to queries, because the test
  platform has no AccessKit tree. Plain string children (`div().child("Hi")`)
  carry no accessibility node; use `text!(..).testable()` or an `aria_label`.
- **Accessible names are explicit.** GPUI does not compute a name from
  children or `aria-labelledby`; the name is `aria_label`. `ByLabelText`
  matches it, not a separate `<label>` element.
- **`ByTitle`** matches `aria_description`, since GPUI has no `title`
  attribute. **`ByAltText`** matches the name of `Role::Image` elements.
  **`ByTestId`** matches `.accessibility_id(..)`, GPUI's platform automation
  id (UIA `AutomationId`, `AXIdentifier`).
- **Disabled, required, read-only and invalid** come from AccessKit flags.
  GPUI's `div` has no setter for them, so custom elements or Kit components
  must publish them; plain divs always read as enabled.
- **Visibility** is "painted with a non-empty, unclipped area, and no
  ancestor hid it". `.invisible()` and zero opacity on the element itself are
  detected for interactive elements; for `text!` only ancestors and clipping.
- **Focus** requires `.testable()` before `.track_focus(&handle)`. There is no
  `fireEvent.focus(el)`; GPUI focus moves through handles, clicks or Tab.
  `user.tab()` uses GPUI's tab stops, so create handles with `cx.focus_handle().tab_stop(true)`.
- **gpui-kit snapshots** expose role, name, value, checked, selected,
  expanded, disabled, bounds, visibility and focus, but not description,
  placeholder or test id. Kit names an unlabeled input after its placeholder,
  so query it by role and name.
- **`fireEvent.change` / `input`** has no DOM value to set. `user.type_text`
  types through GPUI's key dispatch and input handler; `fire_event().input`
  takes an `InputHandler` for IME-style commits.
- **`renderHook`** has no equivalent: GPUI state lives in entities, which a
  test can create directly with `cx.new(..)`.
- **Interactions are synchronous.** `user` methods do not need `.await`; GPUI
  dispatch is synchronous. Set `UserEventOptions::with_delay` to advance the
  test clock between steps.
- `TextMatch::contains` ignores case, like `{ exact: false }`.

## Development

```sh
cargo test --all-features
cargo clippy --all-features --all-targets -- --deny warnings
cargo fmt --check
```
