//! Separately admitted local experiment. Not a public transport or MK1 extension.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub const MAX_FRAME: usize = 4096;
pub const MAX_RESPONSE: usize = 16384;
pub const MAX_ARTIFACT: usize = 256;
pub const MAX_EVENTS: usize = 128;
pub const PAGE: usize = 16;
pub const STREAM_BYTES: usize = 4096;
pub const SEQUENCES: u16 = 16;
pub const PROFILE: &str = "h0-flow-once/0";
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Version {
    #[serde(rename = "namtab.flow/0")]
    V0,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Run {
    Producer,
    Consumer,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub epoch: String,
    pub context: String,
    pub run: Run,
    pub task: String,
    pub attempt: String,
    pub connection: u16,
    pub control: u16,
    pub profile: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub epoch: String,
    pub context: String,
    pub producer_attempt: String,
    pub revision: u16,
    pub sha256: String,
    pub bytes: u16,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    Observe {
        after: u16,
    },
    Queue {
        sequence: u16,
        binding: Box<Binding>,
        dependency: Option<Artifact>,
    },
    Dispatch {
        sequence: u16,
    },
    Takeover {
        expected_control: u16,
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
pub enum State {
    Queued,
    Running,
    Completed,
    Failed,
    Fenced,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub sequence: u16,
    pub origin: Binding,
    pub dependency: Option<Artifact>,
    pub state: State,
    pub calls: u16,
    pub reason: String,
    pub exit_code: Option<i32>,
    pub drained: bool,
    pub stdout_bytes: u16,
    pub stderr_bytes: u16,
    pub stdout_sha256: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Stream {
    Stdout,
    Stderr,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub id: u16,
    pub kind: String,
    pub origin: Option<Binding>,
    pub stream: Option<Stream>,
    pub offset: u16,
    pub bytes: u16,
    pub sha256: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub control: u16,
    pub human: bool,
    pub admission_known: bool,
    pub binding: Option<Binding>,
    pub artifact: Option<Artifact>,
    pub receipts: Vec<Receipt>,
    pub events: Vec<Event>,
    /// Global scanned cut, including filtered events. Advance only to this cut.
    pub cursor: u16,
    pub more: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Body {
    Observation { value: Observation },
    Receipt { value: Receipt },
    Taken { control: u16, running: u16 },
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
    fn strict_and_separate_wire() {
        let valid = r#"{"version":"namtab.flow/0","id":1,"operation":{"op":"observe","after":0}}"#;
        assert!(decode(valid.as_bytes()).is_ok());
        for bad in [
            valid.replace("namtab.flow/0", "namtab.mk1/1"),
            valid.replace("\"id\":1", "\"id\":1,\"id\":2"),
            valid.replace("\"after\":0", "\"after\":0,\"authority\":true"),
            valid.replace("\"id\":1", "\"id\":0"),
            " ".repeat(MAX_FRAME + 1),
        ] {
            assert!(decode(bad.as_bytes()).is_err());
        }
    }
}
