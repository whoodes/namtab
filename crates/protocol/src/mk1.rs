//! Experimental bounded hands-on contract, not the N0 proposed public schema.
use serde::{Deserialize, Serialize};
pub const VERSION: &str = "namtab.mk1/1";
pub const MAX_FRAME: usize = 1_048_576;
pub const MAX_INPUT: usize = 8192;
pub const MAX_ID: u64 = 9_007_199_254_740_991;
/// Additive MK1/1 extensions; clients must observe support before using them.
pub const SHARED_TASKS: &str = "shared_tasks_v1";
pub const ATTEMPT_ORIGIN: &str = "attempt_origin_v1";
pub const HUMAN_SHUTDOWN: &str = "human_shutdown_v1";
pub const RECEIPT_RESULTS: &str = "receipt_results_v1";
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
    LifecycleHello {},
    Observe {},
    Shutdown {},
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
    DefineTask {
        task: String,
        goal: String,
        expected_artifacts: Vec<String>,
    },
    Run {
        task: String,
        argv: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        task_revision: Option<String>,
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
    /// Absent only for an older server that cannot attest launch attribution.
    #[serde(default)]
    pub origin: Option<AttemptOrigin>,
    /// None denotes an ad-hoc label, never implicit membership in a later task.
    #[serde(default)]
    pub task_revision: Option<String>,
    pub state: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub truncated: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttemptOrigin {
    pub endpoint: String,
    pub request_id: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskContract {
    pub task: String,
    pub revision: String,
    pub goal: String,
    pub expected_artifacts: Vec<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<MutationResult>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MutationResult {
    TaskDefined { task: String, revision: String },
    AttemptStarted { attempt: String },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EndpointView {
    pub id: String,
    /// A declared role on the trusted developer host, not human/model presence.
    pub role: Role,
    pub connected: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Snapshot {
    pub version: String,
    pub epoch: String,
    pub endpoint: String,
    pub endpoints: Vec<EndpointView>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub tasks: Vec<TaskContract>,
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
            Self::Hello { .. }
            | Self::LifecycleHello {}
            | Self::Observe {}
            | Self::Receipt { .. } => return None,
            Self::Shutdown {} => "shutdown",
            Self::CreateSession { .. } => "create_session",
            Self::TakeControl { .. } => "take_control",
            Self::ReleaseControl { .. } => "release_control",
            Self::Input { .. } => "input",
            Self::Resize { .. } => "resize",
            Self::DefineTask { .. } => "define_task",
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
    match &request.operation {
        Op::Hello {
            label: l, resume, ..
        } if !label(l) || resume.as_ref().is_some_and(|s| s.len() > 128) => return Err("hello"),
        Op::CreateSession { name } if !label(name) => return Err("name"),
        Op::DefineTask {
            task,
            goal,
            expected_artifacts,
        } => {
            validate_task_definition(task, goal, expected_artifacts)?;
        }
        Op::Run {
            task,
            argv,
            task_revision,
        } if !label(task)
            || argv.is_empty()
            || argv.len() > 32
            || argv.iter().any(|s| s.len() > 8192 || s.contains('\0'))
            || task_revision
                .as_ref()
                .is_some_and(|value| !task_revision_valid(value)) =>
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
fn label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value.chars().all(|c| c.is_ascii_graphic() || c == ' ')
}
fn task_revision_valid(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hash| {
        hash.len() == 64
            && hash
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    })
}
pub fn validate_task_definition(
    task: &str,
    goal: &str,
    expected_artifacts: &[String],
) -> Result<(), &'static str> {
    if !label(task)
        || goal.trim().is_empty()
        || goal.len() > 512
        || goal.chars().any(char::is_control)
    {
        return Err("task_definition");
    }
    if expected_artifacts.len() > 2
        || expected_artifacts.iter().any(|path| {
            path.len() > 512
                || path.chars().any(|c| c == '\\' || c.is_control())
                || path.split('/').any(|part| matches!(part, "" | "." | ".."))
        })
    {
        return Err("expected_artifacts");
    }
    Ok(())
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
        assert_eq!(decode(&[b'['; 17]), Err("nesting"));
        assert_eq!(decode(&vec![0; MAX_FRAME + 1]), Err("frame_size"));
    }
    fn request(operation: serde_json::Value) -> Result<Request, &'static str> {
        decode(
            &serde_json::to_vec(&serde_json::json!({
                "version": VERSION, "id": 1, "operation": operation,
            }))
            .unwrap(),
        )
    }
    #[test]
    fn shared_contract_fields_are_bounded_before_admission() {
        let definition = |goal: &str, paths: Vec<&str>| {
            serde_json::json!({
                "op": "define_task", "task": "shared task", "goal": goal, "expected_artifacts": paths,
            })
        };
        assert!(
            request(definition(
                &"é".repeat(256),
                vec![".artifacts/result", "out.txt"]
            ))
            .is_ok()
        );
        for goal in [" ".to_owned(), "a\nb".into(), "é".repeat(257)] {
            assert_eq!(request(definition(&goal, vec![])), Err("task_definition"));
        }
        for paths in [
            vec!["one", "two", "three"],
            vec![""],
            vec!["../out"],
            vec!["/out"],
            vec!["a//b"],
            vec!["./out"],
            vec!["a\\b"],
            vec!["a\0b"],
        ] {
            assert_eq!(
                request(definition("goal", paths)),
                Err("expected_artifacts")
            );
        }
        let long_path = "x".repeat(513);
        assert_eq!(
            request(definition("goal", vec![&long_path])),
            Err("expected_artifacts")
        );
        let unknown_field = serde_json::json!({
            "op": "define_task", "task": "shared", "goal": "goal", "expected_artifacts": [], "scheduler": true,
        });
        assert_eq!(request(unknown_field), Err("shape"));
    }
    #[test]
    fn additive_mk1_fields_preserve_legacy_json_and_expose_unknown_authority() {
        let legacy =
            request(serde_json::json!({"op":"run", "task":"adhoc", "argv":["/bin/true"]})).unwrap();
        assert!(matches!(
            &legacy.operation,
            Op::Run {
                task_revision: None,
                ..
            }
        ));
        assert!(
            serde_json::to_value(&legacy.operation)
                .unwrap()
                .get("task_revision")
                .is_none()
        );
        assert!(request(serde_json::json!({"op":"run", "task":"shared", "argv":["/bin/true"], "task_revision": format!("sha256:{}", "a".repeat(64))})).is_ok());
        assert_eq!(
            request(
                serde_json::json!({"op":"run", "task":"shared", "argv":["/bin/true"], "task_revision": "unknown"})
            ),
            Err("argv")
        );
        let snapshot: Snapshot = serde_json::from_value(serde_json::json!({
            "version": VERSION, "epoch":"older", "endpoint":"e1", "endpoints":[], "sessions":[], "attempts":[{
                "id":"old-attempt", "task":"label", "state":"exited", "exit_code":0, "stdout":"", "stderr":"", "truncated":false,
            }], "artifacts":[], "receipts":[{
                "endpoint":"e1", "id":1, "operation":"run", "state":"applied", "detail":"legacy", "delivered_bytes":0,
            }], "degraded":null,
        })).unwrap();
        assert!(snapshot.capabilities.is_empty());
        assert!(snapshot.tasks.is_empty());
        assert!(snapshot.attempts[0].origin.is_none());
        assert!(snapshot.attempts[0].task_revision.is_none());
        assert!(snapshot.receipts[0].result.is_none());
    }
}
