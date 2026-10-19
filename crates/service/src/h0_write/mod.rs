//! Bounded experimental write boundary, not a production scheduler or transport.
//! All effect authority stays here; clients get supervisor-issued endpoint pairs.
mod probe;
#[cfg(test)]
mod tests;

use crate::terminal_worker::Worker;
pub use probe::{Config, Report, run};
use project_x_platform::local_process::PtyWriter;
use project_x_protocol::h0_write::{self as wire, Binding, Delivery, Intent, Receipt};
use project_x_terminal::{Frame, Input, Key, KeyPhase, Modifiers, rpc};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io;
use std::sync::Mutex;
use std::time::{Duration, Instant};

type Answer<T> = Result<T, &'static str>;
const PROFILE: &str = "h0-pty-once/0+px-dev-text-0";
const REVISION_CAP: u64 = 65_535;

// Not serializable or supplied by clients. A connection is bound before decoding.
#[derive(Clone, Copy)]
struct Endpoint {
    run: usize,
    connection: u16,
}
struct Run {
    principal: String,
    id: String,
    namespace: String,
    connection: u16,
    control: u16,
    capability: u16,
    connected: bool,
    open: bool,
    permitted: bool,
    records: BTreeMap<u16, Record>,
}
struct Record {
    hash: [u8; 32],
    receipt: Receipt,
    bytes: Vec<u8>,
    deadline: Instant,
}
// Restricted to the actual owned worker response. No wire operation constructs it.
#[derive(Clone, serde::Serialize)]
struct ReplyRecord {
    event: u16,
    source_revision: u64,
    guard: u16,
    requested: usize,
    accepted: Option<usize>,
    delivery: Delivery,
    #[serde(skip)]
    bytes: Vec<u8>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Lane {
    Application(usize, u16),
    Reply(usize),
}
struct Active {
    lane: Lane,
    token: u16,
}
struct Completion {
    lane: Lane,
    token: u16,
    outcome: io::Result<usize>,
}
trait WriteOnce {
    fn write_once(&mut self, bytes: &[u8]) -> io::Result<usize>;
}
impl WriteOnce for PtyWriter {
    fn write_once(&mut self, bytes: &[u8]) -> io::Result<usize> {
        PtyWriter::write_once(self, bytes)
    }
}
struct Core<W> {
    epoch: String,
    context: String,
    task: String,
    expires: Instant,
    runs: [Run; 2],
    frame: Frame,
    holder: Option<usize>,
    assignment: u16,
    writer_generation: u16,
    guard: u16,
    tainted: bool,
    pending_output: bool,
    active: Option<Active>,
    calls: u16,
    replies: Vec<ReplyRecord>,
    output_event: u16,
    writer: W,
}
impl<W: WriteOnce> Core<W> {
    fn new(epoch: String, frame: Frame, writer: W, context: String, task: String) -> Self {
        let runs = std::array::from_fn(|index| Run {
            principal: format!("principal-{index}"),
            id: format!("run-{index}"),
            namespace: format!("namespace-{index}"),
            connection: 1,
            control: 1,
            capability: 1,
            connected: true,
            open: true,
            permitted: true,
            records: BTreeMap::new(),
        });
        Self {
            epoch,
            context,
            task,
            expires: Instant::now() + Duration::from_secs(20),
            runs,
            frame,
            holder: Some(0),
            assignment: 1,
            writer_generation: 1,
            guard: 1,
            tainted: false,
            pending_output: false,
            active: None,
            calls: 0,
            replies: Vec::new(),
            output_event: 0,
            writer,
        }
    }
    fn endpoint(&self, index: usize) -> Endpoint {
        Endpoint {
            run: index,
            connection: self.runs[index].connection,
        }
    }
    fn endpoint_valid(&self, endpoint: Endpoint) -> Answer<()> {
        let run = self.runs.get(endpoint.run).ok_or("unissued_endpoint")?;
        if !run.connected || run.connection != endpoint.connection {
            return Err("disconnected_endpoint");
        }
        Ok(())
    }
    fn binding(&self, index: usize) -> Binding {
        let run = &self.runs[index];
        Binding {
            epoch: self.epoch.clone(),
            principal: run.principal.clone(),
            run: run.id.clone(),
            namespace: run.namespace.clone(),
            connection: run.connection,
            control: run.control,
            capability: run.capability,
            session: "session-0".into(),
            incarnation: 1,
            context: self.context.clone(),
            task: self.task.clone(),
            attempt: "attempt-0".into(),
            assignment: self.assignment,
            writer: self.writer_generation,
            guard: self.guard,
            revision: self.frame.revision,
            mode_revision: self.frame.mode_revision,
            cols: self.frame.cols,
            rows: self.frame.rows,
            profile: PROFILE.into(),
        }
    }
    fn known(&self) -> bool {
        !self.tainted
            && !self.frame.closed
            && self.frame.interpretation_known
            && self.frame.modes.input_protocol_known
            && self.frame.revision <= REVISION_CAP
            && self.frame.mode_revision <= REVISION_CAP
    }
    fn eligible(&self, index: usize, binding: &Binding, deadline: Instant) -> Answer<()> {
        let run = &self.runs[index];
        if !run.open || !run.permitted || !run.connected {
            return Err("run_closed_revoked_or_disconnected");
        }
        if self.holder != Some(index) || *binding != self.binding(index) {
            return Err("stale_exact_binding_or_assignment");
        }
        if !self.known() {
            return Err("new_session_required");
        }
        if self.pending_output {
            return Err("canonical_output_pending");
        }
        if Instant::now() >= deadline || Instant::now() >= self.expires {
            return Err("deadline_expired");
        }
        Ok(())
    }
    fn queued(&self) -> usize {
        // A fenced in-flight receipt may already say Unknown. Its private call
        // still occupies capacity until reconciliation; never count by receipt alone.
        self.runs
            .iter()
            .flat_map(|r| r.records.values())
            .filter(|r| r.receipt.delivery == Delivery::Queued)
            .count()
            + self
                .replies
                .iter()
                .filter(|r| r.delivery == Delivery::Queued)
                .count()
            + usize::from(self.active.is_some())
    }
    fn lookup(&self, endpoint: Endpoint, sequence: u16) -> Answer<Receipt> {
        self.endpoint_valid(endpoint)?;
        self.runs[endpoint.run]
            .records
            .get(&sequence)
            .map(|r| r.receipt.clone())
            .ok_or("receipt_missing_effects_unknown")
    }
    fn existing(
        &self,
        endpoint: Endpoint,
        sequence: u16,
        hash: [u8; 32],
    ) -> Answer<Option<Receipt>> {
        self.endpoint_valid(endpoint)?;
        match self.runs[endpoint.run].records.get(&sequence) {
            Some(record) if record.hash == hash => Ok(Some(record.receipt.clone())),
            Some(_) => Err("consumed_key_body_conflict"),
            None if sequence == 0 || sequence > wire::RECEIPTS => {
                Err("namespace_exhausted_or_invalid")
            }
            None if usize::from(sequence) != self.runs[endpoint.run].records.len() + 1 => {
                Err("sequence_gap")
            }
            None => Ok(None),
        }
    }
    fn bounded_binding(binding: &Binding) -> bool {
        [
            &binding.epoch,
            &binding.principal,
            &binding.run,
            &binding.namespace,
            &binding.session,
            &binding.context,
            &binding.task,
            &binding.attempt,
            &binding.profile,
        ]
        .into_iter()
        .all(|s| !s.is_empty() && s.len() <= 64 && s.bytes().all(|b| b.is_ascii_graphic()))
    }
    fn admit(
        &mut self,
        endpoint: Endpoint,
        sequence: u16,
        binding: Binding,
        hash: [u8; 32],
        bytes: Vec<u8>,
        deadline: Instant,
    ) -> Answer<Receipt> {
        if let Some(existing) = self.existing(endpoint, sequence, hash)? {
            return Ok(existing);
        }
        if !Self::bounded_binding(&binding) {
            return Err("binding_outside_profile");
        }
        let issued = self.binding(endpoint.run);
        if binding.epoch != issued.epoch
            || binding.principal != issued.principal
            || binding.run != issued.run
            || binding.namespace != issued.namespace
        {
            return Err("identity_not_issued_to_connection");
        }
        let error = self
            .eligible(endpoint.run, &binding, deadline)
            .err()
            .or_else(|| (self.queued() >= wire::QUEUE).then_some("queue_capacity"));
        let receipt = Receipt {
            sequence,
            origin: binding,
            delivery: if error.is_some() {
                Delivery::Zero
            } else {
                Delivery::Queued
            },
            requested: bytes.len(),
            accepted: Some(0),
            reason: error.unwrap_or("volatile_admission_only").into(),
            calls: 0,
        };
        self.runs[endpoint.run].records.insert(
            sequence,
            Record {
                hash,
                receipt: receipt.clone(),
                bytes,
                deadline,
            },
        );
        Ok(receipt)
    }
    fn transfer(
        &mut self,
        endpoint: Endpoint,
        sequence: u16,
        binding: Binding,
        target: &str,
        hash: [u8; 32],
    ) -> Answer<Receipt> {
        if let Some(existing) = self.existing(endpoint, sequence, hash)? {
            return Ok(existing);
        }
        let deadline = Instant::now() + Duration::from_millis(u64::from(wire::MAX_TTL_MS));
        if !Self::bounded_binding(&binding) {
            return Err("binding_outside_profile");
        }
        let issued = self.binding(endpoint.run);
        if binding.epoch != issued.epoch
            || binding.principal != issued.principal
            || binding.run != issued.run
            || binding.namespace != issued.namespace
        {
            return Err("identity_not_issued_to_connection");
        }
        // Protective transfer has a reserved metadata slot, not a byte-queue slot.
        let mut receipt = Receipt {
            sequence,
            origin: binding.clone(),
            delivery: Delivery::Zero,
            requested: 0,
            accepted: Some(0),
            reason: "control_decision_pending".into(),
            calls: 0,
        };
        let target_index = self
            .runs
            .iter()
            .position(|r| r.id == target && r.open && r.connected && r.permitted);
        let result = target_index
            .ok_or("target_run_unavailable")
            .and_then(|target| {
                self.eligible(endpoint.run, &binding, deadline)?;
                if target == endpoint.run {
                    return Err("already_holder");
                }
                self.advance_assignment()?;
                self.holder = Some(target);
                Ok(())
            });
        receipt.delivery = if result.is_ok() {
            Delivery::Transferred
        } else {
            Delivery::Zero
        };
        receipt.reason = result
            .err()
            .unwrap_or("assignment_and_writer_transferred_not_effects_stopped")
            .into();
        self.runs[endpoint.run].records.insert(
            sequence,
            Record {
                hash,
                receipt: receipt.clone(),
                bytes: Vec::new(),
                deadline,
            },
        );
        if receipt.delivery == Delivery::Transferred {
            self.fence_queued("authority_changed_before_call");
        }
        Ok(receipt)
    }
    fn advance_assignment(&mut self) -> Answer<()> {
        let Some(assignment) = self.assignment.checked_add(1) else {
            self.tainted = true;
            return Err("generation_exhausted");
        };
        let Some(writer) = self.writer_generation.checked_add(1) else {
            self.tainted = true;
            return Err("generation_exhausted");
        };
        self.assignment = assignment;
        self.writer_generation = writer;
        Ok(())
    }
    fn fence_queued(&mut self, reason: &str) {
        for index in 0..self.runs.len() {
            self.fence_run(index, reason);
        }
    }
    fn fence_run(&mut self, index: usize, reason: &str) {
        for record in self.runs[index].records.values_mut() {
            match record.receipt.delivery {
                Delivery::Queued => {
                    record.receipt.delivery = Delivery::Zero;
                    record.receipt.reason = reason.into();
                }
                Delivery::InFlight => {
                    record.receipt.delivery = Delivery::Unknown;
                    record.receipt.accepted = None;
                    record.receipt.reason = "inflight_extent_pending_original_completion".into();
                }
                _ => {}
            }
        }
    }
    fn poison(&mut self, reason: &str) {
        self.tainted = true;
        self.holder = None;
        if let Some(next) = self.guard.checked_add(1) {
            self.guard = next;
        }
        self.fence_queued(reason);
        for reply in &mut self.replies {
            if reply.delivery == Delivery::Queued {
                reply.delivery = Delivery::Zero;
            }
        }
    }
    // A restricted supervisor operation. It is intentionally not on the client wire.
    fn disconnect(&mut self, endpoint: Endpoint) -> Answer<()> {
        self.endpoint_valid(endpoint)?;
        self.runs[endpoint.run].connected = false;
        self.runs[endpoint.run].control = self.runs[endpoint.run]
            .control
            .checked_add(1)
            .ok_or("generation_exhausted")?;
        if self.holder == Some(endpoint.run) {
            self.advance_assignment()?;
            self.holder = None;
            self.fence_queued("disconnected_before_call");
        } else {
            self.fence_run(endpoint.run, "disconnected_before_call");
        }
        Ok(())
    }
    fn reconnect(&mut self, index: usize) -> Answer<Endpoint> {
        let run = &mut self.runs[index];
        if run.connected || !run.open || !run.permitted {
            return Err("supervisor_reconnect_refused");
        }
        run.connection = run
            .connection
            .checked_add(1)
            .ok_or("generation_exhausted")?;
        run.connected = true;
        // Reconnection does not regrant assignment/writer or revive queued work.
        Ok(self.endpoint(index))
    }
    fn close_or_revoke(&mut self, index: usize, revoke: bool) -> Answer<()> {
        let run = &mut self.runs[index];
        if revoke {
            run.permitted = false;
            run.capability = run
                .capability
                .checked_add(1)
                .ok_or("generation_exhausted")?;
        } else {
            run.open = false;
            run.control = run.control.checked_add(1).ok_or("generation_exhausted")?;
        }
        if self.holder == Some(index) {
            self.advance_assignment()?;
            self.holder = None;
            self.fence_queued("run_closed_or_revoked_before_call");
        } else {
            self.fence_run(index, "run_closed_or_revoked_before_call");
        }
        Ok(())
    }
    fn start(&mut self, endpoint: Endpoint, sequence: u16) -> Answer<Option<Completion>> {
        self.endpoint_valid(endpoint)?;
        let record = self.runs[endpoint.run]
            .records
            .get(&sequence)
            .ok_or("receipt_missing_effects_unknown")?;
        if record.receipt.delivery != Delivery::Queued {
            return Ok(None);
        }
        if self.active.is_some() {
            return Err("byte_lane_unresolved");
        }
        if self.replies.iter().any(|r| r.delivery == Delivery::Queued) {
            return Err("canonical_reply_pending");
        }
        // Final check and syscall are serialized with every authority/output change.
        let failure = self
            .eligible(endpoint.run, &record.receipt.origin, record.deadline)
            .err();
        if let Some(reason) = failure {
            let receipt = &mut self.runs[endpoint.run]
                .records
                .get_mut(&sequence)
                .unwrap()
                .receipt;
            receipt.delivery = Delivery::Zero;
            receipt.reason = reason.into();
            return Ok(None);
        }
        self.calls = self.calls.checked_add(1).ok_or("call_budget_exhausted")?;
        let record = self.runs[endpoint.run].records.get_mut(&sequence).unwrap();
        record.receipt.delivery = Delivery::InFlight;
        record.receipt.accepted = None;
        record.receipt.calls = 1;
        let lane = Lane::Application(endpoint.run, sequence);
        self.active = Some(Active {
            lane,
            token: self.calls,
        });
        // Exactly one OS call. Never write_all, Interrupted retry or a suffix loop.
        let outcome = self.writer.write_once(&record.bytes);
        Ok(Some(Completion {
            lane,
            token: self.calls,
            outcome,
        }))
    }
    fn finish(&mut self, completion: Completion) -> Answer<()> {
        let active = self.active.as_ref().ok_or("no_active_call")?;
        if active.lane != completion.lane || active.token != completion.token {
            return Err("completion_provenance_mismatch");
        }
        let requested = match completion.lane {
            Lane::Application(index, sequence) => self.runs[index].records[&sequence].bytes.len(),
            Lane::Reply(index) => self.replies[index].requested,
        };
        let accepted = match completion.outcome {
            Ok(n) if n <= requested => Some(n),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Some(0),
            _ => None,
        };
        let delivery = match accepted {
            Some(n) if n == requested => Delivery::Full,
            Some(0) => Delivery::Zero,
            Some(_) => Delivery::Partial,
            None => Delivery::Unknown,
        };
        match completion.lane {
            Lane::Application(index, sequence) => {
                let receipt = &mut self.runs[index].records.get_mut(&sequence).unwrap().receipt;
                receipt.accepted = accepted;
                receipt.delivery = delivery.clone();
                receipt.reason =
                    "single_call_extent_only_no_retry_not_application_completion".into();
            }
            Lane::Reply(index) => {
                self.replies[index].accepted = accepted;
                self.replies[index].delivery = delivery.clone();
            }
        }
        self.active = None;
        if matches!(delivery, Delivery::Partial | Delivery::Unknown) {
            self.poison("uncertain_input_new_session_required");
        }
        Ok(())
    }
    fn canonical(&mut self, frame: Frame, replies: Vec<u8>) -> Answer<()> {
        self.pending_output = false;
        if self.frame.revision.checked_add(1) != Some(frame.revision)
            || frame.mode_revision < self.frame.mode_revision
            || frame.cols != 80
            || frame.rows != 24
        {
            self.poison("canonical_discontinuity");
            return Err("canonical_discontinuity");
        }
        self.frame = frame;
        if !self.known() {
            self.poison("canonical_unknown_new_session_required");
            return Err("canonical_unknown");
        }
        self.output_event = match self.output_event.checked_add(1) {
            Some(next) => next,
            None => {
                self.poison("output_counter_exhausted");
                return Err("output_counter_exhausted");
            }
        };
        if !replies.is_empty() {
            if replies.len() > 128 || self.replies.len() >= 2 || self.queued() >= wire::QUEUE {
                self.poison("canonical_reply_capacity");
                return Err("canonical_reply_capacity");
            }
            self.replies.push(ReplyRecord {
                event: self.output_event,
                source_revision: self.frame.revision,
                guard: self.guard,
                requested: replies.len(),
                bytes: replies,
                accepted: Some(0),
                delivery: Delivery::Queued,
            });
        }
        Ok(())
    }
    fn flush_replies(&mut self) -> Answer<()> {
        if Instant::now() >= self.expires {
            self.poison("experiment_deadline_expired");
            return Err("experiment_deadline_expired");
        }
        if self.active.is_some() {
            return Err("byte_lane_unresolved");
        }
        for index in 0..self.replies.len() {
            if self.replies[index].delivery != Delivery::Queued {
                continue;
            }
            if !self.known() || self.pending_output || self.replies[index].guard != self.guard {
                self.replies[index].delivery = Delivery::Zero;
                continue;
            }
            self.calls = self.calls.checked_add(1).ok_or("call_budget_exhausted")?;
            self.replies[index].delivery = Delivery::InFlight;
            let lane = Lane::Reply(index);
            self.active = Some(Active {
                lane,
                token: self.calls,
            });
            let outcome = self.writer.write_once(&self.replies[index].bytes);
            self.finish(Completion {
                lane,
                token: self.calls,
                outcome,
            })?;
        }
        Ok(())
    }
}

struct Broker {
    core: Mutex<Core<PtyWriter>>,
    worker: Worker,
}
impl Broker {
    fn request(&self, endpoint: Endpoint, request: wire::Request) -> wire::Response {
        let result = self.operation(endpoint, request.operation);
        wire::Response {
            version: wire::Version::V0,
            id: request.id,
            result: result.map_err(str::to_owned),
        }
    }
    fn operation(&self, endpoint: Endpoint, operation: wire::Operation) -> Answer<wire::Body> {
        use wire::Operation;
        let hash: [u8; 32] =
            Sha256::digest(serde_json::to_vec(&operation).map_err(|_| "encode_request")?).into();
        match operation {
            Operation::Observe {} => {
                let core = self.core.lock().unwrap();
                core.endpoint_valid(endpoint)?;
                Ok(wire::Body::Observation {
                    value: wire::Observation {
                        binding: core.binding(endpoint.run),
                        holder: core.holder.map(|i| core.runs[i].id.clone()),
                        input_known: core.known(),
                        output_pending: core.pending_output,
                        active: core.active.is_some(),
                    },
                })
            }
            Operation::Query { sequence } => Ok(wire::Body::Receipt {
                value: self.core.lock().unwrap().lookup(endpoint, sequence)?,
            }),
            Operation::Queue {
                sequence,
                binding,
                ttl_ms,
                intent,
            } => {
                // Deadline includes encoding time, never reset after a blocked encode.
                let deadline = Instant::now() + Duration::from_millis(u64::from(ttl_ms));
                {
                    let core = self.core.lock().unwrap();
                    if let Some(value) = core.existing(endpoint, sequence, hash)? {
                        return Ok(wire::Body::Receipt { value });
                    }
                }
                if ttl_ms == 0 || ttl_ms > wire::MAX_TTL_MS {
                    return Err("deadline_profile");
                }
                let input = match intent {
                    Intent::Text { text }
                        if !text.is_empty()
                            && text.len() <= wire::MAX_TEXT
                            && text.bytes().all(|b| (32..=126).contains(&b)) =>
                    {
                        Input::Text { text }
                    }
                    Intent::Enter {} => Input::Key {
                        key: Key::Enter,
                        modifiers: Modifiers::default(),
                        phase: KeyPhase::Press,
                    },
                    _ => return Err("intent_outside_admitted_profile"),
                };
                let encoded = self.worker.call(rpc::Operation::Encode {
                    session: "session-0".into(),
                    input,
                });
                let (bytes, revision, mode_revision) = match encoded {
                    Ok(rpc::Reply::Encoded {
                        bytes,
                        revision,
                        mode_revision,
                    }) if bytes.len() <= wire::MAX_TEXT => (bytes, revision, mode_revision),
                    _ => {
                        self.core.lock().unwrap().poison("encoder_unavailable");
                        return Err("encoder_unavailable");
                    }
                };
                let mut core = self.core.lock().unwrap();
                if revision != binding.revision || mode_revision != binding.mode_revision {
                    return Err("encoding_cut_changed_observe_again");
                }
                let value = core.admit(endpoint, sequence, binding, hash, bytes, deadline)?;
                Ok(wire::Body::Receipt { value })
            }
            Operation::Dispatch { sequence } => {
                let completion = {
                    let mut core = self.core.lock().unwrap();
                    core.flush_replies()?;
                    core.start(endpoint, sequence)?
                };
                if let Some(completion) = completion {
                    // Deliberate separate trusted completion event: fences may arrive
                    // after the call returned, before its ACK is published.
                    self.core.lock().unwrap().finish(completion)?;
                }
                let mut core = self.core.lock().unwrap();
                core.flush_replies()?;
                Ok(wire::Body::Receipt {
                    value: core.lookup(endpoint, sequence)?,
                })
            }
            Operation::Transfer {
                sequence,
                binding,
                target_run,
            } => {
                let value = self.core.lock().unwrap().transfer(
                    endpoint,
                    sequence,
                    binding,
                    &target_run,
                    hash,
                )?;
                Ok(wire::Body::Receipt { value })
            }
        }
    }
}
