//! EA1 private parent/worker pipe protocol, not a frontend or authorization API.
//! Incompatible development changes require a protocol bump. Never expose this pipe.
use crate::{Intent, Key};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const VERSION: u8 = 1;
pub const MAX_REQUEST: usize = 65_536;
pub const MAX_RESPONSE: usize = 2_097_152;
pub const MAX_BYTES: usize = 8192;
pub const MAX_CELLS: usize = 4096;
pub const PROFILE: &str = "ea1-22fa134a-release-safe-simd-off-v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Stamp {
    pub session: String,
    pub incarnation: String,
    pub revision: u32,
    pub geometry: u32,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Input {
    Key {
        key: Key,
        shift: bool,
        alt: bool,
        ctrl: bool,
    },
    Character {
        text: char,
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
    Focus {
        gained: bool,
    },
    Mouse {
        x: u16,
        y: u16,
        button: u8,
        release: bool,
        shift: bool,
        alt: bool,
        ctrl: bool,
    },
}
impl From<Intent> for Input {
    fn from(value: Intent) -> Self {
        match value {
            Intent::Key {
                key,
                shift,
                alt,
                ctrl,
            } => Self::Key {
                key,
                shift,
                alt,
                ctrl,
            },
            Intent::Text { text } => Self::Text { text },
            Intent::Paste { text } => Self::Paste { text },
        }
    }
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    Init {
        session: String,
        incarnation: String,
        cols: u16,
        rows: u16,
    },
    Observe {},
    Feed {
        bytes: Vec<u8>,
    },
    Resize {
        cols: u16,
        rows: u16,
    },
    Encode {
        input: Input,
    },
    Shutdown {},
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u8,
    pub seq: u32,
    pub expected: Option<Stamp>,
    pub operation: Operation,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Cell {
    pub text: String,
    /// Ghostty public wide kind: narrow=0, wide=1, spacer-tail=2, spacer-head=3.
    pub wide: u32,
    pub flags: u32,
    pub underline: u32,
    pub fg: u32,
    pub bg: u32,
    pub underline_color: Option<u32>,
    pub fg_palette: Option<u8>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Frame {
    pub cols: u16,
    pub rows: u16,
    pub cursor: Option<[u16; 2]>,
    pub cursor_visible: bool,
    pub cursor_blink: bool,
    pub cursor_style: u32,
    pub password_hint: bool,
    pub alternate: bool,
    /// Private prototype bitset, documented in EA1. Includes sync-output hold.
    pub modes: u32,
    pub ground: bool,
    pub dirty: u32,
    pub cells: Vec<Cell>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Error {
    Malformed,
    Version,
    Sequence,
    Identity,
    Stale,
    Lifecycle,
    Limit,
    Unsupported,
    Native,
    Unknown,
}
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct Reply {
    /// One batch per work sequence, not one per query or callback. Never byte-dedup.
    pub work: u32,
    pub origin: String,
    pub bytes: Vec<u8>,
}
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct Response {
    pub version: u8,
    pub seq: u32,
    pub profile: String,
    pub stamp: Option<Stamp>,
    pub status: String,
    pub error: Option<Error>,
    pub frame: Option<Frame>,
    pub input_bytes: Option<Vec<u8>>,
    pub reply: Option<Reply>,
}
pub fn request_schema() -> schemars::Schema {
    schemars::generate::SchemaSettings::draft07()
        .into_generator()
        .into_root_schema_for::<Request>()
}

pub fn response_schema() -> schemars::Schema {
    schemars::generate::SchemaSettings::draft07()
        .into_generator()
        .into_root_schema_for::<Response>()
}
