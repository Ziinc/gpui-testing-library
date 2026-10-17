//! user-event's `keyboard` text syntax.
//!
//! - Plain characters are typed: `"hello"`.
//! - `{Enter}` or `[Enter]` presses a named key.
//! - `{Shift>}` holds a key until `{/Shift}`; `{a>3}` presses `a` three times.
//! - `{{` and `[[` type a literal `{` or `[`.

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum KeyAction {
    Type(char),
    /// Press a named key, `repeat` times.
    Press {
        key: String,
        repeat: usize,
    },
    Hold(String),
    Release(String),
}

pub(crate) fn parse(text: &str) -> Vec<KeyAction> {
    let mut actions = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        let close = match character {
            '{' => '}',
            '[' => ']',
            _ => {
                actions.push(KeyAction::Type(character));
                continue;
            }
        };
        if chars.peek() == Some(&character) {
            chars.next();
            actions.push(KeyAction::Type(character));
            continue;
        }
        let mut descriptor = String::new();
        loop {
            match chars.next() {
                Some(next) if next == close => break,
                Some(next) => descriptor.push(next),
                None => panic!("unterminated key descriptor {character}{descriptor} in {text:?}"),
            }
        }
        actions.push(descriptor_action(&descriptor, text));
    }
    actions
}

fn descriptor_action(descriptor: &str, text: &str) -> KeyAction {
    assert!(!descriptor.is_empty(), "empty key descriptor in {text:?}");
    let key_name = |key: &str| {
        assert!(
            !key.is_empty(),
            "missing key name in {{{descriptor}}} of {text:?}"
        );
        key.to_owned()
    };
    if let Some(key) = descriptor.strip_prefix('/') {
        return KeyAction::Release(key_name(key));
    }
    if let Some((key, rest)) = descriptor.split_once('>') {
        let rest = rest.trim_end_matches('/');
        if rest.is_empty() {
            return KeyAction::Hold(key_name(key));
        }
        let repeat = rest
            .parse()
            .unwrap_or_else(|_| panic!("invalid repeat count in {{{descriptor}}} of {text:?}"));
        return KeyAction::Press {
            key: key_name(key),
            repeat,
        };
    }
    KeyAction::Press {
        key: descriptor.to_owned(),
        repeat: 1,
    }
}

/// The modifier a held key sets, if any.
pub(crate) fn modifier(key: &str) -> Option<Modifier> {
    Some(match key {
        "Shift" | "ShiftLeft" | "ShiftRight" => Modifier::Shift,
        "Control" | "ControlLeft" | "ControlRight" | "Ctrl" => Modifier::Control,
        "Alt" | "AltLeft" | "AltRight" | "Option" => Modifier::Alt,
        "Meta" | "MetaLeft" | "MetaRight" | "Cmd" | "Command" | "Super" => Modifier::Platform,
        "Fn" => Modifier::Function,
        _ => return None,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Modifier {
    Shift,
    Control,
    Alt,
    Platform,
    Function,
}

/// Maps a DOM `KeyboardEvent.key` name to GPUI's key name. Single characters
/// map to themselves; anything unknown is lowercased.
pub(crate) fn gpui_key(key: &str) -> String {
    match key {
        "Enter" | "NumpadEnter" => "enter",
        "Tab" => "tab",
        "Backspace" => "backspace",
        "Delete" => "delete",
        "Escape" | "Esc" => "escape",
        "Space" | " " => "space",
        "ArrowLeft" => "left",
        "ArrowRight" => "right",
        "ArrowUp" => "up",
        "ArrowDown" => "down",
        "Home" => "home",
        "End" => "end",
        "PageUp" => "pageup",
        "PageDown" => "pagedown",
        "Insert" => "insert",
        "ContextMenu" => "menu",
        key if key.chars().count() == 1 => return key.to_lowercase(),
        key => return key.to_lowercase(),
    }
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{KeyAction::*, *};

    #[test]
    fn parses_user_event_syntax() {
        assert_eq!(
            parse("a{Enter}{Shift>}B{/Shift}{{[ArrowLeft]{x>2}"),
            vec![
                Type('a'),
                Press {
                    key: "Enter".into(),
                    repeat: 1
                },
                Hold("Shift".into()),
                Type('B'),
                Release("Shift".into()),
                Type('{'),
                Press {
                    key: "ArrowLeft".into(),
                    repeat: 1
                },
                Press {
                    key: "x".into(),
                    repeat: 2
                },
            ]
        );
    }

    #[test]
    fn maps_dom_key_names() {
        assert_eq!(gpui_key("ArrowLeft"), "left");
        assert_eq!(gpui_key("F5"), "f5");
        assert_eq!(gpui_key("A"), "a");
    }

    #[test]
    #[should_panic(expected = "unterminated")]
    fn rejects_unterminated_descriptors() {
        parse("{Enter");
    }

    #[test]
    #[should_panic(expected = "missing key name")]
    fn rejects_descriptors_without_a_key() {
        parse("{>2}");
    }

    proptest::proptest! {
        /// Escaping `{` and `[` types any text literally.
        #[test]
        fn escaped_text_types_literally(text in "\\PC*") {
            let escaped = text.replace('{', "{{").replace('[', "[[");
            let typed: Vec<_> = text.chars().map(Type).collect();
            proptest::prop_assert_eq!(parse(&escaped), typed);
        }

        /// Arbitrary input either parses or fails with one of the parser's
        /// own messages, never an unrelated panic.
        #[test]
        fn arbitrary_input_fails_only_with_parser_errors(text in "[{}\\[\\]/>a-zA-Z0-9 ]{0,12}") {
            let result = std::panic::catch_unwind(|| parse(&text));
            if let Err(payload) = result {
                let message = payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| payload.downcast_ref::<&str>().map(|text| text.to_string()))
                    .unwrap_or_default();
                proptest::prop_assert!(
                    ["unterminated", "empty key descriptor", "missing key name", "invalid repeat"]
                        .iter()
                        .any(|expected| message.contains(expected)),
                    "{message}"
                );
            }
        }

        /// Every parsed hold names a key, so releases can match it.
        #[test]
        fn parsed_actions_always_name_a_key(text in "[{}\\[\\]/>a-z0-9]{0,12}") {
            if let Ok(actions) = std::panic::catch_unwind(|| parse(&text)) {
                for action in actions {
                    if let Press { key, .. } | Hold(key) | Release(key) = action {
                        proptest::prop_assert!(!key.is_empty());
                    }
                }
            }
        }
    }
}
