//! Testing Library's `user-event`: interactions as a user performs them.
//!
//! Each method dispatches the complete native sequence (pointer moves,
//! presses, key-down, text input, key-up) through GPUI's real hit testing and
//! key dispatch, completing a frame after every step, then lets spawned work
//! finish.
use crate::{
    events,
    fire_event::center,
    keyboard::{self, KeyAction, Modifier},
    node::{Node, is_text_field},
    query::{Queries, by},
    screen::Screen,
};
use gpui::{ClipboardItem, Modifiers, MouseButton, Pixels, Point, Role, px};
use std::{cell::RefCell, path::PathBuf, time::Duration};

/// Options for [`UserEvent::with_options`], like `userEvent.setup(options)`.
#[derive(Clone, Copy, Debug, Default)]
pub struct UserEventOptions {
    delay: Option<Duration>,
    skip_hover: bool,
}

impl UserEventOptions {
    pub fn new() -> Self {
        Self::default()
    }
    /// Advances GPUI's test clock by `delay` between steps, so timers and
    /// debounced handlers run as they would for a real user. Default none.
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }
    /// Clicks without first moving the pointer onto the element.
    pub fn with_skip_hover(mut self, skip_hover: bool) -> Self {
        self.skip_hover = skip_hover;
        self
    }
}

/// One step of [`UserEvent::pointer`].
#[derive(Clone, Debug)]
pub enum PointerAction {
    /// Moves to the center of an element.
    MoveTo(Box<Node>),
    /// Moves to a window-local position.
    MoveToPosition(Point<Pixels>),
    Press(MouseButton),
    Release(MouseButton),
}

/// Simulated user. Created by [`Screen::user`]; keeps held keys between calls.
pub struct UserEvent {
    screen: Screen,
    options: UserEventOptions,
    state: RefCell<KeyboardState>,
}

#[derive(Default)]
struct KeyboardState {
    modifiers: Modifiers,
    held_keys: Vec<String>,
    pointer: Option<Point<Pixels>>,
    pressed: Option<MouseButton>,
}

impl UserEvent {
    pub(crate) fn new(screen: Screen) -> Self {
        Self::with_options(screen, UserEventOptions::default())
    }

    /// `userEvent.setup(options)`.
    pub fn with_options(screen: Screen, options: UserEventOptions) -> Self {
        // Start from the window's modifiers, so keys another `UserEvent` left
        // held keep applying to typed characters as they do to clicks.
        let modifiers = screen
            .try_with_window(|window, _| window.modifiers())
            .unwrap_or_default();
        Self {
            screen,
            options,
            state: RefCell::new(KeyboardState {
                modifiers,
                ..KeyboardState::default()
            }),
        }
    }

    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    fn settle(&self) {
        if let Some(delay) = self.options.delay {
            self.screen.executor().advance_clock(delay);
        }
        self.screen.run_until_parked();
        self.screen.redraw();
    }

    #[track_caller]
    fn click_count(&self, node: &Node, button: MouseButton, count: usize) {
        let position = center(node);
        self.screen.with_window(|window, cx| {
            if !self.options.skip_hover {
                events::mouse_move(window, position, None, cx);
            }
            for click_count in 1..=count {
                events::mouse_down(window, position, button, click_count, cx);
                events::mouse_up(window, position, button, click_count, cx);
            }
        });
        self.state.borrow_mut().pointer = Some(position);
        self.settle();
    }

    /// Hovers, then clicks the center of `node` with the left button.
    #[track_caller]
    pub fn click(&self, node: &Node) {
        self.click_count(node, MouseButton::Left, 1);
    }
    #[track_caller]
    pub fn dbl_click(&self, node: &Node) {
        self.click_count(node, MouseButton::Left, 2);
    }
    #[track_caller]
    pub fn triple_click(&self, node: &Node) {
        self.click_count(node, MouseButton::Left, 3);
    }
    /// Clicks with the right button, opening context menus.
    #[track_caller]
    pub fn right_click(&self, node: &Node) {
        self.click_count(node, MouseButton::Right, 1);
    }

    /// Moves the pointer onto `node`.
    #[track_caller]
    pub fn hover(&self, node: &Node) {
        let position = center(node);
        self.move_pointer(position);
    }

    /// Moves the pointer off `node`, outside the window.
    pub fn unhover(&self, _node: &Node) {
        self.move_pointer(Point::new(px(-1.), px(-1.)));
    }

    fn move_pointer(&self, position: Point<Pixels>) {
        let pressed = self.state.borrow().pressed;
        self.screen
            .with_window(|window, cx| events::mouse_move(window, position, pressed, cx));
        self.state.borrow_mut().pointer = Some(position);
        self.settle();
    }

