//! Private prototype wire contract, not authorization or a production transport.
/// N0 public design DTOs only; no decoder, admission or dispatch implementation.
pub mod experimental_native;
/// Separately admitted, bounded headless write-boundary experiment.
pub mod h0_write;
pub mod internal_engine;
pub mod mk1;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_FRAME: usize = 65_536;
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub session: String,
    pub incarnation: String,
    pub endpoint: String,
    pub generation: String,
    pub prefix: String,
    pub geometry: String,
    pub profile: String,
    pub focus: String,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Intent {
    Key {
        key: Key,
        shift: bool,
        alt: bool,
        ctrl: bool,
    },
    Text {
        text: String,
    },
    Paste {
        text: String,
    },
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Tab,
    Enter,
    Escape,
    Backspace,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
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
    KeypadEnter,
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
    KeypadAdd,
    KeypadSubtract,
    KeypadMultiply,
    KeypadDivide,
    KeypadDecimal,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InputRequest {
    pub version: u8,
    pub request_id: String,
    pub context: Context,
    pub intent: Intent,
}
#[derive(Debug, PartialEq, Eq)]
pub enum DecodeError {
    Size,
    Shape,
    Version,
    Decimal,
    Identity,
}

// Derived struct decoding rejects duplicate and unknown fields (no intermediate JSON map).
pub fn decode(bytes: &[u8]) -> Result<InputRequest, DecodeError> {
    if bytes.is_empty() || bytes.len() > MAX_FRAME {
        return Err(DecodeError::Size);
    }
    let request: InputRequest = serde_json::from_slice(bytes).map_err(|_| DecodeError::Shape)?;
    validate(&request)?;
    Ok(request)
}
pub fn validate(request: &InputRequest) -> Result<(), DecodeError> {
    if request.version != 1 {
        return Err(DecodeError::Version);
    }
    let c = &request.context;
    for id in [&request.request_id, &c.session, &c.incarnation, &c.endpoint] {
        if id.is_empty()
            || id.len() > 128
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_:".contains(&b))
        {
            return Err(DecodeError::Identity);
        }
    }
    for n in [&c.generation, &c.prefix, &c.geometry, &c.profile, &c.focus] {
        if n.is_empty()
            || n.len() > 20
            || (n.len() > 1 && n.starts_with('0'))
            || !n.bytes().all(|b| b.is_ascii_digit())
            || n.parse::<u64>().is_err()
        {
            return Err(DecodeError::Decimal);
        }
    }
    Ok(())
}
pub fn schema() -> schemars::Schema {
    schemars::generate::SchemaSettings::draft07()
        .into_generator()
        .into_root_schema_for::<InputRequest>()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixture_and_duplicate_keys() {
        let fixture = include_str!("../../../tests/fixtures/input/request.json");
        assert!(decode(fixture.as_bytes()).is_ok());
        assert_eq!(
            decode(
                fixture
                    .replacen("\"version\": 1", "\"version\": 1, \"version\": 1", 1)
                    .as_bytes()
            )
            .err(),
            Some(DecodeError::Shape)
        );
        assert_eq!(
            decode(
                fixture
                    .replace(
                        "\"generation\": \"1\"",
                        "\"generation\": \"18446744073709551616\""
                    )
                    .as_bytes()
            )
            .err(),
            Some(DecodeError::Decimal)
        );
        assert_eq!(
            decode(&vec![b' '; MAX_FRAME + 1]).err(),
            Some(DecodeError::Size)
        );
    }
    #[test]
    fn unicode_scalar_values_are_lossless_or_rejected() {
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/input/request.json"))
                .unwrap();
        value["intent"] = serde_json::json!({"kind":"text","text":"界😀"});
        let good = serde_json::to_string(&value).unwrap();
        assert!(decode(good.as_bytes()).is_ok());
        let bad = good.replace("界😀", "\\ud800");
        assert_eq!(decode(bad.as_bytes()).err(), Some(DecodeError::Shape));
    }
    proptest::proptest! { #[test] fn arbitrary_frames_do_not_panic(bytes in proptest::collection::vec(proptest::num::u8::ANY,0..2000)) { let _=decode(&bytes); } }
}
