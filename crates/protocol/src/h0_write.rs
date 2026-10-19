//! Experimental local headless proof, deliberately separate from MK1/1 and N0.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_FRAME: usize = 4096;
pub const MAX_TEXT: usize = 256;
pub const RECEIPTS: u16 = 32;
pub const QUEUE: usize = 2;
pub const MAX_TTL_MS: u16 = 250;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Version {
    #[serde(rename = "namtab.h0-write/0")]
    V0,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub epoch: String,
    pub principal: String,
    pub run: String,
    pub namespace: String,
    pub connection: u16,
    pub control: u16,
    pub capability: u16,
    pub session: String,
    pub incarnation: u16,
    pub context: String,
    pub task: String,
    pub attempt: String,
    pub assignment: u16,
    pub writer: u16,
    pub guard: u16,
    pub revision: u64,
    pub mode_revision: u64,
    pub cols: u16,
    pub rows: u16,
    pub profile: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Intent {
    Text { text: String },
    Enter {},
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    Observe {},
    Queue {
        sequence: u16,
        binding: Binding,
        ttl_ms: u16,
        intent: Intent,
    },
    Dispatch {
        sequence: u16,
    },
    Query {
        sequence: u16,
    },
    Transfer {
        sequence: u16,
        binding: Binding,
        target_run: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: Version,
    pub id: u16,
    pub operation: Operation,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Delivery {
    Queued,
    InFlight,
    Full,
    Zero,
    Partial,
    Unknown,
    Transferred,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub sequence: u16,
    /// Frozen admission origin, never rewritten to a successor holder.
    pub origin: Binding,
    pub delivery: Delivery,
    pub requested: usize,
    pub accepted: Option<usize>,
    pub reason: String,
    pub calls: u16,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub binding: Binding,
    pub holder: Option<String>,
    pub input_known: bool,
    pub output_pending: bool,
    pub active: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Body {
    Observation { value: Observation },
    Receipt { value: Receipt },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub version: Version,
    pub id: u16,
    pub result: Result<Body, String>,
}

pub fn decode(bytes: &[u8]) -> Result<Request, &'static str> {
    if bytes.len() > MAX_FRAME {
        return Err("frame_capacity");
    }
    let request: Request = serde_json::from_slice(bytes).map_err(|_| "invalid_request")?;
    if request.id == 0 {
        return Err("zero_correlation");
    }
    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reject_other_protocols_duplicate_and_unknown_fields_before_dispatch() {
        let valid = r#"{"version":"namtab.h0-write/0","id":1,"operation":{"op":"observe"}}"#;
        assert!(decode(valid.as_bytes()).is_ok());
        for bad in [
            valid.replace("namtab.h0-write/0", "namtab.mk1/1"),
            valid.replace("\"id\":1", "\"id\":1,\"id\":2"),
            valid.replace("\"observe\"", "\"observe\",\"authority\":true"),
            valid.replace("\"id\":1", "\"id\":0"),
            " ".repeat(MAX_FRAME + 1),
        ] {
            assert!(decode(bad.as_bytes()).is_err());
        }
    }
}