    /// Runs pointer steps in order, like user-event's `pointer([...])`.
    #[track_caller]
    pub fn pointer(&self, actions: impl IntoIterator<Item = PointerAction>) {
        for action in actions {
            match action {
                PointerAction::MoveTo(node) => self.move_pointer(center(&node)),
                PointerAction::MoveToPosition(position) => self.move_pointer(position),
                PointerAction::Press(button) => {
                    let position = self.pointer_position();
                    self.screen.with_window(|window, cx| {
                        events::mouse_down(window, position, button, 1, cx)
                    });
                    self.state.borrow_mut().pressed = Some(button);
                    self.settle();
                }
                PointerAction::Release(button) => {
                    let position = self.pointer_position();
                    self.screen.with_window(|window, cx| {
                        events::mouse_up(window, position, button, 1, cx)
                    });
                    self.state.borrow_mut().pressed = None;
                    self.settle();
                }
            }
        }
    }

    #[track_caller]
    fn pointer_position(&self) -> Point<Pixels> {
        self.state
            .borrow()
            .pointer
            .expect("move the pointer to a target before pressing a button")
    }

    /// Presses on `from`, moves to `to` in steps, and releases there.
    #[track_caller]
    pub fn drag_and_drop(&self, from: &Node, to: &Node) {
        let start = center(from);
        let end = center(to);
        self.pointer([
            PointerAction::MoveToPosition(start),
            PointerAction::Press(MouseButton::Left),
        ]);
        for step in 1..=8 {
            let fraction = step as f32 / 8.;
            self.pointer([PointerAction::MoveToPosition(Point::new(
                start.x + (end.x - start.x) * fraction,
                start.y + (end.y - start.y) * fraction,
            ))]);
        }
        self.pointer([PointerAction::Release(MouseButton::Left)]);
    }

    /// Types with user-event's keyboard syntax at the current focus, e.g.
    /// `"hello{Enter}"`, `"{Shift>}{Tab}{/Shift}"`, `"{Control>}a{/Control}"`.
    pub fn keyboard(&self, text: &str) {
        for action in keyboard::parse(text) {
            match action {
                KeyAction::Type(character) => {
                    let modifiers = self.state.borrow().modifiers;
                    let keystroke = events::char_keystroke(character, modifiers);
                    self.screen.with_window(|window, cx| {
                        events::press(window, keystroke, cx);
                    });
                }
                KeyAction::Press { key, repeat } => {
                    for _ in 0..repeat {
                        self.press_named(&key);
                    }
                }
                KeyAction::Hold(key) => self.hold(&key),
                KeyAction::Release(key) => self.release(&key),
            }
            self.settle();
        }
    }

    fn press_named(&self, key: &str) {
        if keyboard::modifier(key).is_some() {
            self.hold(key);
            self.release(key);
            return;
        }
        let modifiers = self.state.borrow().modifiers;
        let gpui_key = keyboard::gpui_key(key);
        self.screen.with_window(|window, cx| {
            if gpui_key == "tab" {
                // Tab only moves focus; it never types a tab character.
                let keystroke = events::named_keystroke("tab", modifiers);
                let handled = events::key_down(window, keystroke.clone(), cx);
                if !handled {
                    if modifiers.shift {
                        window.focus_prev(cx);
                    } else {
                        window.focus_next(cx);
                    }
                    crate::screen::draw(window, cx);
                }
                events::key_up(window, keystroke, cx);
            } else {
                events::press(window, events::named_keystroke(&gpui_key, modifiers), cx);
            }
        });
    }

    fn hold(&self, key: &str) {
        if let Some(modifier) = keyboard::modifier(key) {
            let modifiers = {
                let mut state = self.state.borrow_mut();
                set_modifier(&mut state.modifiers, modifier, true);
                state.modifiers
            };
            self.screen
                .with_window(|window, cx| events::set_modifiers(window, modifiers, cx));
        } else {
            let modifiers = self.state.borrow().modifiers;
            let keystroke = events::named_keystroke(&keyboard::gpui_key(key), modifiers);
            self.screen
                .with_window(|window, cx| events::key_down(window, keystroke, cx));
            self.state.borrow_mut().held_keys.push(key.to_owned());
        }
    }

    fn release(&self, key: &str) {
        if let Some(modifier) = keyboard::modifier(key) {
            let modifiers = {
                let mut state = self.state.borrow_mut();
                set_modifier(&mut state.modifiers, modifier, false);
                state.modifiers
            };
            self.screen
                .with_window(|window, cx| events::set_modifiers(window, modifiers, cx));
        } else {
            let modifiers = self.state.borrow().modifiers;
            let keystroke = events::named_keystroke(&keyboard::gpui_key(key), modifiers);
            self.screen
                .with_window(|window, cx| events::key_up(window, keystroke, cx));
            self.state.borrow_mut().held_keys.retain(|held| held != key);
        }
    }

    /// Clicks `node` to focus it, then types `text` with [`Self::keyboard`] syntax.
    #[track_caller]
    pub fn type_text(&self, node: &Node, text: &str) {
        self.type_with(node, text, TypeOptions::default());
    }

    #[track_caller]
    pub fn type_with(&self, node: &Node, text: &str, options: TypeOptions) {
        if !options.skip_click {
            self.click(node);
        }
        self.keyboard(text);
    }

