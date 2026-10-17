//! Native GPUI input dispatch shared by `fire_event` and `user_event`.
//! Every step completes a frame, so later steps hit-test the updated UI.
use crate::screen::draw;
use gpui::{
    App, InputEvent, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, ModifiersChangedEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, ScrollDelta,
    ScrollWheelEvent, Window,
};

pub(crate) fn mouse_move(
    window: &mut Window,
    position: Point<Pixels>,
    pressed_button: Option<MouseButton>,
    cx: &mut App,
) {
    let modifiers = window.modifiers();
    window.dispatch_event(
        MouseMoveEvent {
            position,
            pressed_button,
            modifiers,
        }
        .to_platform_input(),
        cx,
    );
    draw(window, cx);
}

pub(crate) fn mouse_down(
    window: &mut Window,
    position: Point<Pixels>,
    button: MouseButton,
    click_count: usize,
    cx: &mut App,
) {
    let modifiers = window.modifiers();
    window.dispatch_event(
        MouseDownEvent {
            button,
            position,
            modifiers,
            click_count,
            first_mouse: false,
        }
        .to_platform_input(),
        cx,
    );
    draw(window, cx);
}

pub(crate) fn mouse_up(
    window: &mut Window,
    position: Point<Pixels>,
    button: MouseButton,
    click_count: usize,
    cx: &mut App,
) {
    let modifiers = window.modifiers();
    window.dispatch_event(
        MouseUpEvent {
            button,
            position,
            modifiers,
            click_count,
        }
        .to_platform_input(),
        cx,
    );
    draw(window, cx);
}

/// Moves to `position`, then presses and releases `count` times.
pub(crate) fn click(
    window: &mut Window,
    position: Point<Pixels>,
    button: MouseButton,
    count: usize,
    cx: &mut App,
) {
    mouse_move(window, position, None, cx);
    for click_count in 1..=count {
        mouse_down(window, position, button, click_count, cx);
        mouse_up(window, position, button, click_count, cx);
    }
}

pub(crate) fn scroll(
    window: &mut Window,
    position: Point<Pixels>,
    delta: ScrollDelta,
    cx: &mut App,
) {
    mouse_move(window, position, None, cx);
    let modifiers = window.modifiers();
    window.dispatch_event(
        ScrollWheelEvent {
            position,
            delta,
            modifiers,
            ..Default::default()
        }
        .to_platform_input(),
        cx,
    );
    draw(window, cx);
}

pub(crate) fn set_modifiers(window: &mut Window, modifiers: Modifiers, cx: &mut App) {
    if window.modifiers() != modifiers {
        let capslock = window.capslock();
        window.dispatch_event(
            ModifiersChangedEvent {
                modifiers,
                capslock,
            }
            .to_platform_input(),
            cx,
        );
        draw(window, cx);
    }
}

/// Dispatches key-down without inserting text. Returns whether a handler
/// stopped propagation.
pub(crate) fn key_down(window: &mut Window, keystroke: Keystroke, cx: &mut App) -> bool {
    let result = window.dispatch_event(
        KeyDownEvent {
            keystroke,
            is_held: false,
            prefer_character_input: false,
        }
        .to_platform_input(),
        cx,
    );
    draw(window, cx);
    !result.propagate
}

pub(crate) fn key_up(window: &mut Window, keystroke: Keystroke, cx: &mut App) {
    window.dispatch_event(KeyUpEvent { keystroke }.to_platform_input(), cx);
    draw(window, cx);
}

/// A full key press as a user makes it: key-down, then the typed text goes
/// to the focused input handler if no binding handled the key, then key-up.
/// Returns whether a binding or handler consumed the key-down.
pub(crate) fn press(window: &mut Window, keystroke: Keystroke, cx: &mut App) -> bool {
    let handled = window.dispatch_keystroke(keystroke.clone(), cx);
    draw(window, cx);
    // Controls such as buttons activate on key-up, so always complete the pair.
    key_up(window, keystroke, cx);
    handled
}

/// A keystroke typing one character, with the held modifiers.
pub(crate) fn char_keystroke(character: char, modifiers: Modifiers) -> Keystroke {
    let text = character.to_string();
    let typing = !(modifiers.control || modifiers.alt || modifiers.platform || modifiers.function);
    let key = match character {
        ' ' => "space".to_owned(),
        '\n' => "enter".to_owned(),
        '\t' => "tab".to_owned(),
        _ => text.to_lowercase(),
    };
    Keystroke {
        modifiers: Modifiers {
            shift: modifiers.shift || (character.is_uppercase() && typing),
            ..modifiers
        },
        key,
        key_char: typing.then_some(text),
    }
}

/// A keystroke for a named key, such as `enter` or `left`.
pub(crate) fn named_keystroke(key: &str, modifiers: Modifiers) -> Keystroke {
    Keystroke {
        modifiers,
        key: key.to_owned(),
        key_char: None,
    }
    .with_simulated_ime()
}

pub(crate) fn parse_keystroke(key: &str) -> Keystroke {
    Keystroke::parse(key).unwrap_or_else(|error| panic!("invalid keystroke {key:?}: {error}"))
}
