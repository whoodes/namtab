//! Bounded frontend transport worker; one public client, no private state/PTY access.
use super::{Control, Fence, Input};
use project_x_client::{
    mk1::Client,
    mk1_protocol::{self as p, Op, Role, Snapshot},
};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        mpsc::{self, SyncSender, TryRecvError},
    },
    thread,
    time::{Duration, Instant},
};
#[derive(Debug)]
pub enum Update {
    Connected(String),
    Snapshot(Box<Snapshot>),
    Notice(String),
    Paused(String),
    Offline,
    Granted { focus: u64, session: String },
}
struct Command {
    value: Control,
    focus: u64,
}
pub struct Connection {
    control: SyncSender<Command>,
    input: SyncSender<Input>,
    fence: Arc<Fence>,
}
impl Connection {
    pub fn start(
        socket: PathBuf,
        fence: Arc<Fence>,
        notify: impl Fn(Update) + Send + 'static,
    ) -> Self {
        let (ct, cr) = mpsc::sync_channel::<Command>(2);
        let (it, ir) = mpsc::sync_channel::<Input>(8);
        let sender_fence = Arc::clone(&fence);
        thread::spawn(move || {
            let mut client: Option<Client> = None;
            let mut last = Instant::now() - Duration::from_secs(1);
            let mut connect = true;
            let mut resume: Option<(String, u64, String)> = None;
            let mut demo_started = false;
            let mut demo: Option<std::process::Child> = None;
            loop {
                if connect {
                    connect = false;
                    let result = explicit_connect(&socket, resume.as_ref());
                    match result {
                        Ok(c) => {
                            super::trace::emit(
                                serde_json::json!({"event":"connection_ready","endpoint":c.endpoint,"epoch":c.epoch,"next_request_id":c.next_id(),"resumed":resume.is_some()}),
                            );
                            notify(Update::Connected(c.endpoint.clone()));
                            client = Some(c);
                        }
                        Err(_) => {
                            super::trace::emit(
                                serde_json::json!({"event":"connection_unavailable"}),
                            );
                            notify(Update::Paused(
                                "Connection unavailable; explicit reconnect only.".into(),
                            ));
                            notify(Update::Offline);
                        }
                    }
                }
                let cmd = match cr.try_recv() {
                    Ok(c) => Some(c),
                    Err(TryRecvError::Disconnected) => break,
                    Err(TryRecvError::Empty) => None,
                };
                if let Some(child) = demo.as_mut()
                    && let Ok(Some(status)) = child.try_wait()
                {
                    notify(Update::Notice(format!(
                        "Scripted helper exited {status}; see actual broker results, not inferred model status."
                    )));
                    demo = None;
                }
                if let Some(command) = cmd {
                    if !fence.valid(command.focus) {
                        notify(Update::Notice(
                            "Stale UI action discarded before dispatch; no retry.".into(),
                        ));
                        continue;
                    }
                    if matches!(command.value, Control::Reconnect) {
                        super::trace::emit(
                            serde_json::json!({"event":"reconnect_requested","focus_generation":command.focus}),
                        );
                        if let Some(mut old) = client.take() {
                            resume = Some((old.resume.clone(), old.next_id(), old.epoch.clone()));
                            old.close_transport();
                            drop(old);
                        }
                        connect = true;
                    } else if matches!(command.value, Control::Demo) {
                        if client.is_none() {
                            notify(Update::Notice(
                                "Disconnected: scripted launch refused.".into(),
                            ));
                            continue;
                        }
                        if !demo_started {
                            demo_started = true;
                            let result = std::env::current_exe().and_then(|p| {
                                std::process::Command::new(p.with_file_name("namtab-demo"))
                                    .arg("--socket")
                                    .arg(&socket)
                                    .arg("--epoch")
                                    .arg(&client.as_ref().unwrap().epoch)
                                    .stdin(std::process::Stdio::null())
                                    .spawn()
                            });
                            let started = match result {
                                Ok(child) => {
                                    demo = Some(child);
                                    true
                                }
                                Err(_) => false,
                            };
                            notify(Update::Notice(if started {
                                "Scripted demo helper started, not a model. Observe actual attempts; two lifetime slots.".into()
                            } else {
                                "Scripted demo could not start; no automatic retry. Check same-build namtab-demo.".into()
                            }));
                        } else {
                            notify(Update::Notice("Demo already requested. No automatic rerun; use a fresh explicit namespace for more slots.".into()));
                        }
                    } else if let Some(c) = client.as_mut() {
                        // Controls retain the exact UI generation, target and explicit user action.
                        if fence.valid(command.focus) {
                            let result = control(c, command.value, command.focus, &fence, &notify);
                            if let Err(e) = result {
                                fence.advance();
                                notify(Update::Paused(e.to_string()));
                                if c_failed(c) {
                                    resume = Some((c.resume.clone(), c.next_id(), c.epoch.clone()));
                                    client = None;
                                    notify(Update::Offline);
                                }
                            }
                        }
                    } else {
                        notify(Update::Paused(
                            "Disconnected; no action queued or replayed.".into(),
                        ));
                    }
                } else if let Ok(input) = ir.try_recv() {
                    if fence.valid(input.focus)
                        && let Some(c) = client.as_mut()
                    {
                        super::trace::emit(
                            serde_json::json!({"event":"input_dispatch","local":input.trace,"request_id":c.next_id(),"focus_generation":input.focus,"current_focus_generation":fence.get(),"session":input.session,"generation":input.generation}),
                        );
                        let result = c.call(Op::Input {
                            session: input.session,
                            generation: input.generation,
                            revision: input.revision,
                            mode_revision: input.mode_revision,
                            policy: p::InputPolicy::HumanLive,
                            intent: input.intent,
                        });
                        if let Ok(response) = &result {
                            super::trace::emit(
                                serde_json::json!({"event":"input_result","local":input.trace,"request_id":response.id,"ok":response.ok,"state":response.receipt.as_ref().map(|r|&r.state),"delivered_bytes":response.receipt.as_ref().map(|r|r.delivered_bytes)}),
                            );
                        } else {
                            super::trace::emit(
                                serde_json::json!({"event":"input_transport_unknown","local":input.trace}),
                            );
                        }
                        match result {
                            Ok(r)
                                if r.ok
                                    && r.receipt.as_ref().is_some_and(|r| r.state == "applied") => {
                            }
                            Ok(r) => {
                                fence.advance();
                                notify(Update::Paused(format!(
                                    "Input refused/uncertain: {}. No retry.",
                                    r.error.unwrap_or_else(|| r
                                        .receipt
                                        .map_or("unknown".into(), |r| r.detail))
                                )));
                            }
                            Err(_) => {
                                fence.advance();
                                notify(Update::Paused(
                                    "Input transport lost; delivery may be unknown. No retry."
                                        .into(),
                                ));
                                notify(Update::Offline);
                                resume = Some((c.resume.clone(), c.next_id(), c.epoch.clone()));
                                client = None;
                            }
                        }
                    } else {
                        super::trace::emit(
                            serde_json::json!({"event":"input_discarded","local":input.trace,"focus_generation":input.focus,"current_focus_generation":fence.get()}),
                        );
                    }
                } else {
                    thread::sleep(Duration::from_millis(5));
                }
                if last.elapsed() >= Duration::from_millis(75) {
                    if let Some(c) = client.as_mut() {
                        match observe(c) {
                            Ok(s) => notify(Update::Snapshot(Box::new(s))),
                            Err(_) => {
                                fence.advance();
                                notify(Update::Paused(
                                    "Observation lost; current authority is unknown. No replay."
                                        .into(),
                                ));
                                notify(Update::Offline);
                                resume = Some((c.resume.clone(), c.next_id(), c.epoch.clone()));
                                client = None;
                            }
                        }
                    }
                    last = Instant::now();
                }
            }
        });
        Self {
            control: ct,
            input: it,
            fence: sender_fence,
        }
    }
    pub fn control(&self, value: Control, focus: u64) -> Result<(), &'static str> {
        self.control
            .try_send(Command { value, focus })
            .map_err(|_| {
                self.fence.advance();
                "Control queue unavailable/full; nothing retried"
            })
    }
    pub fn input(&self, value: Input) -> Result<(), &'static str> {
        self.input.try_send(value).map_err(|_| {
            self.fence.advance();
            "Input queue unavailable/full; input paused, no queued suffix replay"
        })
    }
}
// A failed Client exposes no retry permission. This probe only detects a failed
// transport; ordinary broker refusals do not require a replacement endpoint.
fn c_failed(c: &mut Client) -> bool {
    c.transport_failed()
}
fn observe(c: &mut Client) -> std::io::Result<Snapshot> {
    c.observe_checked()
}
fn control(
    c: &mut Client,
    op: Control,
    focus: u64,
    fence: &Fence,
    notify: &impl Fn(Update),
) -> std::io::Result<()> {
    let mut grant = None;
    let operation = match op {
        Control::NewShell => Op::CreateSession {
            name: "native-shell".into(),
        },
        Control::Take(session) => {
            let s = observe(c)?
                .sessions
                .into_iter()
                .find(|s| s.id == session)
                .ok_or_else(|| std::io::Error::other("selected session unavailable"))?;
            grant = Some(session);
            Op::TakeControl {
                session: s.id,
                generation: s.generation,
                force: true,
            }
        }
        Control::Release(session) => {
            let s = observe(c)?
                .sessions
                .into_iter()
                .find(|s| s.id == session)
                .ok_or_else(|| std::io::Error::other("selected session unavailable"))?;
            Op::ReleaseControl {
                session: s.id,
                generation: s.generation,
            }
        }
        Control::Resize(session, cols, rows) => {
            let s = observe(c)?
                .sessions
                .into_iter()
                .find(|s| s.id == session)
                .ok_or_else(|| std::io::Error::other("selected session unavailable"))?;
            Op::Resize {
                session: s.id,
                generation: s.generation,
                cols,
                rows,
            }
        }
        Control::Run(argv) => Op::Run {
            task: "manual".into(),
            argv,
            task_revision: None,
        },
        Control::Reconnect | Control::Demo => return Ok(()),
    };
    if !fence.valid(focus) {
        return Err(std::io::Error::other(
            "UI context changed before dispatch; nothing retried",
        ));
    }
    let r = c.call(operation)?;
    trace_receipt(&r);
    if !r.ok || r.receipt.as_ref().is_some_and(|r| r.state != "applied") {
        return Err(std::io::Error::other(r.error.unwrap_or_else(|| {
            r.receipt
                .map_or("unknown control result".into(), |r| r.detail)
        })));
    }
    notify(Update::Notice(
        r.receipt.map_or("Control applied".into(), |r| {
            format!("{}: {}", r.operation, r.detail)
        }),
    ));
    notify(Update::Snapshot(Box::new(observe(c)?)));
    if let Some(session) = grant {
        notify(Update::Granted { focus, session });
    }
    Ok(())
}
pub fn check_socket(path: &Path) -> std::io::Result<()> {
    if path.as_os_str().len() > 107 {
        return Err(std::io::Error::other(
            "Linux socket path too long; use the fresh launcher",
        ));
    }
    Ok(())
}

