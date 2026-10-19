//! Fresh-resource executable evidence. Not an interactive daemon or SDK.
use super::*;
use project_x_platform::local_process::{self as platform, CommandSpec, Geometry, ReadOutcome};
use std::fs::{self, DirBuilder, File};
use std::io::{BufReader, Read, Write};
use std::os::unix::fs::DirBuilderExt;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;

pub struct Config {
    pub artifacts: PathBuf,
    pub worker: PathBuf,
    pub pty_helper: PathBuf,
}
#[derive(serde::Serialize)]
pub struct Provenance {
    pub version: String,
    pub profile: String,
    pub worker_sha256: String,
    pub pty_helper_sha256: String,
    pub program_sha256: String,
    pub request_schema_sha256: String,
    pub response_schema_sha256: String,
    pub cwd: PathBuf,
    pub program: String,
    pub argv: Vec<String>,
    pub environment: Vec<(String, String)>,
    pub capture: String,
    pub geometry: (u16, u16),
}
#[derive(serde::Serialize)]
pub struct ReplyEvidence {
    pub output_event: u16,
    pub source_revision: u64,
    pub guard: u16,
    pub bytes_hex: String,
    pub delivery: Delivery,
    pub accepted: Option<usize>,
}
#[derive(serde::Serialize)]
pub struct Report {
    pub provenance: Provenance,
    pub context_sha256: String,
    pub task_sha256: String,
    pub profile: String,
    pub checks: Vec<String>,
    pub receipts: Vec<Receipt>,
    pub replies: usize,
    pub reply_evidence: Vec<ReplyEvidence>,
    pub output_bytes: usize,
    pub limits: String,
}
struct Client {
    stream: UnixStream,
    reader: BufReader<UnixStream>,
    next: u16,
    thread: Option<JoinHandle<()>>,
}
impl Client {
    fn attach(broker: Arc<Broker>, endpoint: Endpoint) -> io::Result<Self> {
        let (stream, mut server) = UnixStream::pair()?;
        for socket in [&stream, &server] {
            socket.set_read_timeout(Some(Duration::from_secs(2)))?;
            socket.set_write_timeout(Some(Duration::from_secs(2)))?;
        }
        let reader = BufReader::new(stream.try_clone()?);
        let thread = std::thread::spawn(move || {
            let mut reader = BufReader::new(server.try_clone().unwrap());
            while let Ok(Some(frame)) = crate::terminal_worker::line(&mut reader, wire::MAX_FRAME) {
                let Ok(request) = wire::decode(&frame) else {
                    break;
                };
                let response = broker.request(endpoint, request);
                let Ok(mut body) = serde_json::to_vec(&response) else {
                    break;
                };
                if body.len() > wire::MAX_FRAME {
                    break;
                }
                body.push(b'\n');
                if server.write_all(&body).is_err() {
                    break;
                }
            }
            let _ = broker.core.lock().unwrap().disconnect(endpoint);
        });
        Ok(Self {
            stream,
            reader,
            next: 1,
            thread: Some(thread),
        })
    }
    fn call(&mut self, operation: wire::Operation) -> Result<wire::Body, String> {
        let id = self.next;
        self.next = self.next.checked_add(1).ok_or("correlation_exhausted")?;
        let request = wire::Request {
            version: wire::Version::V0,
            id,
            operation,
        };
        let mut body = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
        if body.len() > wire::MAX_FRAME {
            return Err("client_frame_capacity".into());
        }
        body.push(b'\n');
        self.stream
            .write_all(&body)
            .map_err(|_| "transport_unknown_no_retry")?;
        let frame = crate::terminal_worker::line(&mut self.reader, wire::MAX_FRAME)
            .map_err(|_| "transport_unknown_no_retry")?
            .ok_or("transport_unknown_no_retry")?;
        let response: wire::Response =
            serde_json::from_slice(&frame).map_err(|_| "invalid_response_unknown_no_retry")?;
        if response.id != id || response.version != wire::Version::V0 {
            return Err("response_mismatch_unknown_no_retry".into());
        }
        response.result
    }
    fn observe(&mut self) -> Result<wire::Observation, String> {
        match self.call(wire::Operation::Observe {})? {
            wire::Body::Observation { value } => Ok(value),
            _ => Err("expected_observation".into()),
        }
    }
    fn receipt(&mut self, operation: wire::Operation) -> Result<Receipt, String> {
        match self.call(operation)? {
            wire::Body::Receipt { value } => Ok(value),
            _ => Err("expected_receipt".into()),
        }
    }
    fn queue(&mut self, sequence: u16, intent: Intent) -> Result<Receipt, String> {
        let binding = self.observe()?.binding;
        self.receipt(wire::Operation::Queue {
            sequence,
            binding,
            ttl_ms: wire::MAX_TTL_MS,
            intent,
        })
    }
    fn send(&mut self, sequence: u16, intent: Intent) -> Result<Receipt, String> {
        let queued = self.queue(sequence, intent)?;
        if queued.delivery != Delivery::Queued {
            return Err(format!("unexpected_admission:{}", queued.reason));
        }
        let receipt = self.receipt(wire::Operation::Dispatch { sequence })?;
        if receipt.delivery != Delivery::Full || receipt.calls != 1 {
            return Err(format!("single_write_not_full:{}", receipt.reason));
        }
        Ok(receipt)
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
fn require(condition: bool, message: &str) -> Result<(), Box<dyn std::error::Error>> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
// The full output is synthetic and is NOT a command attribution oracle. Its
// expected markers establish only this fixture's observed process behavior.
fn pump(
    broker: &Broker,
    reader: &mut platform::PtyReader,
    output: &mut Vec<u8>,
    expected: &[u8],
    deadline: Instant,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0_u8; 512];
    loop {
        if Instant::now() >= deadline {
            return Err("probe_hard_deadline".into());
        }
        match reader.read_chunk(&mut buffer, Duration::from_millis(20))? {
            ReadOutcome::Data(n) => {
                if output.len() + n > 65_536 {
                    broker
                        .core
                        .lock()
                        .unwrap()
                        .poison("output_budget_exhausted");
                    return Err("synthetic_output_budget_exhausted".into());
                }
                output.extend_from_slice(&buffer[..n]);
                broker.core.lock().unwrap().pending_output = true;
                let result = broker.worker.call(rpc::Operation::Feed {
                    session: "session-0".into(),
                    bytes: buffer[..n].to_vec(),
                });
                match result {
                    Ok(rpc::Reply::Fed { frame, replies }) => {
                        let mut core = broker.core.lock().unwrap();
                        core.canonical(frame, replies)?;
                        core.flush_replies()?;
                    }
                    _ => {
                        broker.core.lock().unwrap().poison("worker_output_unknown");
                        return Err("worker_output_unknown".into());
                    }
                }
            }
            ReadOutcome::Eof => {
                broker.core.lock().unwrap().poison("process_output_closed");
                return Err("early_pty_eof".into());
            }
            ReadOutcome::Timeout if output.windows(expected.len()).any(|w| w == expected) => {
                return Ok(());
            }
            ReadOutcome::Timeout => {}
        }
    }
}
pub fn run(config: Config) -> Result<Report, Box<dyn std::error::Error>> {
    if !config.artifacts.is_absolute()
        || !config.worker.is_absolute()
        || !config.pty_helper.is_absolute()
    {
        return Err("absolute_paths_required".into());
    }
    // Exclusive creation. Never replace, attach to or remove a preexisting path.
    DirBuilder::new().mode(0o700).create(&config.artifacts)?;
    let workspace = config.artifacts.join("workspace");
    DirBuilder::new().mode(0o700).create(&workspace)?;
    let mut epoch = [0_u8; 16];
    File::open("/dev/urandom")?.read_exact(&mut epoch)?;
    let epoch = epoch.iter().map(|b| format!("{b:02x}")).collect();
    let worker = Worker::start(&config.worker, &workspace)?;
    let rpc::Reply::Frame(frame) = worker.call(rpc::Operation::New {
        session: "session-0".into(),
        cols: 80,
        rows: 24,
    })?
    else {
        return Err("new_frame_required".into());
    };
    let mut spec = CommandSpec::new("/bin/bash", &workspace);
    spec.args = ["--noprofile", "--norc"].map(Into::into).into();
    spec.env = [
        ("PATH", "/usr/bin:/bin"),
        ("LC_ALL", "C.UTF-8"),
        ("TERM", "dumb"),
        ("HOME", "/nonexistent"),
        ("HISTFILE", "/dev/null"),
        ("INPUTRC", "/dev/null"),
        ("BASH_ENV", "/dev/null"),
        ("ENV", "/dev/null"),
        ("PS1", "h0$ "),
    ]
    .map(|(k, v)| (k.into(), v.into()))
    .into();
    let digest = |bytes: &[u8]| format!("{:x}", Sha256::digest(bytes));
    let file_digest = |path: &std::path::Path| -> io::Result<String> {
        let mut file = File::open(path)?;
        let mut hash = Sha256::new();
        let mut buffer = [0_u8; 8192];
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hash.update(&buffer[..count]);
        }
        Ok(format!("{:x}", hash.finalize()))
    };
    let provenance = Provenance {
        version: "namtab.h0-write/0".into(),
        profile: PROFILE.into(),
        worker_sha256: file_digest(&config.worker)?,
        pty_helper_sha256: file_digest(&config.pty_helper)?,
        program_sha256: file_digest(std::path::Path::new("/bin/bash"))?,
        request_schema_sha256: digest(include_bytes!(
            "../../../../schemas/experimental/h0-write-request.schema.json"
        )),
        response_schema_sha256: digest(include_bytes!(
            "../../../../schemas/experimental/h0-write-response.schema.json"
        )),
        cwd: workspace.clone(),
        program: "/bin/bash".into(),
        argv: vec!["--noprofile".into(), "--norc".into()],
        environment: spec
            .env
            .iter()
            .map(|(k, v)| {
                (
                    k.to_string_lossy().into_owned(),
                    v.to_string_lossy().into_owned(),
                )
            })
            .collect(),
        capture: "off; explicit synthetic fixture output only".into(),
        geometry: (80, 24),
    };
    let context_sha256 = digest(&serde_json::to_vec(&provenance)?);
    let task_sha256 = digest(
        b"h0-pty-once/0: two scripted clients; one context/attempt; handoff; no automatic retry",
    );
    let platform::PtyProcess {
        mut process,
        mut reader,
        writer,
    } = platform::launch_pty(&config.pty_helper, &spec, Geometry { cols: 80, rows: 24 })?;
    let broker = Arc::new(Broker {
        core: Mutex::new(Core::new(
            epoch,
            frame,
            writer,
            context_sha256.clone(),
            task_sha256.clone(),
        )),
        worker,
    });
    let mut output = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    pump(&broker, &mut reader, &mut output, b"h0$ ", deadline)?;
    let mut a = Client::attach(Arc::clone(&broker), broker.core.lock().unwrap().endpoint(0))?;
    let mut b = Client::attach(Arc::clone(&broker), broker.core.lock().unwrap().endpoint(1))?;
    // An untrusted binding cannot impersonate the other issued socket endpoint.
    let forged = a.observe()?.binding;
    require(
        b.receipt(wire::Operation::Queue {
            sequence: 1,
            binding: forged,
            ttl_ms: wire::MAX_TTL_MS,
            intent: Intent::Text {
                text: "FORGED_CLIENT".into(),
            },
        })
        .is_err(),
        "cross_principal_admission",
    )?;
    let mut receipts = Vec::new();
    receipts.push(a.send(
        1,
        Intent::Text {
            text: "printf 'H0_A_DONE\\n'".into(),
        },
    )?);
    pump(&broker, &mut reader, &mut output, b"printf", deadline)?;
    receipts.push(a.send(2, Intent::Enter {})?);
    pump(
        &broker,
        &mut reader,
        &mut output,
        b"\r\nH0_A_DONE\r\n",
        deadline,
    )?;
    // Real ownership transfer leaves an old admitted intent queued, then proves zero.
    let stale = a.queue(
        3,
        Intent::Text {
            text: "SHOULD_NEVER_BE_WRITTEN".into(),
        },
    )?;
    require(
        stale.delivery == Delivery::Queued,
        "stale_fixture_was_not_queued",
    )?;
    let binding = a.observe()?.binding;
    receipts.push(a.receipt(wire::Operation::Transfer {
        sequence: 4,
        binding,
        target_run: "run-1".into(),
    })?);
    require(
        receipts.last().unwrap().delivery == Delivery::Transferred,
        "transfer_failed",
    )?;
    let closed = a.receipt(wire::Operation::Dispatch { sequence: 3 })?;
    require(
        closed.delivery == Delivery::Zero && closed.calls == 0 && closed.origin == stale.origin,
        "stale_write_or_relabel",
    )?;
    receipts.push(closed);
    receipts.push(b.send(
        1,
        Intent::Text {
            text: "printf 'H0_B_DONE\\n'".into(),
        },
    )?);
    pump(&broker, &mut reader, &mut output, b"H0_B_DONE", deadline)?;
    receipts.push(b.send(2, Intent::Enter {})?);
    pump(
        &broker,
        &mut reader,
        &mut output,
        b"\r\nH0_B_DONE\r\n",
        deadline,
    )?;
    require(
        receipts[0].origin.run != receipts[4].origin.run
            && receipts[0].sequence == receipts[4].sequence,
        "cross_namespace_attribution",
    )?;
    // Disconnect/reconnect is supervisor-controlled and never regrants a writer.
    let original = a.receipt(wire::Operation::Query { sequence: 1 })?;
    drop(a);
    let resumed = broker.core.lock().unwrap().reconnect(0)?;
    let mut a = Client::attach(Arc::clone(&broker), resumed)?;
    require(
        a.receipt(wire::Operation::Query { sequence: 1 })? == original,
        "resume_relabelled_receipt",
    )?;
    require(
        a.observe()?.holder.as_deref() == Some("run-1"),
        "reconnect_stole_writer",
    )?;
    // The owned canonical worker generates a real DSR reply from PTY output.
    receipts.push(b.send(
        3,
        Intent::Text {
            text: "printf '\\033[5n'".into(),
        },
    )?);
    pump(&broker, &mut reader, &mut output, b"033[5n", deadline)?;
    receipts.push(b.send(4, Intent::Enter {})?);
    while broker.core.lock().unwrap().replies.is_empty() {
        pump(&broker, &mut reader, &mut output, b"h0$ ", deadline)?;
    }
    {
        let core = broker.core.lock().unwrap();
        require(
            core.replies.len() == 1
                && core.replies[0].delivery == Delivery::Full
                && core.replies[0].bytes == b"\x1b[0n",
            "canonical_reply_not_exactly_once_dsr_0",
        )?;
    }
    require(
        !output.windows(23).any(|w| w == b"SHOULD_NEVER_BE_WRITTEN"),
        "stale_bytes_observed",
    )?;
    // End only this owned fixture; no user daemon, socket, process or group used.
    broker.core.lock().unwrap().close_or_revoke(0, true)?;
    broker.core.lock().unwrap().close_or_revoke(1, false)?;
    require(
        process.kill_and_wait()?.code().is_none(),
        "expected_fixture_direct_child_signal",
    )?;
    drop(a);
    drop(b);
    let core = broker.core.lock().unwrap();
    let report = Report {
        provenance, context_sha256, task_sha256,
        profile: PROFILE.into(),
        checks: vec!["two_socket_pair_clients_distinct_namespace_sequence_1", "real_pty_text_enter_and_observed_markers", "transfer_stale_queued_intent_zero_calls", "reconnect_original_receipt_no_writer_regrant", "canonical_dsr_single_reply_independent_of_run", "owned_direct_child_reaped"].into_iter().map(str::to_owned).collect(),
        receipts,
        replies: core.replies.len(),
        reply_evidence: core.replies.iter().map(|r| ReplyEvidence {
            output_event: r.event, source_revision: r.source_revision, guard: r.guard,
            bytes_hex: r.bytes.iter().map(|b| format!("{b:02x}")).collect(),
            delivery: r.delivery.clone(), accepted: r.accepted,
        }).collect(),
        output_bytes: output.len(),
        limits: "Scripted clients only; one merged PTY stream, no universal shell-command/output attribution; RAM-only receipts, no durability/crash recovery/capacity/GUI/model or latency qualification.".into(),
    };
    // Synthetic output explicitly retained by this test only, not general capture.
    fs::write(config.artifacts.join("synthetic-output.bin"), &output)?;
    fs::write(
        config.artifacts.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}
