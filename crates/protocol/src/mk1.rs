//! Experimental bounded hands-on contract, not the N0 proposed public schema.
use serde::{Deserialize, Serialize};
pub const VERSION: &str = "namtab.mk1/1";
pub const MAX_FRAME: usize = 1_048_576;
pub const MAX_INPUT: usize = 8192;
pub const MAX_ID: u64 = 9_007_199_254_740_991;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Human,
    Agent,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InputPolicy {
    Observed,
    HumanLive,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Intent {
    Text {
        text: String,
    },
    Paste {
        text: String,
    },
    Key {
        key: String,
        ctrl: bool,
        alt: bool,
        shift: bool,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Op {
    Hello {
        label: String,
        role: Role,
        resume: Option<String>,
    },
    Observe {},
    CreateSession {
        name: String,
    },
    TakeControl {
        session: String,
        generation: u64,
        force: bool,
    },
    ReleaseControl {
        session: String,
        generation: u64,
    },
    Input {
        session: String,
        generation: u64,
        revision: u64,
        mode_revision: u64,
        policy: InputPolicy,
        intent: Intent,
    },
    Resize {
        session: String,
        generation: u64,
        cols: u16,
        rows: u16,
    },
    Run {
        task: String,
        argv: Vec<String>,
    },
    Artifact {
        task: String,
        path: String,
    },
    ValidateArtifact {
        artifact: String,
        revision: String,
    },
    Receipt {
        request_id: u64,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: String,
    pub id: u64,
    pub operation: Op,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Screen {
    pub cols: u16,
    pub rows: u16,
    pub lines: Vec<String>,
    pub cursor_col: u16,
    pub cursor_row: u16,
    pub cursor_visible: bool,
    pub revision: u64,
    pub mode_revision: u64,
    pub alternate: bool,
    pub input_known: bool,
    pub diagnostics: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub owner: Option<String>,
    pub generation: u64,
    pub process: String,
    pub exit_code: Option<i32>,
    pub screen: Screen,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Attempt {
    pub id: String,
    pub task: String,
    pub state: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub truncated: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Artifact {
    pub id: String,
    pub task: String,
    pub path: String,
    pub revision: String,
    pub validated_revision: Option<String>,
    pub stale: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Receipt {
    pub endpoint: String,
    pub id: u64,
    pub operation: String,
    pub state: String,
    pub detail: String,
    pub delivered_bytes: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Snapshot {
    pub version: String,
    pub epoch: String,
    pub endpoint: String,
    pub sessions: Vec<Session>,
    pub attempts: Vec<Attempt>,
    pub artifacts: Vec<Artifact>,
    pub receipts: Vec<Receipt>,
    pub degraded: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Response {
    pub id: u64,
    pub ok: bool,
    pub error: Option<String>,
    pub receipt: Option<Receipt>,
    pub snapshot: Option<Snapshot>,
    pub endpoint: Option<String>,
    pub resume: Option<String>,
    pub epoch: String,
}
impl Op {
    pub fn mutation_name(&self) -> Option<&'static str> {
        Some(match self {
            Self::Hello { .. } | Self::Observe {} | Self::Receipt { .. } => return None,
            Self::CreateSession { .. } => "create_session",
            Self::TakeControl { .. } => "take_control",
            Self::ReleaseControl { .. } => "release_control",
            Self::Input { .. } => "input",
            Self::Resize { .. } => "resize",
            Self::Run { .. } => "run",
            Self::Artifact { .. } => "artifact",
            Self::ValidateArtifact { .. } => "validate_artifact",
        })
    }
}
/// A byte cap and lexical nesting/string preflight precede bounded typed serde parsing.
/// This is not an adversarial production codec: the whole capped frame is resident.
pub fn decode(bytes: &[u8]) -> Result<Request, &'static str> {
    if bytes.is_empty() || bytes.len() > MAX_FRAME {
        return Err("frame_size");
    }
    let (mut quoted, mut escaped, mut depth, mut string_len) = (false, false, 0_u16, 0_usize);
    for &b in bytes {
        if quoted {
            string_len += 1;
            if string_len > 65_536 {
                return Err("string_size");
            }
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                quoted = false;
            }
        } else {
            match b {
                b'"' => {
                    quoted = true;
                    string_len = 0;
                }
                b'[' | b'{' => {
                    depth += 1;
                    if depth > 16 {
                        return Err("nesting");
                    }
                }
                b']' | b'}' => {
                    depth = depth.checked_sub(1).ok_or("shape")?;
                }
                _ => {}
            }
        }
    }
    if quoted || depth != 0 {
        return Err("shape");
    }
    let request: Request = serde_json::from_slice(bytes).map_err(|_| "shape")?;
    if request.version != VERSION {
        return Err("version");
    }
    if request.id == 0 || request.id > MAX_ID {
        return Err("request_id");
    }
    let label = |s: &str| {
        !s.is_empty() && s.len() <= 80 && s.chars().all(|c| c.is_ascii_graphic() || c == ' ')
    };
    match &request.operation {
        Op::Hello {
            label: l, resume, ..
        } if !label(l) || resume.as_ref().is_some_and(|s| s.len() > 128) => return Err("hello"),
        Op::CreateSession { name } if !label(name) => return Err("name"),
        Op::Run { task, argv }
            if !label(task)
                || argv.is_empty()
                || argv.len() > 32
                || argv.iter().any(|s| s.len() > 8192 || s.contains('\0')) =>
        {
            return Err("argv");
        }
        Op::Input {
            intent: Intent::Text { text } | Intent::Paste { text },
            ..
        } if text.is_empty() || text.len() > MAX_INPUT => return Err("input_size"),
        Op::Input {
            intent: Intent::Key { key, .. },
            ..
        } if key.is_empty() || key.len() > 32 => return Err("key"),
        Op::Artifact { task, path }
            if !label(task) || path.is_empty() || path.len() > 4096 || path.contains('\0') =>
        {
            return Err("artifact");
        }
        _ => {}
    }
    Ok(request)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hello_roundtrip_unknown_and_duplicate_fields() {
        let q=br#"{"version":"namtab.mk1/1","id":1,"operation":{"op":"hello","label":"test","role":"human","resume":null}}"#;
        assert!(decode(q).is_ok());
        for bad in [
            br#"{"version":"namtab.mk1/1","id":1,"id":2,"operation":{"op":"observe"}}"#.as_slice(),
            br#"{"version":"namtab.mk1/1","id":1,"operation":{"op":"observe","extra":true}}"#,
        ] {
            assert!(decode(bad).is_err());
        }
    }
    #[test]
    fn incompatible_ids_and_resource_bounds() {
        for id in [0, MAX_ID + 1] {
            let q =
                format!(r#"{{"version":"namtab.mk1/1","id":{id},"operation":{{"op":"observe"}}}}"#);
            assert!(decode(q.as_bytes()).is_err());
        }
        assert_eq!(decode(&vec![b'['; 17]), Err("nesting"));
        assert_eq!(decode(&vec![0; MAX_FRAME + 1]), Err("frame_size"));
    }
}
