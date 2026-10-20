//! Experimental fixed-workflow coordinator. No scheduler, public auth or PTY input.
mod probe;
mod process;
#[cfg(test)]
mod tests;
pub use probe::{Report, run};
use project_x_protocol::flow::{
    self as wire, Artifact, Binding, Body, Event, Observation, Operation, Receipt, Run, State,
    Stream,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
type Answer<T> = Result<T, &'static str>;
const CONTENT: &[u8] = b"namtab-artifact-v1";
pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
/// Nonserializable issued handle, never constructed by decoding client labels.
#[derive(Clone, Copy)]
enum Endpoint {
    Run(Run),
    Human,
}
fn index(run: Run) -> usize {
    match run {
        Run::Producer => 0,
        Run::Consumer => 1,
    }
}
struct Record {
    hash: String,
    receipt: Receipt,
}
struct Core {
    epoch: String,
    context: String,
    control: u16,
    human: bool,
    known: bool,
    expires: Instant,
    records: [BTreeMap<u16, Record>; 2],
    events: Vec<Event>,
    artifact: Option<(Artifact, Vec<u8>)>,
}
impl Core {
    fn new(epoch: String, context: String) -> Self {
        Self {
            epoch,
            context,
            control: 1,
            human: false,
            known: true,
            expires: Instant::now() + Duration::from_secs(20),
            records: std::array::from_fn(|_| BTreeMap::new()),
            events: Vec::new(),
            artifact: None,
        }
    }
    fn binding(&self, run: Run) -> Binding {
        Binding {
            epoch: self.epoch.clone(),
            context: self.context.clone(),
            run,
            task: format!("task-{}", index(run)),
            attempt: format!("attempt-{}-1", index(run)),
            connection: 1,
            control: self.control,
            profile: wire::PROFILE.into(),
        }
    }
    fn eligible(&self, binding: &Binding) -> Answer<()> {
        if !self.known {
            return Err("incomplete_or_exhausted_new_invocation_required");
        }
        if self.human || *binding != self.binding(binding.run) {
            return Err("stale_context_or_human_owned");
        }
        if Instant::now() >= self.expires {
            return Err("invocation_expired");
        }
        Ok(())
    }
    fn fence(&mut self) {
        for r in self.records.iter_mut().flat_map(|r| r.values_mut()) {
            if r.receipt.state == State::Queued {
                r.receipt.state = State::Fenced;
                r.receipt.reason = "authority_or_resource_changed_before_launch".into();
            }
        }
    }
    fn event(
        &mut self,
        kind: &str,
        origin: Option<Binding>,
        stream: Option<Stream>,
        offset: usize,
        bytes: &[u8],
    ) -> Answer<()> {
        if self.events.len() >= wire::MAX_EVENTS {
            self.known = false;
            self.fence();
            return Err("event_capacity_incomplete");
        }
        self.events.push(Event {
            id: (self.events.len() + 1) as u16,
            kind: kind.into(),
            origin,
            stream,
            offset: offset as u16,
            bytes: bytes.len() as u16,
            sha256: (!bytes.is_empty()).then(|| digest(bytes)),
        });
        Ok(())
    }
    fn observe(&self, endpoint: Endpoint, after: u16) -> Answer<Observation> {
        if usize::from(after) > self.events.len() {
            return Err("future_cut");
        }
        let run = match endpoint {
            Endpoint::Run(r) => Some(r),
            Endpoint::Human => None,
        };
        let mut events = Vec::new();
        let mut cursor = after;
        for e in self.events.iter().skip(usize::from(after)) {
            cursor = e.id;
            if e.origin
                .as_ref()
                .is_none_or(|b| run.is_none_or(|r| r == b.run))
            {
                events.push(e.clone());
            }
            if events.len() == wire::PAGE {
                break;
            }
        }
        Ok(Observation {
            control: self.control,
            human: self.human,
            admission_known: self.known && Instant::now() < self.expires,
            binding: run.map(|r| self.binding(r)),
            artifact: self.artifact.as_ref().map(|a| a.0.clone()),
            receipts: self
                .records
                .iter()
                .enumerate()
                .filter(|(i, _)| run.is_none_or(|r| index(r) == *i))
                .flat_map(|(_, rs)| rs.values().map(|r| r.receipt.clone()))
                .collect(),
            events,
            cursor,
            more: usize::from(cursor) < self.events.len(),
        })
    }
    fn queue(
        &mut self,
        run: Run,
        sequence: u16,
        binding: Binding,
        dependency: Option<Artifact>,
        hash: String,
    ) -> Answer<Receipt> {
        let records = &self.records[index(run)];
        if let Some(r) = records.get(&sequence) {
            return if r.hash == hash {
                Ok(r.receipt.clone())
            } else {
                Err("consumed_sequence_body_conflict")
            };
        }
        if sequence == 0 || sequence > wire::SEQUENCES {
            return Err("sequence_capacity");
        }
        if !records.is_empty() {
            return Err("one_attempt_per_run");
        }
        if binding.run != run {
            return Err("endpoint_run_mismatch");
        }
        self.eligible(&binding)?;
        match (run, &dependency, &self.artifact) {
            (Run::Producer, None, _) => {}
            (Run::Consumer, Some(a), Some((current, _))) if a == current => {}
            _ => return Err("exact_dependency_missing_or_stale"),
        }
        let receipt = Receipt {
            sequence,
            origin: binding.clone(),
            dependency,
            state: State::Queued,
            calls: 0,
            reason: "ram_admission_not_execution".into(),
            exit_code: None,
            drained: false,
            stdout_bytes: 0,
            stderr_bytes: 0,
            stdout_sha256: None,
        };
        self.event("queued", Some(binding), None, 0, &[])?;
        self.records[index(run)].insert(
            sequence,
            Record {
                hash,
                receipt: receipt.clone(),
            },
        );
        Ok(receipt)
    }
    /// Caller holds the same mutex used by takeover. Validation is adjacent to
    /// the actual spawn, not a reusable permit. Kernel spawn has no hard bound.
    fn launch<T>(
        &mut self,
        run: Run,
        sequence: u16,
        spawn: impl FnOnce(&Receipt, Option<&[u8]>) -> std::io::Result<T>,
    ) -> Answer<(Receipt, Option<T>)> {
        let original = self.records[index(run)]
            .get(&sequence)
            .ok_or("receipt_missing_effects_unknown")?
            .receipt
            .clone();
        if original.state != State::Queued {
            return Ok((original, None));
        }
        let dependency_ok = match (&original.dependency, &self.artifact, run) {
            (None, _, Run::Producer) => true,
            (Some(a), Some((current, bytes)), Run::Consumer) => {
                a == current && a.sha256 == digest(bytes) && usize::from(a.bytes) == bytes.len()
            }
            _ => false,
        };
        if self.eligible(&original.origin).is_err() || !dependency_ok {
            let receipt = &mut self.records[index(run)].get_mut(&sequence).unwrap().receipt;
            receipt.state = State::Fenced;
            receipt.reason = "final_launch_binding_or_dependency_changed".into();
            return Ok((receipt.clone(), None));
        }
        self.event(
            "launch_boundary",
            Some(original.origin.clone()),
            None,
            0,
            &[],
        )?;
        let r = &mut self.records[index(run)].get_mut(&sequence).unwrap().receipt;
        r.state = State::Running;
        r.calls = 1;
        r.reason = "launch_started_effects_not_stopped_by_takeover".into();
        let result = spawn(r, self.artifact.as_ref().map(|a| a.1.as_slice()));
        let r = &mut self.records[index(run)].get_mut(&sequence).unwrap().receipt;
        match result {
            Ok(child) => Ok((r.clone(), Some(child))),
            Err(_) => {
                r.state = State::Unknown;
                r.reason = "launch_error_effects_unknown_no_retry".into();
                Ok((r.clone(), None))
            }
        }
    }
    fn output(
        &mut self,
        origin: &Binding,
        stream: Stream,
        offset: usize,
        bytes: &[u8],
    ) -> Answer<()> {
        // Output is attributed to the immutable launched attempt, even after takeover.
        if !self.records[index(origin.run)]
            .values()
            .any(|r| r.receipt.origin == *origin && r.receipt.state == State::Running)
        {
            return Err("output_origin_not_running");
        }
        if offset
            .checked_add(bytes.len())
            .is_none_or(|n| n > wire::STREAM_BYTES)
        {
            self.known = false;
            self.fence();
            return Err("stream_capacity_incomplete");
        }
        self.event("output", Some(origin.clone()), Some(stream), offset, bytes)
    }
    fn complete(&mut self, origin: &Receipt, outcome: &process::Outcome) -> Answer<Receipt> {
        let r = &self.records[index(origin.origin.run)]
            .get(&origin.sequence)
            .ok_or("missing_original_launch")?
            .receipt;
        if r.origin != origin.origin || r.state != State::Running || r.calls != 1 {
            return Err("completion_provenance_mismatch");
        }
        if !outcome.drained {
            self.known = false;
            self.fence();
        }
        let success = outcome.exit_code == Some(0) && outcome.drained;
        let can_publish = success && self.eligible(&origin.origin).is_ok();
        let mut artifact = None;
        if origin.origin.run == Run::Producer && can_publish && outcome.stdout == CONTENT {
            artifact = Some((
                Artifact {
                    epoch: self.epoch.clone(),
                    context: self.context.clone(),
                    producer_attempt: origin.origin.attempt.clone(),
                    revision: 1,
                    sha256: digest(&outcome.stdout),
                    bytes: outcome.stdout.len() as u16,
                },
                outcome.stdout.clone(),
            ));
        }
        let event = self.event("result", Some(origin.origin.clone()), None, 0, &[]);
        let r = &mut self.records[index(origin.origin.run)]
            .get_mut(&origin.sequence)
            .unwrap()
            .receipt;
        r.exit_code = outcome.exit_code;
        r.drained = outcome.drained;
        r.stdout_bytes = outcome.counts[0] as u16;
        r.stderr_bytes = outcome.counts[1] as u16;
        r.stdout_sha256 = outcome.drained.then(|| digest(&outcome.stdout));
        r.state = if success {
            State::Completed
        } else {
            State::Failed
        };
        r.reason = if !success {
            "failed_or_incomplete_no_artifact"
        } else if origin.origin.run == Run::Producer && artifact.is_none() {
            "process_completed_artifact_not_valid_or_authorized"
        } else {
            "exit_and_streams_observed_not_correctness"
        }
        .into();
        if event.is_err() {
            r.reason = "event_capacity_result_incomplete".into();
            r.state = State::Failed;
        }
        let result = r.clone();
        if event.is_ok()
            && let Some(value) = artifact
        {
            self.event("artifact_published", None, None, 0, &[])?;
            self.artifact = Some(value);
        }
        Ok(result)
    }
    fn takeover(&mut self, expected: u16) -> Answer<u16> {
        if self.control != expected || self.human {
            return Err("stale_takeover");
        }
        self.control = self.control.checked_add(1).ok_or("generation_exhausted")?;
        self.human = true;
        self.fence();
        // Takeover remains effective even when the journal is exhausted; the
        // observation explicitly reports input_known=false in that case.
        let _ = self.event("human_takeover_future_effects_fenced", None, None, 0, &[]);
        Ok(self.control)
    }
    fn request(&mut self, endpoint: Endpoint, request: wire::Request) -> wire::Response {
        let result = (|| match request.operation {
            Operation::Observe { after } => Ok(Body::Observation {
                value: self.observe(endpoint, after)?,
            }),
            Operation::Queue {
                sequence,
                binding,
                dependency,
            } => {
                let Endpoint::Run(run) = endpoint else {
                    return Err("human_control_endpoint_not_run");
                };
                let hash = digest(
                    &serde_json::to_vec(&(sequence, &binding, &dependency))
                        .map_err(|_| "encode")?,
                );
                Ok(Body::Receipt {
                    value: self.queue(run, sequence, *binding, dependency, hash)?,
                })
            }
            Operation::Takeover { expected_control } => {
                if !matches!(endpoint, Endpoint::Human) {
                    return Err("human_endpoint_required");
                }
                let running = self
                    .records
                    .iter()
                    .flat_map(|rs| rs.values())
                    .filter(|r| r.receipt.state == State::Running)
                    .count() as u16;
                Ok(Body::Taken {
                    control: self.takeover(expected_control)?,
                    running,
                })
            }
            Operation::Dispatch { .. } => Err("dispatch_requires_runtime_boundary"),
        })();
        wire::Response {
            version: wire::Version::V0,
            id: request.id,
            result: result.map_err(str::to_owned),
        }
    }
}