    /// Focuses an editable element, selects its contents and deletes them.
    #[track_caller]
    pub fn clear(&self, node: &Node) {
        let snapshot = node.snapshot();
        assert!(
            snapshot.role().is_some_and(is_text_field),
            "clear() is only supported on text fields, not {snapshot:?}"
        );
        assert!(
            !snapshot.is_disabled() && !snapshot.is_read_only(),
            "clear() cannot edit a disabled or read-only field: {snapshot:?}"
        );
        self.click(node);
        self.shortcut("a");
        self.keyboard("{Backspace}");
    }

    /// Presses the platform's primary modifier (cmd on macOS, ctrl elsewhere) with `key`.
    fn shortcut(&self, key: &str) {
        let modifiers = Modifiers::secondary_key();
        self.screen.with_window(|window, cx| {
            events::press(window, events::named_keystroke(key, modifiers), cx);
        });
        self.settle();
    }

    /// Presses Tab, moving focus to the next tab stop unless a binding handles it.
    pub fn tab(&self) {
        self.keyboard("{Tab}");
    }

    /// Presses Shift-Tab.
    pub fn tab_backward(&self) {
        self.keyboard("{Shift>}{Tab}{/Shift}");
    }

    /// Copies the focused selection and returns the clipboard text.
    pub fn copy(&self) -> Option<String> {
        self.shortcut("c");
        self.screen
            .read_app(|cx| cx.read_from_clipboard())
            .and_then(|item| item.text())
    }

    /// Cuts the focused selection and returns the clipboard text.
    pub fn cut(&self) -> Option<String> {
        self.shortcut("x");
        self.screen
            .read_app(|cx| cx.read_from_clipboard())
            .and_then(|item| item.text())
    }

    /// Writes `text` to the clipboard and pastes it into the focused element.
    pub fn paste(&self, text: &str) {
        self.screen
            .update_app(|cx| cx.write_to_clipboard(ClipboardItem::new_string(text.to_owned())));
        self.shortcut("v");
    }

    /// Opens `node` (a select, combobox or listbox) if needed, then clicks
    /// each option named in `values`.
    #[track_caller]
    pub fn select_options(&self, node: &Node, values: &[&str]) {
        for value in values {
            let option = match self.find_option(value) {
                Some(option) => option,
                None => {
                    self.click(node);
                    self.find_option(value).unwrap_or_else(|| {
                        panic!(
                            "no option named {value:?} after opening {:?}\n\n{}",
                            node.initial_snapshot(),
                            self.screen.pretty()
                        )
                    })
                }
            };
            self.click(&option);
        }
    }

    /// Clicks each selected option named in `values` to deselect it.
    #[track_caller]
    pub fn deselect_options(&self, node: &Node, values: &[&str]) {
        self.select_options(node, values);
    }

    fn find_option(&self, value: &str) -> Option<Node> {
        const OPTION_ROLES: [Role; 6] = [
            Role::ListBoxOption,
            Role::MenuListOption,
            Role::MenuItem,
            Role::MenuItemRadio,
            Role::MenuItemCheckBox,
            Role::TreeItem,
        ];
        OPTION_ROLES.into_iter().find_map(|role| {
            let mut options = self.screen.query_all(by::role(role).name(value));
            if options.is_empty() {
                options = self
                    .screen
                    .query_all(by::text(value))
                    .into_iter()
                    .filter(|option| option.role() == Some(role))
                    .collect();
            }
            options.into_iter().next()
        })
    }

    /// Clicks `node`, which must open GPUI's "Open" dialog, and chooses `paths`.
    #[track_caller]
    pub fn upload(&self, node: &Node, paths: impl IntoIterator<Item = impl Into<PathBuf>>) {
        let paths: Vec<PathBuf> = paths.into_iter().map(Into::into).collect();
        let cx = self.screen.test_cx().expect(
            "upload() answers the test platform's file prompt, so it needs a TestAppContext",
        );
        self.click(node);
        assert!(
            cx.did_prompt_for_paths(),
            "clicking {:?} did not open a file prompt",
            node.initial_snapshot()
        );
        cx.simulate_path_prompt_response(move |_| Some(paths));
        self.settle();
    }
}

/// Options for [`UserEvent::type_with`].
#[derive(Clone, Copy, Debug, Default)]
pub struct TypeOptions {
    skip_click: bool,
}

impl TypeOptions {
    pub fn new() -> Self {
        Self::default()
    }
    /// Types at the current focus without clicking the element first.
    pub fn with_skip_click(mut self, skip_click: bool) -> Self {
        self.skip_click = skip_click;
        self
    }
}

fn set_modifier(modifiers: &mut Modifiers, modifier: Modifier, on: bool) {
    match modifier {
        Modifier::Shift => modifiers.shift = on,
        Modifier::Control => modifiers.control = on,
        Modifier::Alt => modifiers.alt = on,
        Modifier::Platform => modifiers.platform = on,
        Modifier::Function => modifiers.function = on,
    }
}
