//! MK1 is a same-UID supervised developer host, not a sandbox or durable terminal.
mod admission;
mod attempts;
mod journal;
mod observations;
mod server;
mod sessions;
mod tasks;
#[cfg(test)]
mod tests;
mod watch;
mod worker;

use project_x_platform::local_process::{
    self as platform, CommandSpec, Geometry, ProcessHandle, PtyWriter, ReadOutcome,
};
use project_x_protocol::mk1::{
    self as wire, Artifact, Attempt, InputPolicy, Intent, MutationResult, Op, Receipt, Request,
    Response, Role, Screen, Session, Snapshot, TaskContract,
};
use project_x_terminal::{
    Frame, Input, Key, KeyPhase, Modifiers,
    rpc::{Operation, Reply},
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

pub use server::serve;
pub struct Config {
    pub socket: PathBuf,
    pub workspace: PathBuf,
    pub worker: PathBuf,
    pub pty_helper: PathBuf,
    pub terminfo: Option<PathBuf>,
}
impl Config {
    pub fn from_args() -> Result<Self, Box<dyn std::error::Error>> {
        let mut args = std::env::args_os().skip(1);
        let mut values = BTreeMap::new();
        while let Some(key) = args.next() {
            let value = args.next().ok_or("each argument needs a path")?;
            if values.insert(key, PathBuf::from(value)).is_some() {
                return Err("duplicate argument".into());
            }
        }
        let terminfo = values
            .remove(&OsString::from("--terminfo"))
            .map(|path| path.canonicalize())
            .transpose()?;
        let mut take = |name: &str| {
            values
                .remove(&OsString::from(name))
                .ok_or("required: --socket --workspace --worker --pty-helper")
        };
        let config = Self {
            terminfo,
            socket: take("--socket")?,
            workspace: take("--workspace")?.canonicalize()?,
            worker: take("--worker")?.canonicalize()?,
            pty_helper: take("--pty-helper")?.canonicalize()?,
        };
        if !values.is_empty() || !config.socket.is_absolute() || !config.workspace.is_dir() {
            return Err("invalid paths or extra arguments".into());
        }
        Ok(config)
    }
}

fn token() -> io::Result<String> {
    let mut bytes = [0_u8; 32];
    File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn screen(frame: Frame) -> Screen {
    let lines = frame
        .cells
        .chunks(usize::from(frame.cols))
        .map(|row| row.iter().map(|cell| cell.text.as_str()).collect())
        .collect();
    let mut diagnostics = Vec::new();
    if frame.diagnostics.unsupported_sequences != 0 {
        diagnostics.push("unsupported_output_consumed".into());
    }
    if frame.diagnostics.unsupported_text != 0 {
        diagnostics.push("outside_finite_repertoire_replaced".into());
    }
    if frame.diagnostics.malformed_sequences != 0 || frame.diagnostics.invalid_utf8 != 0 {
        diagnostics.push("malformed_output_replaced_or_ignored".into());
    }
    Screen {
        cols: frame.cols,
        rows: frame.rows,
        lines,
        cursor_col: frame.cursor_col,
        cursor_row: frame.cursor_row,
        cursor_visible: frame.cursor_visible,
        revision: frame.revision,
        mode_revision: frame.mode_revision,
        alternate: frame.alternate,
        input_known: frame.interpretation_known
            && frame.modes.input_protocol_known
            && !frame.closed,
        diagnostics,
    }
}
fn input(intent: &Intent) -> Result<Input, String> {
    Ok(match intent {
        Intent::Text { text } => Input::Text { text: text.clone() },
        Intent::Paste { text } => Input::Paste { text: text.clone() },
        Intent::Key {
            key,
            ctrl,
            alt,
            shift,
        } => {
            let scalar = {
                let mut chars = key.chars();
                chars.next().filter(|_| chars.next().is_none())
            };
            let key = if let Some(character) = scalar {
                Key::Character(character)
            } else {
                serde_json::from_value(serde_json::Value::String(key.clone()))
                    .map_err(|_| "unsupported_key")?
            };
            Input::Key {
                key,
                modifiers: Modifiers {
                    ctrl: *ctrl,
                    alt: *alt,
                    shift: *shift,
                },
                phase: KeyPhase::Press,
            }
        }
    })
}
struct Decision {
    binding: [u8; 32],
    receipt: Receipt,
}
struct Endpoint {
    role: Role,
    resume: String,
    connected: bool,
    high_water: u64,
    decisions: BTreeMap<u64, Decision>,
    emergency: BTreeMap<u64, Decision>,
}
struct LiveSession {
    view: Session,
    writer: PtyWriter,
    pending_output: usize,
    exited: bool,
    input_uncertain: bool,
}
impl LiveSession {
    fn publish(&mut self, frame: Frame) {
        if frame.revision >= self.view.screen.revision {
            let mut projection = screen(frame);
            projection.input_known &= !self.input_uncertain;
            if self.input_uncertain {
                projection
                    .diagnostics
                    .push("delivery_or_continuity_unknown_new_session_required".into());
            }
            self.view.screen = projection;
        }
    }
}
struct LiveAttempt {
    view: Attempt,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    stdout_eof: bool,
    stderr_eof: bool,
    exited: bool,
    cancel: Arc<AtomicBool>,
}
struct State {
    endpoints: BTreeMap<String, Endpoint>,
    sessions: BTreeMap<String, LiveSession>,
    attempts: BTreeMap<String, LiveAttempt>,
    tasks: BTreeMap<String, TaskContract>,
    artifacts: BTreeMap<String, Artifact>,
    reserved_sessions: usize,
    reserved_attempts: usize,
    sequence: u64,
    system_sequence: u64,
    journal: journal::Journal,
    degraded: Option<String>,
    shutdown_receipt: Option<Receipt>,
}
pub struct Broker {
    state: Mutex<State>,
    worker: worker::Worker,
    config: Config,
    epoch: String,
    stopping: AtomicBool,
    active_mutations: AtomicUsize,
    active_children: Arc<AtomicUsize>,
    shutdown_requested: AtomicBool,
    changed: Condvar,
    #[cfg(test)]
    human_waiters: std::sync::atomic::AtomicUsize,
}
// Counts only still-owned direct process handles, not descendant containment.
struct Active<'a>(&'a AtomicUsize);
impl<'a> Active<'a> {
    fn new(counter: &'a AtomicUsize) -> Self {
        counter.fetch_add(1, Ordering::SeqCst);
        Self(counter)
    }
}
impl Drop for Active<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
struct ChildActive(Arc<AtomicUsize>);
impl ChildActive {
    fn new(counter: &Arc<AtomicUsize>) -> Self {
        counter.fetch_add(1, Ordering::SeqCst);
        Self(Arc::clone(counter))
    }
}
impl Drop for ChildActive {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
impl Broker {
    fn new(config: Config, journal_path: &Path) -> io::Result<Arc<Self>> {
        let epoch = token()?;
        let journal =
            journal::Journal::open(journal_path, epoch.clone(), platform::effective_uid())?;
        let worker = worker::Worker::start(&config.worker, &config.workspace)?;
        Ok(Arc::new(Self {
            state: Mutex::new(State {
                endpoints: BTreeMap::new(),
                sessions: BTreeMap::new(),
                attempts: BTreeMap::new(),
                tasks: BTreeMap::new(),
                artifacts: BTreeMap::new(),
                reserved_sessions: 0,
                reserved_attempts: 0,
                sequence: 0,
                system_sequence: 0,
                journal,
                degraded: None,
                shutdown_receipt: None,
            }),
            worker,
            config,
            epoch,
            stopping: AtomicBool::new(false),
            active_mutations: AtomicUsize::new(0),
            active_children: Arc::new(AtomicUsize::new(0)),
            shutdown_requested: AtomicBool::new(false),
            changed: Condvar::new(),
            #[cfg(test)]
            human_waiters: std::sync::atomic::AtomicUsize::new(0),
        }))
    }
    fn shutdown(&self) {
        self.stopping.store(true, Ordering::Release);
        self.worker.stop();
    }
    fn response(&self, id: u64) -> Response {
        Response {
            id,
            ok: true,
            error: None,
            receipt: None,
            snapshot: None,
            observation: None,
            watch: None,
            endpoint: None,
            resume: None,
            epoch: self.epoch.clone(),
        }
    }
    fn fail(&self, id: u64, error: impl Into<String>) -> Response {
        let mut response = self.response(id);
        response.ok = false;
        response.error = Some(error.into());
        response
    }
    fn hello(&self, id: u64, role: Role, resume: Option<String>) -> (Response, Option<String>) {
        let mut state = self.state.lock().unwrap();
        self.bind_hello(&mut state, id, role, resume, None, || true)
    }
    fn control_state(&self) -> Option<std::sync::MutexGuard<'_, State>> {
        let deadline = std::time::Instant::now() + admission::STATE_TIME;
        loop {
            if std::time::Instant::now() >= deadline {
                return None;
            }
            match self.state.try_lock() {
                Ok(state) => return Some(state),
                Err(std::sync::TryLockError::Poisoned(_)) => return None,
                Err(std::sync::TryLockError::WouldBlock) => {
                    std::thread::sleep(Duration::from_millis(1))
                }
            }
        }
    }
    fn control_hello(
        &self,
        id: u64,
        resume: Option<String>,
        secret: Option<String>,
        permit: impl FnOnce() -> bool,
    ) -> (Response, Option<String>) {
        if resume.is_none() && secret.is_none() {
            return (self.fail(id, "entropy_unavailable"), None);
        }
        let Some(mut state) = self.control_state() else {
            return (self.fail(id, "control_admission_state_busy"), None);
        };
        self.bind_hello(&mut state, id, Role::Human, resume, secret, permit)
    }
    fn bind_hello(
        &self,
        state: &mut State,
        id: u64,
        role: Role,
        resume: Option<String>,
        secret: Option<String>,
        permit: impl FnOnce() -> bool,
    ) -> (Response, Option<String>) {
        let (endpoint, new) = if let Some(resume) = resume {
            let Some(endpoint) = state
                .endpoints
                .iter()
                .find(|(_, e)| !e.resume.is_empty() && e.resume == resume && e.role == role)
                .map(|(id, _)| id.clone())
            else {
                return (self.fail(id, "resume_unknown_in_this_epoch"), None);
            };
            if state.endpoints[&endpoint].connected {
                return (self.fail(id, "endpoint_already_connected"), None);
            }
            (endpoint, None)
        } else {
            let count = state
                .endpoints
                .keys()
                .filter(|id| id.as_str() != "lifecycle")
                .count();
            if count >= 15 {
                return (self.fail(id, "endpoint_capacity"), None);
            }
            let resume = match secret {
                Some(s) => s,
                None => {
                    let Ok(s) = token() else {
                        return (self.fail(id, "entropy_unavailable"), None);
                    };
                    s
                }
            };
            (
                format!("e{}", count + 1),
                Some(Endpoint {
                    role,
                    resume,
                    connected: false,
                    high_water: 0,
                    decisions: BTreeMap::new(),
                    emergency: BTreeMap::new(),
                }),
            )
        };
        if !permit() {
            return (
                self.fail(id, "control_admission_no_eligible_transport"),
                None,
            );
        }
        if let Some(new) = new {
            state.endpoints.insert(endpoint.clone(), new);
        }
        let value = state.endpoints.get_mut(&endpoint).unwrap();
        value.connected = true;
        let mut r = self.response(id);
        r.endpoint = Some(endpoint.clone());
        r.resume = Some(value.resume.clone());
        (r, Some(endpoint))
    }
    fn lifecycle_hello(&self, id: u64) -> (Response, Option<String>) {
        let mut state = self.state.lock().unwrap();
        self.bind_lifecycle(&mut state, id, || true)
    }
    fn control_lifecycle_hello(
        &self,
        id: u64,
        permit: impl FnOnce() -> bool,
    ) -> (Response, Option<String>) {
        let Some(mut state) = self.control_state() else {
            return (self.fail(id, "control_admission_state_busy"), None);
        };
        self.bind_lifecycle(&mut state, id, permit)
    }
    fn bind_lifecycle(
        &self,
        state: &mut State,
        id: u64,
        permit: impl FnOnce() -> bool,
    ) -> (Response, Option<String>) {
        if state
            .endpoints
            .get("lifecycle")
            .is_some_and(|e| e.connected)
        {
            return (self.fail(id, "lifecycle_already_connected"), None);
        }
        if !permit() {
            return (
                self.fail(id, "control_admission_no_eligible_transport"),
                None,
            );
        }
        let value = state
            .endpoints
            .entry("lifecycle".into())
            .or_insert_with(|| Endpoint {
                role: Role::Human,
                resume: String::new(),
                connected: false,
                high_water: 0,
                decisions: BTreeMap::new(),
                emergency: BTreeMap::new(),
            });
        value.connected = true;
        let mut r = self.response(id);
        r.endpoint = Some("lifecycle".into());
        (r, Some("lifecycle".into()))
    }
    fn protective_shutdown(&self, endpoint: &str, id: u64) -> Response {
        let mut state = self.state.lock().unwrap();
        let Some(principal) = state.endpoints.get(endpoint) else {
            return self.fail(id, "endpoint");
        };
        if principal.role != Role::Human {
            return self.fail(id, "shutdown_requires_human_endpoint");
        }
        if let Some(r) = &state.shutdown_receipt {
            if r.endpoint == endpoint && r.id == id {
                let mut response = self.response(id);
                response.ok = r.state == "applied";
                response.receipt = Some(r.clone());
                return response;
            }
            return self.fail(id, "shutdown_already_requested");
        }
        if id <= principal.high_water
            || principal.decisions.contains_key(&id)
            || principal.emergency.contains_key(&id)
        {
            return self.fail(id, "mutation_id_conflict");
        }
        // Safety-only authority decrease survives worker/journal/receipt exhaustion.
        // It creates no writer grant and makes no durable completion promise.
        for session in state.sessions.values_mut() {
            session.view.owner = None;
            session.view.generation = session.view.generation.saturating_add(1);
        }
        state.degraded = Some("shutdown_requested_no_new_effects".into());
        let durable = state
            .journal
            .record(endpoint, id, "shutdown", "applied", 0)
            .is_ok();
        let receipt = Receipt {
            endpoint: endpoint.into(),
            id,
            operation: "shutdown".into(),
            state: if durable { "applied" } else { "unknown" }.into(),
            detail: if durable {
                "shutdown_requested_not_process_completion_descendants_may_continue"
            } else {
                "protective_shutdown_requested_receipt_not_durable"
            }
            .into(),
            delivered_bytes: 0,
            result: None,
        };
        state.endpoints.get_mut(endpoint).unwrap().high_water = id;
        state.shutdown_receipt = Some(receipt.clone());
        self.shutdown_requested.store(true, Ordering::SeqCst);
        self.changed.notify_all();
        let mut r = self.response(id);
        r.ok = durable;
        r.receipt = Some(receipt);
        r
    }
    fn owns_shutdown(&self, endpoint: &str, id: u64) -> bool {
        self.state
            .lock()
            .unwrap()
            .shutdown_receipt
            .as_ref()
            .is_some_and(|r| r.endpoint == endpoint && r.id == id)
    }
    fn disconnect(&self, endpoint: &str) {
        if let Some(value) = self.state.lock().unwrap().endpoints.get_mut(endpoint) {
            value.connected = false;
        }
    }
    fn worker_call(&self, operation: Operation) -> Result<Reply, String> {
        let result = self.worker.call(operation);
        if self.worker.failed() {
            self.fence_worker();
            return Err("worker_unavailable".into());
        }
        result
    }
    fn fence_worker(&self) {
        let mut state = self.state.lock().unwrap();
        state.degraded = Some("worker_unavailable_all_interpretations_unknown_no_replay".into());
        for session in state.sessions.values_mut() {
            session.view.screen.input_known = false;
            session.input_uncertain = true;
            session.view.owner = None;
            session.view.generation += 1;
        }
        self.changed.notify_all();
        for endpoint in state.endpoints.values_mut() {
            for decision in endpoint.decisions.values_mut() {
                if decision.receipt.state == "reserved" {
                    decision.receipt.state = "unknown".into();
                    decision.receipt.detail = "worker_or_effect_continuity_lost_no_retry".into();
                }
            }
        }
    }
    fn reserve(&self, endpoint: &str, request: &Request) -> Result<Option<Receipt>, String> {
        let operation = request.operation.mutation_name().ok_or("not_mutation")?;
        let binding = serde_json::to_vec(&request.operation).map_err(|_| "binding")?;
        if binding.len() > 65_536 {
            return Err("mutation_body_capacity".into());
        }
        let binding: [u8; 32] = Sha256::digest(&binding).into();
        let mut state = self.state.lock().unwrap();
        let principal = state.endpoints.get(endpoint).ok_or("endpoint")?;
        if let Some(decision) = principal
            .decisions
            .get(&request.id)
            .or_else(|| principal.emergency.get(&request.id))
        {
            return if decision.binding == binding {
                Ok(Some(decision.receipt.clone()))
            } else {
                Err("mutation_id_conflict".into())
            };
        }
        if state.degraded.is_some() || self.worker.failed() {
            return Err("broker_degraded_new_mutations_fenced".into());
        }
        if principal.decisions.len() >= 4096 {
            state.degraded =
                Some("receipt_capacity_all_new_effects_fenced_restart_required".into());
            return Err("receipt_capacity_no_eviction".into());
        }
        if request.id <= principal.high_water {
            return Err("mutation_id_not_increasing".into());
        }
        if state
            .journal
            .record(endpoint, request.id, operation, "reserved", 0)
            .is_err()
        {
            state.degraded = Some("journal_unavailable_mutations_fenced".into());
            return Err("journal_reservation_failed_no_effect".into());
        }
        let receipt = Receipt {
            endpoint: endpoint.into(),
            id: request.id,
            operation: operation.into(),
            state: "reserved".into(),
            detail: "reserved_before_effect".into(),
            delivered_bytes: 0,
            result: None,
        };
        let principal = state.endpoints.get_mut(endpoint).unwrap();
        principal.high_water = request.id;
        principal
            .decisions
            .insert(request.id, Decision { binding, receipt });
        Ok(None)
    }
    fn finish(
        &self,
        endpoint: &str,
        id: u64,
        outcome: Result<(String, usize, String), String>,
    ) -> Response {
        let mut state = self.state.lock().unwrap();
        let mut receipt = state.endpoints[endpoint].decisions[&id].receipt.clone();
        let mut response = self.response(id);
        if receipt.state == "unknown" {
            response.ok = false;
            response.error = Some("effect_continuity_unknown_no_retry".into());
        } else {
            match outcome {
                Ok((status, delivered, detail)) => {
                    receipt.state = status;
                    receipt.delivered_bytes = delivered;
                    receipt.detail = detail;
                }
                Err(error) => {
                    receipt.state = "rejected".into();
                    receipt.detail = error.clone();
                    response.ok = false;
                    response.error = Some(error);
                }
            }
        }
        if state
            .journal
            .record(
                endpoint,
                id,
                &receipt.operation,
                &receipt.state,
                receipt.delivered_bytes,
            )
            .is_err()
        {
            state.degraded = Some("journal_completion_unavailable_effect_may_have_occurred".into());
            receipt.state = "unknown".into();
            receipt.detail = "completion_persistence_unknown_no_retry".into();
            response.ok = false;
            response.error = Some(receipt.detail.clone());
        }
        state
            .endpoints
            .get_mut(endpoint)
            .unwrap()
            .decisions
            .get_mut(&id)
            .unwrap()
            .receipt = receipt.clone();
        if receipt.state != "applied" {
            response.ok = false;
            response.error.get_or_insert_with(|| receipt.detail.clone());
        }
        response.receipt = Some(receipt);
        response
    }
    pub fn request(self: &Arc<Self>, endpoint: &str, request: Request) -> Response {
        if matches!(request.operation, Op::SessionWatch { .. }) {
            return self.fail(request.id, "watch_requires_transport");
        }
        if let Op::ObserveScoped { epoch, scope } = &request.operation {
            return match self.scoped_observation(endpoint, epoch, scope) {
                Ok(observation) => {
                    let mut response = self.response(request.id);
                    response.observation = Some(observation);
                    response
                }
                Err(error) => self.fail(request.id, error),
            };
        }
        if let Op::Observe {} = request.operation {
            return match self.snapshot(endpoint) {
                Ok(snapshot) => {
                    let mut response = self.response(request.id);
                    response.snapshot = Some(snapshot);
                    response
                }
                Err(error) => self.fail(request.id, error),
            };
        }
        if let Op::Receipt { request_id } = request.operation {
            let state = self.state.lock().unwrap();
            if let Some(r) = &state.shutdown_receipt
                && r.endpoint == endpoint
                && r.id == request_id
            {
                let mut response = self.response(request.id);
                response.receipt = Some(r.clone());
                return response;
            }
            let Some(decision) = state.endpoints[endpoint]
                .decisions
                .get(&request_id)
                .or_else(|| state.endpoints[endpoint].emergency.get(&request_id))
            else {
                return self.fail(request.id, "receipt_unavailable");
            };
            let mut response = self.response(request.id);
            response.receipt = Some(decision.receipt.clone());
            return response;
        }
        if matches!(request.operation, Op::Hello { .. } | Op::LifecycleHello {}) {
            return self.fail(request.id, "hello_only_before_endpoint_binding");
        }
        let _active = Active::new(&self.active_mutations);
        if self.stopping.load(Ordering::SeqCst) {
            return self.fail(request.id, "shutdown_in_progress");
        }
        if matches!(request.operation, Op::Shutdown {}) {
            return self.protective_shutdown(endpoint, request.id);
        }
        if endpoint == "lifecycle" {
            return self.fail(request.id, "lifecycle_has_no_work_or_input_authority");
        }
        match self.reserve(endpoint, &request) {
            Ok(Some(receipt)) => {
                let mut response = self.response(request.id);
                response.ok = matches!(receipt.state.as_str(), "applied" | "reserved");
                response.receipt = Some(receipt);
                return response;
            }
            Err(error) => {
                return self
                    .emergency_fence(endpoint, &request, &error)
                    .unwrap_or_else(|| self.fail(request.id, error));
            }
            Ok(None) => {}
        }
        let outcome = self.apply(endpoint, request.id, &request.operation);
        self.finish(endpoint, request.id, outcome)
    }
    /// A safety-only authority decrease may survive failed grant/storage admission.
    /// It never writes application bytes or grants the requesting endpoint a lease.
    fn emergency_fence(
        &self,
        endpoint: &str,
        request: &Request,
        admission_error: &str,
    ) -> Option<Response> {
        if !matches!(
            admission_error,
            "receipt_capacity_no_eviction"
                | "broker_degraded_new_mutations_fenced"
                | "journal_reservation_failed_no_effect"
        ) {
            return None;
        }
        let mut state = self.state.lock().unwrap();
        let principal = state.endpoints.get(endpoint)?;
        if request.id <= principal.high_water || !principal.emergency.is_empty() {
            return None;
        }
        let (session, generation) = match &request.operation {
            Op::TakeControl {
                session,
                generation,
                force: true,
            } if principal.role == Role::Human => (session, *generation),
            Op::ReleaseControl {
                session,
                generation,
            } => (session, *generation),
            _ => return None,
        };
        let value = state.sessions.get_mut(session)?;
        if value.view.generation != generation {
            return None;
        }
        if matches!(request.operation, Op::ReleaseControl { .. })
            && value.view.owner.as_deref() != Some(endpoint)
        {
            return None;
        }
        let body = serde_json::to_vec(&request.operation).ok()?;
        let binding = Sha256::digest(&body).into();
        value.view.generation = value.view.generation.checked_add(1)?;
        value.view.owner = None;
        self.changed.notify_all();
        let receipt = Receipt {
            endpoint: endpoint.into(),
            id: request.id,
            operation: request.operation.mutation_name()?.into(),
            state: "unknown".into(),
            detail: "protective_fence_applied_new_writer_not_granted_durable_receipt_unavailable"
                .into(),
            delivered_bytes: 0,
            result: None,
        };
        let principal = state.endpoints.get_mut(endpoint)?;
        principal.high_water = request.id;
        principal.emergency.insert(
            request.id,
            Decision {
                binding,
                receipt: receipt.clone(),
            },
        );
        let mut response = self.fail(request.id, receipt.detail.clone());
        response.receipt = Some(receipt);
        Some(response)
    }
    fn apply(
        self: &Arc<Self>,
        endpoint: &str,
        request_id: u64,
        operation: &Op,
    ) -> Result<(String, usize, String), String> {
        match operation {
            Op::CancelAttempt {
                epoch,
                attempt,
                origin_endpoint,
                origin_request_id,
            } => self.cancel_attempt(
                endpoint,
                request_id,
                epoch,
                attempt,
                origin_endpoint,
                *origin_request_id,
            ),
            Op::CreateSession { name } => self.create_session(name),
            Op::DefineTask {
                task,
                goal,
                expected_artifacts,
            } => self.define_task(endpoint, request_id, task, goal, expected_artifacts),
            Op::Run {
                task,
                argv,
                task_revision,
            } => self.run(endpoint, request_id, task, task_revision.as_deref(), argv),
            Op::TakeControl {
                session,
                generation,
                force,
            } => {
                let mut state = self.state.lock().unwrap();
                let human = state.endpoints[endpoint].role == Role::Human;
                if state.degraded.is_some() || self.shutdown_requested.load(Ordering::SeqCst) {
                    return Err("grant_fenced_by_degraded_or_shutdown".into());
                }
                let value = state.sessions.get_mut(session).ok_or("unknown_session")?;
                if value.view.generation != *generation {
                    return Err("stale_generation".into());
                }
                if !value.view.screen.input_known {
                    return Err("interpretation_unknown".into());
                }
                if *force && !human {
                    return Err("agent_cannot_force_takeover".into());
                }
                if value
                    .view
                    .owner
                    .as_deref()
                    .is_some_and(|owner| owner != endpoint)
                    && !(*force && human)
                {
                    return Err("writer_owned_use_human_takeover".into());
                }
                value.view.generation = value
                    .view
                    .generation
                    .checked_add(1)
                    .ok_or("generation_exhausted")?;
                value.view.owner = Some(endpoint.into());
                self.changed.notify_all();
                Ok((
                    "applied".into(),
                    0,
                    "writer_changed_future_old_generation_input_fenced_not_process_pause".into(),
                ))
            }
            Op::ReleaseControl {
                session,
                generation,
            } => {
                let mut state = self.state.lock().unwrap();
                let value = state.sessions.get_mut(session).ok_or("unknown_session")?;
                Self::owner(value, endpoint, *generation)?;
                value.view.owner = None;
                value.view.generation += 1;
                self.changed.notify_all();
                Ok((
                    "applied".into(),
                    0,
                    "writer_released_process_continues".into(),
                ))
            }
            Op::Input { .. } => self.deliver(endpoint, operation),
            Op::Resize {
                session,
                generation,
                cols,
                rows,
            } => self.resize(endpoint, session, *generation, *cols, *rows),
            Op::Artifact { task, path } => {
                let (path, revision) = self.file_revision(path)?;
                let mut state = self.state.lock().unwrap();
                if state.artifacts.len() >= 16 {
                    return Err("artifact_capacity".into());
                }
                state.sequence += 1;
                let id = format!("artifact{}", state.sequence);
                state.artifacts.insert(
                    id.clone(),
                    Artifact {
                        id,
                        task: task.clone(),
                        path,
                        revision,
                        validated_revision: None,
                        stale: false,
                    },
                );
                Ok((
                    "applied".into(),
                    0,
                    "bounded_file_observed_not_attempt_causality_or_sealed_tree".into(),
                ))
            }
            Op::ValidateArtifact { artifact, revision } => {
                if self.state.lock().unwrap().endpoints[endpoint].role != Role::Human {
                    return Err("artifact_review_requires_human_endpoint".into());
                }
                let path = self
                    .state
                    .lock()
                    .unwrap()
                    .artifacts
                    .get(artifact)
                    .ok_or("unknown_artifact")?
                    .path
                    .clone();
                let (_, current) = self.file_revision(&path)?;
                let mut state = self.state.lock().unwrap();
                let value = state
                    .artifacts
                    .get_mut(artifact)
                    .ok_or("unknown_artifact")?;
                value.revision = current.clone();
                value.stale = value
                    .validated_revision
                    .as_ref()
                    .is_some_and(|validated| validated != &current);
                if &current != revision {
                    return Err("artifact_revision_stale".into());
                }
                value.validated_revision = Some(current);
                value.stale = false;
                Ok((
                    "applied".into(),
                    0,
                    "human_review_bound_to_current_observed_file_not_correctness_proof".into(),
                ))
            }
            _ => Err("not_mutation".into()),
        }
    }
    fn file_revision(&self, path: &str) -> Result<(String, String), String> {
        let supplied = Path::new(path);
        let path = if supplied.is_absolute() {
            supplied.to_owned()
        } else {
            self.config.workspace.join(supplied)
        };
        let canonical = path.canonicalize().map_err(|_| "artifact_unavailable")?;
        if !canonical.starts_with(&self.config.workspace) || !canonical.is_file() {
            return Err("artifact_outside_workspace_or_not_file".into());
        }
        let file = File::open(&canonical).map_err(|_| "artifact_unavailable")?;
        // Check the opened identity before reading any bytes; path canonicalization
        // alone can race a symlink replacement even on a trusted developer host.
        use std::os::fd::AsRawFd;
        let opened_path = std::fs::read_link(format!("/proc/self/fd/{}", file.as_raw_fd()))
            .map_err(|_| "artifact_identity_unavailable")?;
        if opened_path != canonical || !opened_path.starts_with(&self.config.workspace) {
            return Err("artifact_path_changed_before_read".into());
        }
        if file.metadata().map_err(|_| "artifact_metadata")?.len() > 1_048_576 {
            return Err("artifact_size".into());
        }
        let mut bytes = Vec::new();
        file.take(1_048_577)
            .read_to_end(&mut bytes)
            .map_err(|_| "artifact_read")?;
        if bytes.len() > 1_048_576 {
            return Err("artifact_size".into());
        }
        let revision = format!("sha256:{:x}", Sha256::digest(&bytes));
        Ok((
            canonical
                .strip_prefix(&self.config.workspace)
                .map_err(|_| "artifact_path")?
                .to_string_lossy()
                .into_owned(),
            revision,
        ))
    }
    // Shared by legacy Observe and the explicit work-export read. File reads
    // occur outside the state mutex; this is not a sealed filesystem snapshot.
    fn refresh_artifacts(&self) {
        let artifacts = self
            .state
            .lock()
            .unwrap()
            .artifacts
            .values()
            .map(|value| (value.id.clone(), value.path.clone()))
            .collect::<Vec<_>>();
        for (id, path) in artifacts {
            let revision = self
                .file_revision(&path)
                .map(|(_, revision)| revision)
                .unwrap_or_else(|_| "unavailable".into());
            let mut state = self.state.lock().unwrap();
            let value = state.artifacts.get_mut(&id).unwrap();
            value.revision = revision.clone();
            value.stale = revision == "unavailable"
                || value
                    .validated_revision
                    .as_ref()
                    .is_some_and(|validated| validated != &revision);
        }
    }
    fn snapshot(&self, endpoint: &str) -> Result<Snapshot, String> {
        self.refresh_artifacts();
        let state = self.state.lock().unwrap();
        let attempts = state
            .attempts
            .values()
            .map(|value| {
                let mut view = value.view.clone();
                let (stdout, truncated_out) = safe_output(&value.stdout);
                let (stderr, truncated_err) = safe_output(&value.stderr);
                view.stdout = stdout;
                view.stderr = stderr;
                view.truncated |= truncated_out || truncated_err;
                view
            })
            .collect();
        let mut receipts = state.endpoints[endpoint]
            .decisions
            .values()
            .chain(state.endpoints[endpoint].emergency.values())
            .map(|value| value.receipt.clone())
            .collect::<Vec<_>>();
        receipts.sort_by_key(|receipt| std::cmp::Reverse(receipt.id));
        receipts.truncate(16);
        Ok(Snapshot {
            version: wire::VERSION.into(),
            epoch: self.epoch.clone(),
            endpoint: endpoint.into(),
            endpoints: state
                .endpoints
                .iter()
                .map(|(id, value)| wire::EndpointView {
                    id: id.clone(),
                    role: value.role.clone(),
                    connected: value.connected,
                })
                .collect(),
            capabilities: observations::capabilities(),
            tasks: state.tasks.values().cloned().collect(),
            sessions: state
                .sessions
                .values()
                .map(|value| value.view.clone())
                .collect(),
            attempts,
            artifacts: state.artifacts.values().cloned().collect(),
            receipts,
            degraded: state.degraded.clone(),
        })
    }
}
fn safe_output(bytes: &[u8]) -> (String, bool) {
    let mut result = String::new();
    for character in output_characters(bytes) {
        if result.len() + output_character_width(character) > 65_536 {
            return (result, true);
        }
        let text = if character.is_control() && !matches!(character, '\n' | '\t') {
            format!("\\u{{{:x}}}", character as u32)
        } else {
            character.to_string()
        };
        result.push_str(&text);
    }
    (result, false)
}

// utf8_chunks has the same replacement grouping as from_utf8_lossy, without
// allocating a normalized stream. Both paths share widths and the exact cap.
fn output_characters(bytes: &[u8]) -> impl Iterator<Item = char> + '_ {
    bytes.utf8_chunks().flat_map(|chunk| {
        chunk
            .valid()
            .chars()
            .chain((!chunk.invalid().is_empty()).then_some(char::REPLACEMENT_CHARACTER))
    })
}
fn output_character_width(c: char) -> usize {
    if c.is_control() && !matches!(c, '\n' | '\t') {
        4 + (u32::from(c).max(1).ilog(16) + 1) as usize
    } else {
        c.len_utf8()
    }
}
fn output_truncated(bytes: &[u8]) -> bool {
    let mut length = 0;
    for c in output_characters(bytes) {
        length += output_character_width(c);
        if length > 65_536 {
            return true;
        }
    }
    false
}
