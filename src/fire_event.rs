//! Testing Library's `fireEvent`: one native event at a time.
//!
//! Prefer [`crate::UserEvent`], which performs the full sequence a user
//! produces. Use these when a test needs a single step, such as a mouse-down
//! without the matching mouse-up.
use crate::{events, node::Node, screen::Screen};
use gpui::{
    Action, InputHandler, Modifiers, MouseButton, Pixels, PlatformInput, Point, ScrollDelta,
};

/// Dispatches single events to a screen. Created by [`Screen::fire_event`].
pub struct FireEvent {
    screen: Screen,
}

/// The window-local center of `node`, which must be visible.
#[track_caller]
pub(crate) fn center(node: &Node) -> Point<Pixels> {
    let snapshot = node.snapshot();
    assert!(
        node.is_connected(),
        "cannot interact with {snapshot:?}: it is no longer rendered"
    );
    assert!(
        snapshot.is_visible(),
        "cannot interact with {snapshot:?}: it is not visible"
    );
    // The center of what is painted, so a partly scrolled-out element is
    // hit where the user can see it.
    snapshot.visible_bounds().center()
}

impl FireEvent {
    pub(crate) fn new(screen: Screen) -> Self {
        Self { screen }
    }

    /// Moves the pointer to the center of `node`.
    #[track_caller]
    pub fn mouse_move(&self, node: &Node) {
        let position = center(node);
        self.screen
            .with_window(|window, cx| events::mouse_move(window, position, None, cx));
    }
    /// Moves the pointer onto `node`, firing GPUI's hover handlers.
    #[track_caller]
    pub fn mouse_enter(&self, node: &Node) {
        self.mouse_move(node);
    }
    /// Moves the pointer outside the window.
    pub fn mouse_leave(&self) {
        self.screen.with_window(|window, cx| {
            events::mouse_move(
                window,
                Point::new(Pixels::from(-1.), Pixels::from(-1.)),
                None,
                cx,
            )
        });
    }
    /// Presses the left button at the center of `node`, without moving there first.
    #[track_caller]
    pub fn mouse_down(&self, node: &Node) {
        let position = center(node);
        self.screen.with_window(|window, cx| {
            events::mouse_down(window, position, MouseButton::Left, 1, cx)
        });
    }
    /// Releases the left button at the center of `node`.
    #[track_caller]
    pub fn mouse_up(&self, node: &Node) {
        let position = center(node);
        self.screen
            .with_window(|window, cx| events::mouse_up(window, position, MouseButton::Left, 1, cx));
    }
    /// Moves to `node`, then presses and releases the left button. GPUI hit-tests
    /// the pointer position, so a click always includes the move.
    #[track_caller]
    pub fn click(&self, node: &Node) {
        let position = center(node);
        self.screen
            .with_window(|window, cx| events::click(window, position, MouseButton::Left, 1, cx));
    }
    #[track_caller]
    pub fn double_click(&self, node: &Node) {
        let position = center(node);
        self.screen
            .with_window(|window, cx| events::click(window, position, MouseButton::Left, 2, cx));
    }
    /// A right click, which opens context menus.
    #[track_caller]
    pub fn context_menu(&self, node: &Node) {
        let position = center(node);
        self.screen
            .with_window(|window, cx| events::click(window, position, MouseButton::Right, 1, cx));
    }
    /// Sends a scroll wheel event over `node`, in GPUI's delta units and sign.
    #[track_caller]
    pub fn scroll(&self, node: &Node, delta: ScrollDelta) {
        let position = center(node);
        self.screen
            .with_window(|window, cx| events::scroll(window, position, delta, cx));
    }
    /// Key-down for a GPUI keystroke such as `"enter"` or `"cmd-s"`; no text is typed.
    pub fn key_down(&self, key: &str) {
        let keystroke = events::parse_keystroke(key);
        self.screen
            .with_window(|window, cx| events::key_down(window, keystroke, cx));
    }
    pub fn key_up(&self, key: &str) {
        let keystroke = events::parse_keystroke(key);
        self.screen
            .with_window(|window, cx| events::key_up(window, keystroke, cx));
    }
    /// Key-down, typed text if unhandled, then key-up.
    pub fn key_press(&self, key: &str) {
        let keystroke = events::parse_keystroke(key);
        self.screen.with_window(|window, cx| {
            events::press(window, keystroke, cx);
        });
    }
    /// Changes the held modifier keys.
    pub fn modifiers_changed(&self, modifiers: Modifiers) {
        self.screen
            .with_window(|window, cx| events::set_modifiers(window, modifiers, cx));
    }
    /// Inserts `text` through `handler`, like a committed IME composition,
    /// replacing the current selection. GPUI keeps the window's own input
    /// handler private, so build one with `ElementInputHandler::new`.
    pub fn input(&self, mut handler: impl InputHandler, text: &str) {
        self.screen.update(|window, cx| {
            handler.replace_text_in_range(None, text, window, cx);
        });
    }
    /// Dispatches a GPUI action to the focused element.
    pub fn action(&self, action: &dyn Action) {
        self.screen
            .update(|window, cx| window.dispatch_action(action.boxed_clone(), cx));
        self.screen.run_until_parked();
        self.screen.redraw();
    }
    /// Dispatches any platform input.
    pub fn dispatch(&self, event: PlatformInput) {
        self.screen.update(|window, cx| {
            window.dispatch_event(event, cx);
        });
    }
}
