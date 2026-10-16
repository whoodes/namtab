//! Logical input encoding. This does not infer physical keys or application focus.
use crate::{Engine, repertoire};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Modifiers {
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub ctrl: bool,
}

impl Modifiers {
    fn parameter(self) -> u8 {
        1 + u8::from(self.shift) + 2 * u8::from(self.alt) + 4 * u8::from(self.ctrl)
    }
    fn none(self) -> bool {
        self.parameter() == 1
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyPhase {
    Press,
    Repeat,
    Release,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Key {
    Character(char),
    Tab,
    Enter,
    Escape,
    Backspace,
    Up,
    Down,
    Right,
    Left,
    Home,
    End,
    Insert,
    Delete,
    PageUp,
    PageDown,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Keypad0,
    Keypad1,
    Keypad2,
    Keypad3,
    Keypad4,
    Keypad5,
    Keypad6,
    Keypad7,
    Keypad8,
    Keypad9,
    KeypadDecimal,
    KeypadDivide,
    KeypadMultiply,
    KeypadSubtract,
    KeypadAdd,
    KeypadEnter,
    KeypadComma,
    KeypadEqual,
    KeypadTab,
    KeypadSpace,
}

/// No raw byte route: all application input uses known canonical modes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Input {
    Key {
        key: Key,
        modifiers: Modifiers,
        phase: KeyPhase,
    },
    Text {
        text: String,
    },
    Paste {
        text: String,
    },
    Focus {
        focused: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputError {
    Closed,
    UnknownModes,
    UnsupportedKey,
    UnsupportedModifiers,
    UnsupportedRelease,
    UnsupportedText,
    PasteNotEnabled,
    FocusNotEnabled,
    TooLarge,
}

impl Engine {
    pub fn encode(&self, input: &Input) -> Result<Vec<u8>, InputError> {
        if self.closed {
            return Err(InputError::Closed);
        }
        if self.unknown || !self.modes.input_protocol_known {
            return Err(InputError::UnknownModes);
        }
        match input {
            Input::Text { text } => {
                validate_text(text, false)?;
                Ok(text.as_bytes().to_vec())
            }
            Input::Paste { text } => {
                validate_text(text, true)?;
                if !self.modes.bracketed_paste {
                    return Err(InputError::PasteNotEnabled);
                }
                let mut bytes = Vec::with_capacity(text.len() + 12);
                bytes.extend_from_slice(b"\x1b[200~");
                bytes.extend_from_slice(text.replace("\r\n", "\n").replace('\r', "\n").as_bytes());
                bytes.extend_from_slice(b"\x1b[201~");
                Ok(bytes)
            }
            Input::Focus { focused } => {
                if !self.modes.focus_reporting {
                    return Err(InputError::FocusNotEnabled);
                }
                Ok(if *focused { b"\x1b[I" } else { b"\x1b[O" }.to_vec())
            }
            Input::Key {
                key,
                modifiers,
                phase,
            } => {
                if *phase == KeyPhase::Release {
                    return Err(InputError::UnsupportedRelease);
                }
                self.encode_key(key, *modifiers)
            }
        }
    }

    fn encode_key(&self, key: &Key, m: Modifiers) -> Result<Vec<u8>, InputError> {
        if let Key::Character(ch) = key {
            if m.shift {
                return Err(InputError::UnsupportedModifiers);
            }
            let payload = if m.ctrl {
                vec![match ch.to_ascii_uppercase() {
                    'A'..='Z' => ch.to_ascii_uppercase() as u8 - b'A' + 1,
                    ' ' | '@' => 0,
                    '[' => 27,
                    '\\' => 28,
                    ']' => 29,
                    '^' => 30,
                    '_' => 31,
                    '?' => 127,
                    _ => return Err(InputError::UnsupportedKey),
                }]
            } else if repertoire(*ch) {
                let mut encoded = [0; 4];
                ch.encode_utf8(&mut encoded).as_bytes().to_vec()
            } else {
                return Err(InputError::UnsupportedText);
            };
            return Ok(if m.alt {
                [b"\x1b".as_slice(), &payload].concat()
            } else {
                payload
            });
        }
        if *key == Key::Tab && m.shift && !m.ctrl && !m.alt {
            return Ok(b"\x1b[Z".to_vec());
        }
        let named = match key {
            Key::Tab => Some(9),
            Key::Enter => Some(13),
            Key::Escape => Some(27),
            Key::Backspace => Some(if self.modes.backarrow_bs { 8 } else { 127 }),
            _ => None,
        };
        if let Some(byte) = named {
            return if m.none() {
                Ok(vec![byte])
            } else {
                Err(InputError::UnsupportedModifiers)
            };
        }
        let final_byte = match key {
            Key::Up => Some('A'),
            Key::Down => Some('B'),
            Key::Right => Some('C'),
            Key::Left => Some('D'),
            Key::Home => Some('H'),
            Key::End => Some('F'),
            Key::F1 => Some('P'),
            Key::F2 => Some('Q'),
            Key::F3 => Some('R'),
            Key::F4 => Some('S'),
            _ => None,
        };
        if let Some(final_byte) = final_byte {
            if !m.none() {
                return Ok(format!("\x1b[1;{}{final_byte}", m.parameter()).into_bytes());
            }
            let ss3 = matches!(key, Key::F1 | Key::F2 | Key::F3 | Key::F4)
                || self.modes.application_cursor;
            return Ok(format!("\x1b{}{final_byte}", if ss3 { 'O' } else { '[' }).into_bytes());
        }
        let tilde = match key {
            Key::Insert => Some(2),
            Key::Delete => Some(3),
            Key::PageUp => Some(5),
            Key::PageDown => Some(6),
            Key::F5 => Some(15),
            Key::F6 => Some(17),
            Key::F7 => Some(18),
            Key::F8 => Some(19),
            Key::F9 => Some(20),
            Key::F10 => Some(21),
            Key::F11 => Some(23),
            Key::F12 => Some(24),
            _ => None,
        };
        if let Some(number) = tilde {
            return Ok(if m.none() {
                format!("\x1b[{number}~")
            } else {
                format!("\x1b[{number};{}~", m.parameter())
            }
            .into_bytes());
        }
        if !m.none() {
            return Err(InputError::UnsupportedModifiers);
        }
        let (normal, application) = match key {
            Key::Keypad0 => (b'0', b'p'),
            Key::Keypad1 => (b'1', b'q'),
            Key::Keypad2 => (b'2', b'r'),
            Key::Keypad3 => (b'3', b's'),
            Key::Keypad4 => (b'4', b't'),
            Key::Keypad5 => (b'5', b'u'),
            Key::Keypad6 => (b'6', b'v'),
            Key::Keypad7 => (b'7', b'w'),
            Key::Keypad8 => (b'8', b'x'),
            Key::Keypad9 => (b'9', b'y'),
            Key::KeypadDecimal => (b'.', b'n'),
            Key::KeypadDivide => (b'/', b'o'),
            Key::KeypadMultiply => (b'*', b'j'),
            Key::KeypadSubtract => (b'-', b'm'),
            Key::KeypadAdd => (b'+', b'k'),
            Key::KeypadEnter => (13, b'M'),
            Key::KeypadComma => (b',', b'l'),
            Key::KeypadEqual => (b'=', b'X'),
            Key::KeypadTab => (9, b'I'),
            Key::KeypadSpace => (b' ', b' '),
            _ => return Err(InputError::UnsupportedKey),
        };
        Ok(if self.modes.application_keypad {
            vec![27, b'O', application]
        } else {
            vec![normal]
        })
    }
}

fn validate_text(text: &str, paste: bool) -> Result<(), InputError> {
    if text.len() > 8192 {
        return Err(InputError::TooLarge);
    }
    if !text
        .chars()
        .all(|ch| repertoire(ch) || (paste && matches!(ch, '\r' | '\n' | '\t')))
    {
        return Err(InputError::UnsupportedText);
    }
    Ok(())
}