/// One explicit user resume may wait for the old handler to close. Only the
/// known-zero HELLO refusal is retried; never transport errors or effects.
fn explicit_connect(
    path: &Path,
    resume: Option<&(String, u64, String)>,
) -> std::io::Result<Client> {
    let Some((token, next, epoch)) = resume else {
        return Client::connect(path, "native-window", Role::Human);
    };
    let mut client = resume_wait(|| {
        Client::connect_resume(
            path,
            "native-window",
            Role::Human,
            Some(token.clone()),
            *next,
        )
    })?;
    if client.epoch != *epoch {
        client.close_transport();
        return Err(std::io::Error::other("broker_epoch_changed_no_restore"));
    }
    Ok(client)
}
fn resume_wait<T>(mut connect: impl FnMut() -> std::io::Result<T>) -> std::io::Result<T> {
    let deadline = Instant::now() + Duration::from_millis(250);
    loop {
        match connect() {
            Err(e)
                if e.to_string() == "endpoint_already_connected" && Instant::now() < deadline =>
            {
                thread::sleep(Duration::from_millis(10))
            }
            other => return other,
        }
    }
}

// Captureoff: only bounded receipt metadata, never argv/input/output/detail or resume.
fn trace_receipt(response: &p::Response) {
    if let Some(r) = &response.receipt {
        eprintln!(
            "{}",
            serde_json::json!({"event":"native_receipt","epoch":response.epoch,"endpoint":r.endpoint,"id":r.id,"operation":r.operation,"state":r.state,"delivered_bytes":r.delivered_bytes})
        );
    }
}
#[cfg(test)]
mod resume_tests {
    use super::*;
    #[test]
    fn explicit_resume_retries_only_known_zero_hello_refusal() {
        let mut count = 0;
        let result = resume_wait(|| {
            count += 1;
            if count < 3 {
                Err(std::io::Error::other("endpoint_already_connected"))
            } else {
                Ok(42)
            }
        })
        .unwrap();
        assert_eq!(result, 42);
        assert_eq!(count, 3);
    }
    #[test]
    fn unknown_transport_and_epoch_fail_without_retry() {
        for reason in ["transport_unknown", "resume_unknown_in_this_epoch"] {
            let mut count = 0;
            let result: std::io::Result<()> = resume_wait(|| {
                count += 1;
                Err(std::io::Error::other(reason))
            });
            assert!(result.is_err());
            assert_eq!(count, 1);
        }
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader, Write},
        os::unix::net::UnixListener,
        sync::{
            atomic::{AtomicUsize, Ordering},
            mpsc,
        },
    };
    static ID: AtomicUsize = AtomicUsize::new(0);
    #[derive(Clone, Copy)]
    enum ResultKind {
        Applied,
        Partial,
        DropTransport,
    }
    struct Fixture {
        connection: Option<Connection>,
        fence: Arc<Fence>,
        entered: mpsc::Receiver<()>,
        release: mpsc::Sender<()>,
        input_count: Arc<AtomicUsize>,
        thread: Option<thread::JoinHandle<()>>,
        path: PathBuf,
    }
    impl Fixture {
        fn new(outcome: ResultKind) -> Self {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.artifacts");
            std::fs::create_dir_all(&root).unwrap();
            let path = root.join(format!(
                "nc-{}-{}",
                std::process::id(),
                ID.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            let socket = path.join("s");
            let listener = UnixListener::bind(&socket).unwrap();
            let (entered, er) = mpsc::channel();
            let (release, rr) = mpsc::channel();
            let count = Arc::new(AtomicUsize::new(0));
            let seen = Arc::clone(&count);
            let server = thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut block = true;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        break;
                    }
                    let request: p::Request = serde_json::from_str(&line).unwrap();
                    let mut receipt = None;
                    match request.operation {
                        Op::Observe {} if block => {
                            block = false;
                            entered.send(()).unwrap();
                            rr.recv_timeout(Duration::from_secs(3)).unwrap();
                        }
                        Op::Input { .. } => {
                            seen.fetch_add(1, Ordering::SeqCst);
                            if matches!(outcome, ResultKind::DropTransport) {
                                break;
                            }
                            receipt = Some(p::Receipt {
                                endpoint: "human".into(),
                                id: request.id,
                                operation: "input".into(),
                                state: if matches!(outcome, ResultKind::Partial) {
                                    "partial"
                                } else {
                                    "applied"
                                }
                                .into(),
                                detail: "injected extent, not actual kernel delivery".into(),
                                delivered_bytes: 1,
                                result: None,
                            });
                        }
                        _ => {}
                    }
                    let snapshot = Snapshot {
                        version: p::VERSION.into(),
                        epoch: "epoch".into(),
                        endpoint: "human".into(),
                        endpoints: vec![],
                        capabilities: vec![],
                        tasks: vec![],
                        sessions: vec![],
                        attempts: vec![],
                        artifacts: vec![],
                        receipts: vec![],
                        degraded: None,
                    };
                    let response = p::Response {
                        id: request.id,
                        ok: true,
                        error: None,
                        receipt,
                        snapshot: Some(snapshot),
                        endpoint: Some("human".into()),
                        resume: Some("synthetic-ram-resume".into()),
                        epoch: "epoch".into(),
                    };
                    let mut data = serde_json::to_vec(&response).unwrap();
                    data.push(b'\n');
                    if stream.write_all(&data).is_err() {
                        break;
                    }
                }
            });
            let fence = Arc::new(Fence::default());
            let conn = Connection::start(socket, Arc::clone(&fence), |_| {});
            Self {
                connection: Some(conn),
                fence,
                entered: er,
                release,
                input_count: count,
                thread: Some(server),
                path,
            }
        }
        fn input(&self) -> Input {
            Input {
                session: "s".into(),
                generation: 1,
                revision: 1,
                mode_revision: 1,
                intent: p::Intent::Text { text: "x".into() },
                focus: self.fence.get(),
                trace: 0,
            }
        }
        fn wait_count(&self, n: usize) {
            let deadline = Instant::now() + Duration::from_secs(2);
            while self.input_count.load(Ordering::SeqCst) < n {
                assert!(Instant::now() < deadline);
                thread::sleep(Duration::from_millis(5));
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = self.release.send(());
            self.connection.take();
            if let Some(t) = self.thread.take() {
                t.join().unwrap();
            }
            std::fs::remove_file(self.path.join("s")).unwrap();
            std::fs::remove_dir(&self.path).unwrap();
        }
    }
    #[test]
    fn stale_queued_input_never_crosses_the_public_transport() {
        let f = Fixture::new(ResultKind::Applied);
        f.entered.recv_timeout(Duration::from_secs(2)).unwrap();
        f.connection.as_ref().unwrap().input(f.input()).unwrap();
        f.fence.advance();
        f.release.send(()).unwrap();
        thread::sleep(Duration::from_millis(100));
        assert_eq!(f.input_count.load(Ordering::SeqCst), 0);
        f.connection.as_ref().unwrap().input(f.input()).unwrap();
        f.wait_count(1);
    }
    #[test]
    fn input_queue_exhaustion_fences_all_undispatched_tails() {
        let f = Fixture::new(ResultKind::Applied);
        f.entered.recv_timeout(Duration::from_secs(2)).unwrap();
        for _ in 0..8 {
            f.connection.as_ref().unwrap().input(f.input()).unwrap();
        }
        let old = f.fence.get();
        assert!(f.connection.as_ref().unwrap().input(f.input()).is_err());
        assert!(!f.fence.valid(old));
        f.release.send(()).unwrap();
        thread::sleep(Duration::from_millis(150));
        assert_eq!(f.input_count.load(Ordering::SeqCst), 0);
        f.connection.as_ref().unwrap().input(f.input()).unwrap();
        f.wait_count(1);
    }
    #[test]
    fn transport_loss_drops_queued_suffix_without_input_retry() {
        let f = Fixture::new(ResultKind::DropTransport);
        f.entered.recv_timeout(Duration::from_secs(2)).unwrap();
        let old = f.fence.get();
        f.connection.as_ref().unwrap().input(f.input()).unwrap();
        f.connection.as_ref().unwrap().input(f.input()).unwrap();
        f.release.send(()).unwrap();
        f.wait_count(1);
        let deadline = Instant::now() + Duration::from_secs(2);
        while f.fence.valid(old) {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(f.input_count.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn partial_response_pauses_and_never_retries_or_sends_suffix() {
        let f = Fixture::new(ResultKind::Partial);
        f.entered.recv_timeout(Duration::from_secs(2)).unwrap();
        let old = f.fence.get();
        f.connection.as_ref().unwrap().input(f.input()).unwrap();
        f.connection.as_ref().unwrap().input(f.input()).unwrap();
        f.release.send(()).unwrap();
        f.wait_count(1);
        thread::sleep(Duration::from_millis(150));
        assert_eq!(f.input_count.load(Ordering::SeqCst), 1);
        assert!(!f.fence.valid(old));
    }
}
