//! Fresh-resource evidence; no daemon or attachment to user sessions.
use super::*;
use project_x_platform::local_process as platform;
use std::{
    fs::{self, DirBuilder, File},
    io::Read,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};
#[derive(serde::Serialize)]
pub struct Provenance {
    version: String,
    producer_argv: Vec<String>,
    consumer_format: String,
    expected_artifact_sha256: String,
    profile: String,
    fixture_sha256: String,
    consumer_sha256: String,
    request_schema_sha256: String,
    response_schema_sha256: String,
    cwd: PathBuf,
    producer_program: PathBuf,
    consumer_program: String,
    environment: Vec<(String, String)>,
    capture: String,
}
#[derive(serde::Serialize)]
pub struct Report {
    provenance: Provenance,
    context: String,
    checks: Vec<String>,
    receipts: Vec<Receipt>,
    artifact: Option<Artifact>,
    updates: Vec<Observation>,
    final_human: Vec<Observation>,
    final_consumer: Vec<Observation>,
    takeover_ack: Body,
    output_counts: Vec<[usize; 2]>,
    takeover_us: Option<u128>,
    takeover_during_running_output: bool,
    limits: String,
}
struct Client {
    core: Arc<Mutex<Core>>,
    endpoint: Endpoint,
    cwd: PathBuf,
    fixture: PathBuf,
    producer_mode: String,
    next: u16,
}
impl Client {
    fn call(&mut self, operation: Operation) -> Result<Body, String> {
        let id = self.next;
        self.next = self.next.checked_add(1).ok_or("correlation_exhausted")?;
        let data = serde_json::to_vec(&wire::Request {
            version: wire::Version::V0,
            id,
            operation,
        })
        .map_err(|_| "request_encode")?;
        let request = wire::decode(&data).map_err(str::to_owned)?;
        let response = if let Operation::Dispatch { sequence } = request.operation {
            let Endpoint::Run(run) = self.endpoint else {
                return Err("run_endpoint_required".into());
            };
            let (receipt, child) = {
                let mut core = self.core.lock().unwrap();
                core.launch(run, sequence, |_, artifact| {
                    platform::launch_managed(&process::spec(
                        &self.cwd,
                        run,
                        artifact,
                        &self.producer_mode,
                        &self.fixture,
                    )?)
                })
                .map_err(str::to_owned)?
            };
            let value = if let Some(child) = child {
                let outcome = process::collect(&self.core, &receipt, child);
                self.core
                    .lock()
                    .unwrap()
                    .complete(&receipt, &outcome)
                    .map_err(str::to_owned)?
            } else {
                receipt
            };
            wire::Response {
                version: wire::Version::V0,
                id,
                result: Ok(Body::Receipt { value }),
            }
        } else {
            self.core.lock().unwrap().request(self.endpoint, request)
        };
        let bytes = serde_json::to_vec(&response).map_err(|_| "response_encode")?;
        if bytes.len() > wire::MAX_RESPONSE {
            return Err("response_capacity_unknown_no_retry".into());
        }
        let decoded: wire::Response =
            serde_json::from_slice(&bytes).map_err(|_| "invalid_response_unknown_no_retry")?;
        if decoded.id != id || decoded.version != wire::Version::V0 {
            return Err("response_mismatch_unknown_no_retry".into());
        }
        decoded.result
    }
    fn observe(&mut self, after: u16) -> Result<Observation, String> {
        match self.call(Operation::Observe { after })? {
            Body::Observation { value } => Ok(value),
            _ => Err("observation_required".into()),
        }
    }
    fn pages(&mut self) -> Result<Vec<Observation>, String> {
        let mut after = 0;
        let mut pages = Vec::new();
        loop {
            let page = self.observe(after)?;
            if page.cursor < after
                || (page.more && page.cursor == after)
                || pages.len() > wire::MAX_EVENTS
            {
                return Err("invalid_pagination".into());
            }
            after = page.cursor;
            let more = page.more;
            pages.push(page);
            if !more {
                return Ok(pages);
            }
        }
    }
    fn queue(&mut self, sequence: u16, dependency: Option<Artifact>) -> Result<Receipt, String> {
        let binding = self.observe(0)?.binding.ok_or("run_binding_required")?;
        match self.call(Operation::Queue {
            sequence,
            binding: Box::new(binding),
            dependency,
        })? {
            Body::Receipt { value } => Ok(value),
            _ => Err("receipt_required".into()),
        }
    }
    fn dispatch(&mut self, sequence: u16) -> Result<Receipt, String> {
        match self.call(Operation::Dispatch { sequence })? {
            Body::Receipt { value } => Ok(value),
            _ => Err("receipt_required".into()),
        }
    }
}
fn require(ok: bool, why: &str) -> Result<(), Box<dyn std::error::Error>> {
    if ok { Ok(()) } else { Err(why.into()) }
}
fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut f = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buf = [0; 8192];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
/// Real managed producer/consumer journey or active-producer takeover trial.
/// Every invocation creates a new private directory and at most two own children.
pub fn run(
    artifacts: &Path,
    fixture: &Path,
    mode: &str,
) -> Result<Report, Box<dyn std::error::Error>> {
    let noisy_takeover = mode == "takeover";
    let fault = mode == "nonzero" || mode == "overflow";
    if !["normal", "takeover", "nonzero", "overflow"].contains(&mode) {
        return Err("profile_mode".into());
    }
    if !fixture.is_absolute() {
        return Err("absolute_fixture_path_required".into());
    }
    DirBuilder::new().mode(0o700).create(artifacts)?;
    let cwd = artifacts.join("workspace");
    DirBuilder::new().mode(0o700).create(&cwd)?;
    let mut epoch = [0; 16];
    File::open("/dev/urandom")?.read_exact(&mut epoch)?;
    let epoch = epoch.iter().map(|b| format!("{b:02x}")).collect();
    let provenance = Provenance {
        version: "namtab.flow/0".into(),
        producer_argv: vec![
            match mode {
                "takeover" => "noisy",
                "normal" => "producer",
                other => other,
            }
            .into(),
        ],
        consumer_format: "consumed:%s\n".into(),
        expected_artifact_sha256: digest(CONTENT),
        profile: wire::PROFILE.into(),
        fixture_sha256: hash_file(fixture)?,
        consumer_sha256: hash_file(Path::new("/usr/bin/printf"))?,
        request_schema_sha256: digest(include_bytes!(
            "../../../../schemas/experimental/flow-request.schema.json"
        )),
        response_schema_sha256: digest(include_bytes!(
            "../../../../schemas/experimental/flow-response.schema.json"
        )),
        cwd: cwd.clone(),
        producer_program: fixture.into(),
        consumer_program: "/usr/bin/printf".into(),
        environment: vec![
            ("PATH".into(), "/usr/bin:/bin".into()),
            ("LC_ALL".into(), "C".into()),
        ],
        capture: "off; bounded intentional artifact; synthetic counts/digests only".into(),
    };
    let context = digest(&serde_json::to_vec(&provenance)?);
    let core = Arc::new(Mutex::new(Core::new(epoch, context.clone())));
    let client = |endpoint| Client {
        core: Arc::clone(&core),
        endpoint,
        cwd: cwd.clone(),
        fixture: fixture.into(),
        producer_mode: match mode {
            "takeover" => "noisy",
            "normal" => "producer",
            other => other,
        }
        .into(),
        next: 1,
    };
    let mut producer = client(Endpoint::Run(Run::Producer));
    let mut consumer = client(Endpoint::Run(Run::Consumer));
    let mut human = client(Endpoint::Human);
    require(
        consumer.queue(1, None).is_err(),
        "missing_dependency_admitted",
    )?;
    let queued = producer.queue(1, None)?;
    require(queued.state == State::Queued, "producer_not_queued")?;
    let mut receipts = Vec::new();
    let mut updates = Vec::new();
    let takeover_us;
    let takeover_during_running_output;
    let takeover_ack;
    if fault {
        takeover_us = None;
        takeover_during_running_output = false;
        let result = producer.dispatch(1)?;
        require(
            result.state == State::Failed && (mode != "nonzero" || result.exit_code == Some(3)),
            "failure_result_not_truthful",
        )?;
        require(
            consumer.observe(0)?.artifact.is_none() && consumer.queue(1, None).is_err(),
            "failed_producer_satisfied_dependency",
        )?;
        receipts.push(result);
        let before = human.observe(0)?;
        takeover_ack = human.call(Operation::Takeover {
            expected_control: before.control,
        })?;
        updates.push(consumer.observe(0)?);
    } else if noisy_takeover {
        // Scoped join also reaps our fixed child on an early observer error.
        let (elapsed, observed, receipt, cut, taken) =
            thread::scope(|scope| -> Result<_, Box<dyn std::error::Error>> {
                let dispatch = scope.spawn(|| producer.dispatch(1));
                let deadline = Instant::now() + Duration::from_secs(3);
                let observed = loop {
                    let cut = human.observe(0)?;
                    if cut.receipts.iter().any(|r| r.state == State::Running)
                        && cut.events.iter().any(|e| e.kind == "output")
                    {
                        break cut;
                    }
                    if Instant::now() >= deadline {
                        return Err("did_not_observe_running_output_for_takeover".into());
                    }
                    thread::sleep(Duration::from_millis(1));
                };
                let start = Instant::now();
                let taken = human.call(Operation::Takeover {
                    expected_control: observed.control,
                })?;
                require(
                    matches!(taken, Body::Taken { running: 1, .. }),
                    "takeover_not_during_running_attempt",
                )?;
                let elapsed = start.elapsed().as_micros();
                let receipt = dispatch.join().map_err(|_| "client_thread_failed")??;
                let cut = consumer.observe(0)?;
                require(
                    cut.human && cut.artifact.is_none(),
                    "takeover_published_artifact",
                )?;
                require(
                    consumer.queue(1, None).is_err(),
                    "old_agent_admitted_after_takeover",
                )?;
                Ok((elapsed, observed, receipt, cut, taken))
            })?;
        takeover_ack = taken;
        takeover_us = Some(elapsed);
        takeover_during_running_output = true;
        receipts.push(receipt);
        updates.push(observed);
        updates.push(cut);
    } else {
        takeover_us = None;
        takeover_during_running_output = false;
        receipts.push(producer.dispatch(1)?);
        let view = consumer.observe(0)?;
        let artifact = view.artifact.clone().ok_or("published_artifact_missing")?;
        require(
            artifact.sha256 == digest(CONTENT) && artifact.bytes == CONTENT.len() as u16,
            "wrong_artifact",
        )?;
        require(
            !view
                .events
                .iter()
                .any(|e| e.origin.as_ref().is_some_and(|b| b.run == Run::Producer)),
            "cross_scope_output",
        )?;
        consumer.queue(1, Some(artifact.clone()))?;
        let result = consumer.dispatch(1)?;
        require(
            result.state == State::Completed && result.dependency.as_ref() == Some(&artifact),
            "consumer_not_exact_dependency",
        )?;
        require(
            result.origin.run == Run::Consumer && receipts[0].origin.run == Run::Producer,
            "cross_attribution",
        )?;
        let duplicate = consumer.dispatch(1)?;
        require(
            duplicate == result && duplicate.calls == 1,
            "duplicate_launched_again",
        )?;
        receipts.push(result);
        let before = human.observe(0)?;
        takeover_ack = human.call(Operation::Takeover {
            expected_control: before.control,
        })?;
        updates.push(view);
        updates.push(human.observe(0)?);
    }
    let final_human = human.pages()?;
    let final_consumer = consumer.pages()?;
    let locked = core.lock().unwrap();
    let mut counts = vec![[0_usize; 2]; 2];
    for e in &locked.events {
        if let (Some(b), Some(stream)) = (&e.origin, e.stream) {
            counts[index(b.run)][if stream == Stream::Stdout { 0 } else { 1 }] +=
                usize::from(e.bytes);
        }
    }
    if !noisy_takeover && !fault {
        let expected = b"consumed:namtab-artifact-v1\n";
        let consumed = receipts.last().ok_or("consumer_receipt_missing")?;
        require(
            counts[1][0] == expected.len()
                && consumed.stdout_sha256.as_deref() == Some(digest(expected).as_str()),
            "consumer_literal_argv_bytes_not_observed",
        )?;
    }
    let report=Report{provenance,context,checks:vec!["missing_dependency_refused","issued_endpoint_attribution","scoped_output_and_shared_artifact","human_takeover_future_effects_fenced","bounded_streams_and_receipts"].into_iter().map(str::to_owned).collect(),receipts,artifact:locked.artifact.as_ref().map(|a|a.0.clone()),updates,final_human,final_consumer,takeover_ack,output_counts:counts,takeover_us,takeover_during_running_output,limits:"Synthetic in-process protocol handles, not external agents/models. Managed pipes only; no PTY/terminal input/reply/GUI/durable recovery/capacity or hard latency qualification. Takeover fences future mediated actions, not child effects.".into()};
    fs::write(
        artifacts.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}
