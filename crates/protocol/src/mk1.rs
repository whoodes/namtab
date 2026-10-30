//! Experimental bounded hands-on contract, not the N0 proposed public schema.
use serde::{Deserialize, Serialize};
mod watch;
pub use watch::*;
pub const VERSION: &str = "namtab.mk1/1";
pub const MAX_FRAME: usize = 1_048_576;
pub const MAX_INPUT: usize = 8192;
pub const MAX_ID: u64 = 9_007_199_254_740_991;
/// Additive MK1/1 extensions; clients must observe support before using them.
pub const SHARED_TASKS: &str = "shared_tasks_v1";
pub const ATTEMPT_ORIGIN: &str = "attempt_origin_v1";
pub const ATTEMPT_CANCEL: &str = "attempt_cancel_v0";
pub const ATTEMPT_CONTROL_PROFILE: &str = "namtab.attempt-control/0";
pub const HUMAN_ADMISSION: &str = "human_admission_v0";
pub const HUMAN_SHUTDOWN: &str = "human_shutdown_v1";
pub const RECEIPT_RESULTS: &str = "receipt_results_v1";
pub const SCOPED_OBSERVE: &str = "scoped_observe_v0";
/// Separately negotiated freshness-preserving MCP-compatible work export.
pub const SCOPED_WORK_EXPORT: &str = "scoped_work_export_v0";
pub const OBSERVATION_PROFILE: &str = "namtab.scoped-observation/0";
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
    SessionWatch {
        epoch: String,
        request: WatchRequest,
    },
    ObserveScoped {
        epoch: String,
        scope: ObservationScope,
    },
    Shutdown {},
    CancelAttempt {
        epoch: String,
        attempt: String,
        origin_endpoint: String,
        origin_request_id: u64,
    },
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
#[serde(rename_all = "snake_case")]
pub enum CancellationState {
    Requested,
    ExitObserved,
    Unknown,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CancelRequestStatus {
    Requested,
    AlreadyRequested,
    AlreadyExited,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttemptCancellation {
    pub requested_by: String,
    pub request_id: u64,
    /// Requested or observed exit/unknown; never a promise that all effects stopped.
    pub status: CancellationState,
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
    #[serde(default)]
    pub cancellation: Option<AttemptCancellation>,
    #[serde(default)]
    pub exit_signal: Option<i32>,
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
    TaskDefined {
        task: String,
        revision: String,
    },
    AttemptStarted {
        attempt: String,
    },
    CancellationRequested {
        attempt: String,
        status: CancelRequestStatus,
    },
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
/// Pull selection, not a subscription, permission or semantic application query.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservationScope {
    Work {},
    WorkExport {},
    Session { session: String },
    Attempt { attempt: String },
}
impl ObservationScope {
    pub fn valid(&self) -> bool {
        match self {
            Self::Work {} | Self::WorkExport {} => true,
            Self::Session { session: id } | Self::Attempt { attempt: id } => {
                !id.is_empty() && id.len() <= 80 && !id.chars().any(char::is_control)
            }
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SessionSummary {
    pub id: String,
    pub name: String,
    pub owner: Option<String>,
    pub generation: u64,
    pub process: String,
    pub exit_code: Option<i32>,
    pub revision: u64,
    pub mode_revision: u64,
    pub input_known: bool,
}
impl From<&Session> for SessionSummary {
    fn from(s: &Session) -> Self {
        Self {
            id: s.id.clone(),
            name: s.name.clone(),
            owner: s.owner.clone(),
            generation: s.generation,
            process: s.process.clone(),
            exit_code: s.exit_code,
            revision: s.screen.revision,
            mode_revision: s.screen.mode_revision,
            input_known: s.screen.input_known,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AttemptSummary {
    pub id: String,
    pub task: String,
    pub task_revision: Option<String>,
    pub origin: Option<AttemptOrigin>,
    pub state: String,
    pub exit_code: Option<i32>,
    pub exit_signal: Option<i32>,
    pub cancellation: Option<AttemptCancellation>,
    pub source_truncated: bool,
}
impl From<&Attempt> for AttemptSummary {
    fn from(a: &Attempt) -> Self {
        Self {
            id: a.id.clone(),
            task: a.task.clone(),
            task_revision: a.task_revision.clone(),
            origin: a.origin.clone(),
            state: a.state.clone(),
            exit_code: a.exit_code,
            exit_signal: a.exit_signal,
            cancellation: a.cancellation.clone(),
            source_truncated: a.truncated,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservationView {
    WorkExport {
        capabilities: Vec<String>,
        tasks: Vec<TaskContract>,
        sessions: Vec<SessionSummary>,
        attempts: Vec<AttemptSummary>,
        artifacts: Vec<Artifact>,
    },
    Work {
        endpoints: Vec<EndpointView>,
        capabilities: Vec<String>,
        tasks: Vec<TaskContract>,
        sessions: Vec<SessionSummary>,
        attempts: Vec<AttemptSummary>,
    },
    Session {
        session: Session,
    },
    Attempt {
        attempt: Attempt,
    },
}
impl ObservationView {
    pub fn matches(&self, scope: &ObservationScope) -> bool {
        match (self, scope) {
            (Self::Work { .. }, ObservationScope::Work {}) => true,
            (Self::WorkExport { .. }, ObservationScope::WorkExport {}) => true,
            (Self::Session { session }, ObservationScope::Session { session: id }) => {
                session.id == *id
            }
            (Self::Attempt { attempt }, ObservationScope::Attempt { attempt: id }) => {
                attempt.id == *id
            }
            _ => false,
        }
    }
}
/// One lock-time projection of broker state. No global event cut, artifact
/// complete capture or atomic application/OS state is attested. Only the
/// separately negotiated WorkExport projection refreshes registered files.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ScopedObservation {
    pub version: String,
    pub profile: String,
    pub epoch: String,
    pub endpoint: String,
    pub degraded: Option<String>,
    pub view: ObservationView,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Response {
    pub id: u64,
    pub ok: bool,
    pub error: Option<String>,
    pub receipt: Option<Receipt>,
    pub snapshot: Option<Snapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation: Option<ScopedObservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watch: Option<SessionWatch>,
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
            | Self::ObserveScoped { .. }
            | Self::SessionWatch { .. }
            | Self::Receipt { .. } => return None,
            Self::Shutdown {} => "shutdown",
            Self::CancelAttempt { .. } => "cancel_attempt",
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
        Op::CancelAttempt {
            epoch,
            attempt,
            origin_endpoint,
            origin_request_id,
        } if epoch.len() != 64
            || !epoch.bytes().all(|b| b.is_ascii_hexdigit())
            || !label(attempt)
            || !label(origin_endpoint)
            || *origin_request_id == 0
            || *origin_request_id > MAX_ID =>
        {
            return Err("cancel_target");
        }
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

#[cfg(test)]
mod cancel_contract_tests {
    use super::*;
    #[test]
    fn cancel_wire_is_exact_bounded_and_rejects_extra_authority() {
        let operation = serde_json::json!({"op":"cancel_attempt","epoch":"a".repeat(64),"attempt":"attempt1","origin_endpoint":"e1","origin_request_id":2});
        let check = |op: serde_json::Value| {
            decode(
                &serde_json::to_vec(&serde_json::json!({"version":VERSION,"id":4,"operation":op}))
                    .unwrap(),
            )
        };
        assert!(check(operation.clone()).is_ok());
        for (field, value) in [
            ("epoch", serde_json::json!("bad")),
            ("attempt", serde_json::json!("x".repeat(81))),
            ("origin_endpoint", serde_json::json!("")),
            ("origin_request_id", serde_json::json!(0)),
            ("origin_request_id", serde_json::json!(MAX_ID + 1)),
            ("pid", serde_json::json!(42)),
        ] {
            let mut bad = operation.clone();
            bad[field] = value;
            assert!(check(bad).is_err(), "{field}");
        }
        let mut bad = operation.clone();
        bad.as_object_mut().unwrap().remove("origin_endpoint");
        assert!(check(bad).is_err());
        assert!(serde_json::from_str::<CancellationState>("\"unknown\"").is_ok());
        assert!(serde_json::from_str::<CancellationState>("\"complete\"").is_err());
    }
}

#[cfg(test)]
mod scoped_tests {
    use super::*;
    #[test]
    fn scopes_are_typed_nonmutating_and_legacy_response_omits_extension() {
        for scope in [
            ObservationScope::Work {},
            ObservationScope::WorkExport {},
            ObservationScope::Session {
                session: "session1".into(),
            },
            ObservationScope::Attempt {
                attempt: "attempt2".into(),
            },
        ] {
            let q = Request {
                version: VERSION.into(),
                id: 2,
                operation: Op::ObserveScoped {
                    epoch: "epoch".into(),
                    scope,
                },
            };
            assert!(q.operation.mutation_name().is_none());
            assert_eq!(decode(&serde_json::to_vec(&q).unwrap()).unwrap(), q);
        }
        let bad = br#"{"version":"namtab.mk1/1","id":2,"operation":{"op":"observe_scoped","epoch":"epoch","scope":{"kind":"work","session":"extra"}}}"#;
        assert!(decode(bad).is_err());
        assert!(!ObservationScope::Session { session: "".into() }.valid());
        assert!(
            !ObservationScope::Attempt {
                attempt: "x".repeat(81)
            }
            .valid()
        );
        let r: Response = serde_json::from_value(serde_json::json!({"id":1,"ok":true,"error":null,"receipt":null,"snapshot":null,"endpoint":null,"resume":null,"epoch":"epoch"})).unwrap();
        assert!(r.observation.is_none());
        assert!(
            !serde_json::to_value(r)
                .unwrap()
                .as_object()
                .unwrap()
                .contains_key("observation")
        );
    }
}
