use super::*;
use std::fs;
use std::time::Instant;

struct Harness {
    broker: Arc<Broker>,
    directory: PathBuf,
    human: String,
    agent: String,
    next: u64,
}
impl Harness {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("namtab-broker-test-{}", token().unwrap()));
        fs::create_dir(&directory).unwrap();
        let target = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_owned();
        let worker = target.join("namtab-terminal-worker");
        let pty_helper = target.join("namtab-pty-helper");
        assert!(
            worker.is_file() && pty_helper.is_file(),
            "build the real worker and PTY helper before broker integration tests"
        );
        let broker = Broker::new(
            Config {
                socket: directory.join("socket"),
                workspace: directory.clone(),
                worker,
                pty_helper,
                terminfo: None,
            },
            &directory.join("journal"),
        )
        .unwrap();
        let (_, human) = broker.hello(1, Role::Human, None);
        let (_, agent) = broker.hello(1, Role::Agent, None);
        Self {
            broker,
            directory,
            human: human.unwrap(),
            agent: agent.unwrap(),
            next: 10,
        }
    }
    fn request(&mut self, endpoint: &str, operation: Op) -> Response {
        let id = self.next;
        self.next += 1;
        self.broker.request(
            endpoint,
            Request {
                version: wire::VERSION.into(),
                id,
                operation,
            },
        )
    }
    fn human(&mut self, operation: Op) -> Response {
        self.request(&self.human.clone(), operation)
    }
    fn wait(&self, predicate: impl Fn(&Snapshot) -> bool) -> Snapshot {
        let start = Instant::now();
        loop {
            let snapshot = self.broker.snapshot(&self.human).unwrap();
            if predicate(&snapshot) {
                return snapshot;
            }
            assert!(
                start.elapsed() < Duration::from_secs(4),
                "timed out: {snapshot:?}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn shell(&mut self) -> Session {
        assert!(
            self.human(Op::CreateSession {
                name: "real shell".into()
            })
            .ok
        );
        self.wait(|snapshot| {
            snapshot.sessions[0]
                .screen
                .lines
                .iter()
                .any(|line| line.contains("mk1$"))
        });
        let session = self.broker.snapshot(&self.human).unwrap().sessions[0].clone();
        assert!(
            self.human(Op::TakeControl {
                session: session.id.clone(),
                generation: session.generation,
                force: false
            })
            .ok
        );
        self.broker.snapshot(&self.human).unwrap().sessions[0].clone()
    }
}
impl Drop for Harness {
    fn drop(&mut self) {
        self.broker.shutdown();
        std::thread::sleep(Duration::from_millis(130));
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[test]
fn real_shell_typed_input_takeover_stale_refusal_and_no_secret_journal() {
    let mut h = Harness::new();
    let initial = h.shell();
    let stale = h.request(
        &h.agent.clone(),
        Op::TakeControl {
            session: initial.id.clone(),
            generation: initial.generation,
            force: true,
        },
    );
    assert_eq!(stale.error.as_deref(), Some("agent_cannot_force_takeover"));
    let text = "printf MK1_REAL_CHILD";
    let response = h.human(Op::Input {
        session: initial.id.clone(),
        generation: initial.generation,
        revision: initial.screen.revision,
        mode_revision: initial.screen.mode_revision,
        policy: InputPolicy::HumanLive,
        intent: Intent::Text { text: text.into() },
    });
    assert!(response.ok, "{response:?}");
    assert_eq!(response.receipt.unwrap().delivered_bytes, text.len());
    h.wait(|snapshot| {
        snapshot.sessions[0]
            .screen
            .lines
            .iter()
            .any(|line| line.contains(text))
    });
    let current = h.broker.snapshot(&h.human).unwrap().sessions[0].clone();
    let refused = h.human(Op::Input {
        session: current.id.clone(),
        generation: current.generation,
        revision: initial.screen.revision,
        mode_revision: current.screen.mode_revision,
        policy: InputPolicy::Observed,
        intent: Intent::Key {
            key: "Enter".into(),
            ctrl: false,
            alt: false,
            shift: false,
        },
    });
    assert_eq!(refused.error.as_deref(), Some("screen_revision_stale"));
    let enter = h.human(Op::Input {
        session: current.id.clone(),
        generation: current.generation,
        revision: current.screen.revision,
        mode_revision: current.screen.mode_revision,
        policy: InputPolicy::HumanLive,
        intent: Intent::Key {
            key: "Enter".into(),
            ctrl: false,
            alt: false,
            shift: false,
        },
    });
    assert!(enter.ok, "{enter:?}");
    h.wait(|snapshot| {
        snapshot.sessions[0]
            .screen
            .lines
            .iter()
            .filter(|line| line.contains("MK1_REAL_CHILD"))
            .count()
            >= 2
    });
    let agent_live = h.request(
        &h.agent.clone(),
        Op::Input {
            session: current.id,
            generation: current.generation,
            revision: current.screen.revision,
            mode_revision: current.screen.mode_revision,
            policy: InputPolicy::HumanLive,
            intent: Intent::Text {
                text: "DO_NOT_JOURNAL_INPUT".into(),
            },
        },
    );
    assert!(!agent_live.ok);
    let journal = fs::read_to_string(h.directory.join("journal")).unwrap();
    assert!(!journal.contains(text));
    assert!(!journal.contains("DO_NOT_JOURNAL_INPUT"));
    assert!(!journal.contains("MK1_REAL_CHILD"));
}

#[test]
fn duplicate_mutation_never_reexecutes_and_stream_drain_is_distinct() {
    let mut h = Harness::new();
    let operation = Op::Run { task: "attributed".into(), argv: vec!["/bin/sh".into(), "-c".into(), "printf once >> counter; printf stdout; printf stderr >&2; (sleep 0.25; printf tail)& exit 0".into()] };
    let request = Request {
        version: wire::VERSION.into(),
        id: 4,
        operation: operation.clone(),
    };
    let first = h.broker.request(&h.agent, request.clone());
    assert!(first.ok, "{first:?}");
    let duplicate = h.broker.request(&h.agent, request);
    assert_eq!(duplicate.receipt, first.receipt);
    let conflict = h.broker.request(
        &h.agent,
        Request {
            version: wire::VERSION.into(),
            id: 4,
            operation: Op::Run {
                task: "changed".into(),
                argv: vec!["/bin/true".into()],
            },
        },
    );
    assert_eq!(conflict.error.as_deref(), Some("mutation_id_conflict"));
    h.wait(|snapshot| snapshot.attempts[0].state == "draining");
    let snapshot = h.wait(|snapshot| snapshot.attempts[0].state == "exited");
    assert_eq!(snapshot.attempts[0].exit_code, Some(0));
    assert_eq!(snapshot.attempts[0].stdout, "stdouttail");
    assert_eq!(snapshot.attempts[0].stderr, "stderr");
    assert_eq!(
        fs::read_to_string(h.directory.join("counter")).unwrap(),
        "once"
    );
    assert!(
        h.human(Op::Run {
            task: "bounded flood".into(),
            argv: vec![
                "/bin/sh".into(),
                "-c".into(),
                "head -c 70000 /dev/zero; printf drained >&2".into()
            ]
        })
        .ok
    );
    let snapshot = h.wait(|snapshot| {
        snapshot
            .attempts
            .iter()
            .all(|attempt| attempt.state == "exited")
    });
    let flood = snapshot
        .attempts
        .iter()
        .find(|attempt| attempt.task == "bounded flood")
        .unwrap();
    assert!(flood.truncated);
    assert!(flood.stdout.len() <= 65_536);
    assert!(!flood.stdout.contains('\0'));
    assert_eq!(flood.stderr, "drained");
    assert_eq!(
        h.human(Op::Run {
            task: "third".into(),
            argv: vec!["/bin/true".into()]
        })
        .error
        .as_deref(),
        Some("attempt_capacity_two_total")
    );
}

#[test]
fn live_resume_is_endpoint_bound_and_old_epoch_tokens_fail() {
    let h = Harness::new();
    let token = h.broker.state.lock().unwrap().endpoints[&h.human]
        .resume
        .clone();
    assert!(!h.broker.hello(2, Role::Human, Some(token.clone())).0.ok);
    h.broker.disconnect(&h.human);
    assert!(!h.broker.hello(2, Role::Agent, Some(token.clone())).0.ok);
    let (response, endpoint) = h.broker.hello(2, Role::Human, Some(token.clone()));
    assert!(response.ok);
    assert_eq!(endpoint.as_deref(), Some(h.human.as_str()));
    let other = Harness::new();
    assert!(!other.broker.hello(2, Role::Human, Some(token)).0.ok);
    assert!(h.broker.hello(3, Role::Human, None).0.ok);
    assert!(h.broker.hello(3, Role::Human, None).0.ok);
    assert_eq!(
        h.broker.hello(3, Role::Human, None).0.error.as_deref(),
        Some("endpoint_capacity")
    );
}

#[test]
fn explicit_artifact_revision_changes_make_validation_stale() {
    let mut h = Harness::new();
    fs::write(h.directory.join("artifact.txt"), "first").unwrap();
    assert!(
        h.human(Op::Artifact {
            task: "review".into(),
            path: "artifact.txt".into()
        })
        .ok
    );
    let first = h.broker.snapshot(&h.human).unwrap().artifacts[0].clone();
    let denied_agent_review = h.request(
        &h.agent.clone(),
        Op::ValidateArtifact {
            artifact: first.id.clone(),
            revision: first.revision.clone(),
        },
    );
    assert_eq!(
        denied_agent_review.error.as_deref(),
        Some("artifact_review_requires_human_endpoint")
    );
    assert!(
        h.human(Op::ValidateArtifact {
            artifact: first.id.clone(),
            revision: first.revision.clone()
        })
        .ok
    );
    fs::write(h.directory.join("artifact.txt"), "second").unwrap();
    let changed = h.broker.snapshot(&h.human).unwrap().artifacts[0].clone();
    assert!(changed.stale);
    assert_ne!(changed.revision, first.revision);
    assert_eq!(
        h.human(Op::ValidateArtifact {
            artifact: first.id,
            revision: first.revision
        })
        .error
        .as_deref(),
        Some("artifact_revision_stale")
    );
    assert!(
        !h.human(Op::Artifact {
            task: "escape".into(),
            path: "/etc/passwd".into()
        })
        .ok
    );
    fs::write(h.directory.join("too-large"), vec![0; 1_048_577]).unwrap();
    assert_eq!(
        h.human(Op::Artifact {
            task: "oversize".into(),
            path: "too-large".into()
        })
        .error
        .as_deref(),
        Some("artifact_size")
    );
}

#[test]
fn worker_loss_fences_input_but_preserves_independent_process_facts() {
    let mut h = Harness::new();
    assert!(h.broker.snapshot(&h.human).unwrap().degraded.is_none());
    h.shell();
    h.broker.worker.stop();
    h.broker.fence_worker();
    let snapshot = h.broker.snapshot(&h.human).unwrap();
    assert!(snapshot.degraded.is_some());
    assert!(!snapshot.sessions[0].screen.input_known);
    assert!(snapshot.sessions[0].owner.is_none());
    assert_eq!(snapshot.sessions[0].process, "running");
    let protected = h.human(Op::TakeControl {
        session: snapshot.sessions[0].id.clone(),
        generation: snapshot.sessions[0].generation,
        force: true,
    });
    assert!(!protected.ok);
    assert_eq!(protected.receipt.unwrap().state, "unknown");
}

#[test]
fn human_takeover_does_not_wait_for_worker_and_final_write_rechecks_owner() {
    let mut h = Harness::new();
    let session = h.shell();
    let (_, second_human) = h.broker.hello(1, Role::Human, None);
    let second_human = second_human.unwrap();
    let pid = h.broker.worker.process_id().to_string();
    assert!(
        std::process::Command::new("/bin/kill")
            .args(["-STOP", &pid])
            .status()
            .unwrap()
            .success()
    );
    let broker = Arc::clone(&h.broker);
    let endpoint = h.human.clone();
    let input_session = session.clone();
    let input = std::thread::spawn(move || {
        broker.request(
            &endpoint,
            Request {
                version: wire::VERSION.into(),
                id: 100,
                operation: Op::Input {
                    session: input_session.id,
                    generation: input_session.generation,
                    revision: input_session.screen.revision,
                    mode_revision: input_session.screen.mode_revision,
                    policy: InputPolicy::HumanLive,
                    intent: Intent::Text {
                        text: "MUST_NOT_BE_WRITTEN".into(),
                    },
                },
            },
        )
    });
    let start = Instant::now();
    while h.broker.worker.outstanding() == 0 {
        assert!(start.elapsed() < Duration::from_secs(1));
        std::thread::sleep(Duration::from_millis(1));
    }
    let broker = Arc::clone(&h.broker);
    let session_id = session.id.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let response = broker.request(
            &second_human,
            Request {
                version: wire::VERSION.into(),
                id: 2,
                operation: Op::TakeControl {
                    session: session_id,
                    generation: session.generation,
                    force: true,
                },
            },
        );
        tx.send(response).unwrap();
    });
    let takeover = rx.recv_timeout(Duration::from_millis(750));
    assert!(
        std::process::Command::new("/bin/kill")
            .args(["-CONT", &pid])
            .status()
            .unwrap()
            .success()
    );
    assert!(takeover.expect("takeover blocked behind worker RPC").ok);
    let response = input.join().unwrap();
    assert!(!response.ok);
    assert_eq!(response.receipt.unwrap().delivered_bytes, 0);
    assert!(
        !h.broker.snapshot(&h.human).unwrap().sessions[0]
            .screen
            .lines
            .iter()
            .any(|line| line.contains("MUST_NOT_BE_WRITTEN"))
    );
}

#[test]
fn receipts_remain_queryable_while_snapshot_is_bounded_and_cap_refuses_before_effect() {
    let mut h = Harness::new();
    for _ in 0..20 {
        assert!(
            !h.human(Op::TakeControl {
                session: "missing".into(),
                generation: 1,
                force: false
            })
            .ok
        );
    }
    let snapshot = h.broker.snapshot(&h.human).unwrap();
    assert_eq!(snapshot.receipts.len(), 16);
    let old = h.human(Op::Receipt { request_id: 10 });
    assert_eq!(old.receipt.unwrap().id, 10);
    {
        let mut state = h.broker.state.lock().unwrap();
        let endpoint = state.endpoints.get_mut(&h.human).unwrap();
        for id in 100..4176 {
            endpoint.decisions.insert(
                id,
                Decision {
                    binding: [0; 32],
                    receipt: Receipt {
                        endpoint: h.human.clone(),
                        id,
                        operation: "input".into(),
                        state: "unknown".into(),
                        detail: "synthetic retained capacity setup".into(),
                        delivered_bytes: 0,
                    },
                },
            );
        }
        endpoint.high_water = 4175;
    }
    let response = h.broker.request(
        &h.human,
        Request {
            version: wire::VERSION.into(),
            id: 5000,
            operation: Op::Run {
                task: "must not launch".into(),
                argv: vec!["/bin/sh".into(), "-c".into(), "touch MUST_NOT_EXIST".into()],
            },
        },
    );
    assert_eq!(
        response.error.as_deref(),
        Some("receipt_capacity_no_eviction")
    );
    assert!(!h.directory.join("MUST_NOT_EXIST").exists());
}

#[test]
fn raw_public_socket_exchange_binds_endpoint_and_rejects_untyped_operations() {
    use std::io::{BufReader, Write};
    use std::os::unix::net::UnixStream;
    let h = Harness::new();
    let (mut client, server) = UnixStream::pair().unwrap();
    assert_eq!(
        platform::peer_uid(&server).unwrap(),
        platform::effective_uid()
    );
    let broker = Arc::clone(&h.broker);
    let thread = std::thread::spawn(move || server::connection(server, broker));
    let mut reader = BufReader::new(client.try_clone().unwrap());
    let mut exchange = |body: &[u8]| {
        client.write_all(body).unwrap();
        client.write_all(b"\n").unwrap();
        let body = worker::line(&mut reader, wire::MAX_FRAME).unwrap().unwrap();
        serde_json::from_slice::<Response>(&body).unwrap()
    };
    let hello = Request {
        version: wire::VERSION.into(),
        id: 1,
        operation: Op::Hello {
            label: "socket".into(),
            role: Role::Human,
            resume: None,
        },
    };
    assert!(exchange(&serde_json::to_vec(&hello).unwrap()).ok);
    let malformed =
        exchange(br#"{"version":"namtab.mk1/1","id":2,"operation":{"op":"observe","extra":true}}"#);
    assert!(!malformed.ok);
    let observe = Request {
        version: wire::VERSION.into(),
        id: 3,
        operation: Op::Observe {},
    };
    let response = exchange(&serde_json::to_vec(&observe).unwrap());
    assert!(response.ok);
    assert!(response.snapshot.unwrap().attempts.is_empty());
    drop(reader);
    drop(client);
    thread.join().unwrap();
}

#[test]
fn real_nonreading_pty_partial_write_reports_prefix_and_never_redispatches() {
    let h = Harness::new();
    let id = "partial_fixture".to_owned();
    let Reply::Frame(frame) = h
        .broker
        .worker_call(Operation::New {
            session: id.clone(),
            cols: 80,
            rows: 24,
        })
        .unwrap()
    else {
        panic!("frame")
    };
    let spec = h.broker.spec(
        "/bin/sh".into(),
        vec![
            "-c".into(),
            "/usr/bin/stty raw -echo; printf RAW_READY; exec /bin/sleep 10".into(),
        ],
    );
    let mut process = platform::launch_pty(
        &h.broker.config.pty_helper,
        &spec,
        Geometry { cols: 80, rows: 24 },
    )
    .unwrap();
    let mut ready = Vec::new();
    let started = Instant::now();
    while !String::from_utf8_lossy(&ready).contains("RAW_READY") {
        assert!(started.elapsed() < Duration::from_secs(2));
        let mut bytes = [0; 512];
        if let ReadOutcome::Data(n) = process
            .reader
            .read_chunk(&mut bytes, Duration::from_millis(50))
            .unwrap()
        {
            ready.extend_from_slice(&bytes[..n]);
        }
    }
    let projection = screen(frame);
    let revision = projection.revision;
    let mode_revision = projection.mode_revision;
    h.broker.state.lock().unwrap().sessions.insert(
        id.clone(),
        LiveSession {
            view: Session {
                id: id.clone(),
                name: "nonreading PTY fault fixture".into(),
                owner: Some(h.human.clone()),
                generation: 1,
                process: "running".into(),
                exit_code: None,
                screen: projection,
            },
            writer: process.writer,
            pending_output: 0,
            exited: false,
            input_uncertain: false,
        },
    );
    let mut partial = None;
    for request_id in 10..74 {
        let request = Request {
            version: wire::VERSION.into(),
            id: request_id,
            operation: Op::Input {
                session: id.clone(),
                generation: 1,
                revision,
                mode_revision,
                policy: InputPolicy::HumanLive,
                intent: Intent::Text {
                    text: "x".repeat(8192),
                },
            },
        };
        let response = h.broker.request(&h.human, request.clone());
        let receipt = response.receipt.clone().unwrap();
        if receipt.state != "applied" {
            assert!(matches!(receipt.state.as_str(), "partial" | "rejected"));
            assert!(receipt.delivered_bytes < 8192);
            assert!(!response.ok);
            let duplicate = h.broker.request(&h.human, request);
            assert_eq!(duplicate.receipt, Some(receipt.clone()));
            partial = Some(receipt);
            break;
        }
    }
    assert!(
        partial.is_some(),
        "real PTY did not reach bounded partial/no-progress boundary"
    );
    let Reply::Fed { frame, .. } = h
        .broker
        .worker_call(Operation::Feed {
            session: id.clone(),
            bytes: b"later output".to_vec(),
        })
        .unwrap()
    else {
        panic!("frame")
    };
    h.broker
        .state
        .lock()
        .unwrap()
        .sessions
        .get_mut(&id)
        .unwrap()
        .publish(frame);

    assert!(
        !h.broker.snapshot(&h.human).unwrap().sessions[0]
            .screen
            .input_known
    );
    let _ = process.process.kill_and_wait();
}

#[test]
fn real_pty_eof_finishes_utf8_prefix_without_inventing_child_exit() {
    let mut h = Harness::new();
    let session = h.shell();
    let text = "printf '\\303'; exit 7";
    let response = h.human(Op::Input {
        session: session.id.clone(),
        generation: session.generation,
        revision: session.screen.revision,
        mode_revision: session.screen.mode_revision,
        policy: InputPolicy::HumanLive,
        intent: Intent::Text { text: text.into() },
    });
    assert!(response.ok, "{response:?}");
    h.wait(|snapshot| {
        snapshot.sessions[0]
            .screen
            .lines
            .iter()
            .any(|line| line.contains(text))
    });
    let session = h.broker.snapshot(&h.human).unwrap().sessions[0].clone();
    let response = h.human(Op::Input {
        session: session.id.clone(),
        generation: session.generation,
        revision: session.screen.revision,
        mode_revision: session.screen.mode_revision,
        policy: InputPolicy::HumanLive,
        intent: Intent::Key {
            key: "Enter".into(),
            ctrl: false,
            alt: false,
            shift: false,
        },
    });
    assert!(response.ok, "{response:?}");
    let snapshot = h.wait(|snapshot| snapshot.sessions[0].process == "exited");
    let session = &snapshot.sessions[0];
    assert_eq!(session.exit_code, Some(7));
    assert!(!session.screen.input_known);
    assert!(
        session
            .screen
            .lines
            .iter()
            .any(|line| line.contains('\u{fffd}'))
    );
}

#[test]
fn full_journal_still_allows_protective_human_revoke_but_never_a_new_grant() {
    let mut h = Harness::new();
    let session = h.shell();
    assert!(
        h.human(Op::ReleaseControl {
            session: session.id.clone(),
            generation: session.generation
        })
        .ok
    );
    let session = h.broker.snapshot(&h.human).unwrap().sessions[0].clone();
    assert!(
        h.request(
            &h.agent.clone(),
            Op::TakeControl {
                session: session.id.clone(),
                generation: session.generation,
                force: false
            }
        )
        .ok
    );
    let session = h.broker.snapshot(&h.human).unwrap().sessions[0].clone();
    assert_eq!(session.owner.as_deref(), Some(h.agent.as_str()));
    h.broker.state.lock().unwrap().journal.exhaust_for_test();
    let request = Request {
        version: wire::VERSION.into(),
        id: 100,
        operation: Op::TakeControl {
            session: session.id.clone(),
            generation: session.generation,
            force: true,
        },
    };
    let response = h.broker.request(&h.human, request.clone());
    assert!(!response.ok);
    let receipt = response.receipt.clone().unwrap();
    assert_eq!(receipt.state, "unknown");
    assert_eq!(receipt.delivered_bytes, 0);
    let protected = h.broker.snapshot(&h.human).unwrap().sessions[0].clone();
    assert!(protected.owner.is_none());
    assert_eq!(protected.generation, session.generation + 1);
    assert_eq!(protected.process, "running");
    assert_eq!(
        h.broker.request(&h.human, request.clone()).receipt,
        Some(receipt)
    );
    assert_eq!(
        h.broker.snapshot(&h.human).unwrap().sessions[0].generation,
        protected.generation
    );
    let conflict = Request {
        operation: Op::TakeControl {
            session: session.id.clone(),
            generation: protected.generation,
            force: true,
        },
        ..request
    };
    assert_eq!(
        h.broker.request(&h.human, conflict).error.as_deref(),
        Some("mutation_id_conflict")
    );
    let denied = h.broker.request(
        &h.agent,
        Request {
            version: wire::VERSION.into(),
            id: 101,
            operation: Op::TakeControl {
                session: session.id,
                generation: protected.generation,
                force: false,
            },
        },
    );
    assert!(!denied.ok);
    assert!(
        h.broker.snapshot(&h.human).unwrap().sessions[0]
            .owner
            .is_none()
    );
}

fn typed_input_request(id: u64, session: &Session, text: &str, policy: InputPolicy) -> Request {
    Request {
        version: wire::VERSION.into(),
        id,
        operation: Op::Input {
            session: session.id.clone(),
            generation: session.generation,
            revision: session.screen.revision,
            mode_revision: session.screen.mode_revision,
            policy,
            intent: Intent::Text { text: text.into() },
        },
    }
}
fn wait_for_human_waiter(broker: &Broker) {
    let start = Instant::now();
    while broker.human_waiters.load(Ordering::Acquire) == 0 {
        assert!(
            start.elapsed() < Duration::from_millis(200),
            "human input did not enter pending-output wait"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn human_live_waits_for_real_pending_echo_and_delivers_new_key_once() {
    let mut h = Harness::new();
    let session = h.shell();
    let pid = h.broker.worker.process_id().to_string();
    assert!(
        std::process::Command::new("/bin/kill")
            .args(["-STOP", &pid])
            .status()
            .unwrap()
            .success()
    );
    assert_eq!(
        h.broker
            .state
            .lock()
            .unwrap()
            .sessions
            .get_mut(&session.id)
            .unwrap()
            .writer
            .write_once(b"x")
            .unwrap(),
        1
    );
    let start = Instant::now();
    while h.broker.state.lock().unwrap().sessions[&session.id].pending_output == 0 {
        assert!(start.elapsed() < Duration::from_secs(1));
        std::thread::sleep(Duration::from_millis(1));
    }
    let request = typed_input_request(100, &session, "y", InputPolicy::HumanLive);
    let duplicate = request.clone();
    let endpoint = h.human.clone();
    let broker = Arc::clone(&h.broker);
    let input = std::thread::spawn(move || broker.request(&endpoint, request));
    wait_for_human_waiter(&h.broker);
    assert!(
        std::process::Command::new("/bin/kill")
            .args(["-CONT", &pid])
            .status()
            .unwrap()
            .success()
    );
    let response = input.join().unwrap();
    assert!(response.ok, "{response:?}");
    assert_eq!(response.receipt.as_ref().unwrap().delivered_bytes, 1);
    let snapshot = h.wait(|snapshot| {
        snapshot.sessions[0]
            .screen
            .lines
            .iter()
            .any(|line| line.trim_end().ends_with("xy"))
    });
    assert!(
        snapshot.sessions[0]
            .screen
            .lines
            .iter()
            .all(|line| !line.contains("xyy"))
    );
    assert_eq!(
        h.broker.request(&h.human, duplicate).receipt,
        response.receipt
    );
}
#[test]
fn mode_change_during_human_pending_wait_refuses_before_first_write() {
    let mut h = Harness::new();
    let session = h.shell();
    h.broker
        .state
        .lock()
        .unwrap()
        .sessions
        .get_mut(&session.id)
        .unwrap()
        .pending_output += 1;
    let request = typed_input_request(100, &session, "SHOULD_NOT_APPEAR", InputPolicy::HumanLive);
    let endpoint = h.human.clone();
    let broker = Arc::clone(&h.broker);
    let input = std::thread::spawn(move || broker.request(&endpoint, request));
    wait_for_human_waiter(&h.broker);
    let Reply::Fed { frame, .. } = h
        .broker
        .worker_call(Operation::Feed {
            session: session.id.clone(),
            bytes: b"\x1b[?1h".to_vec(),
        })
        .unwrap()
    else {
        panic!("frame")
    };
    {
        let mut state = h.broker.state.lock().unwrap();
        let value = state.sessions.get_mut(&session.id).unwrap();
        value.publish(frame);
        value.pending_output -= 1;
        h.broker.changed.notify_all();
    }
    let response = input.join().unwrap();
    assert!(!response.ok);
    assert_eq!(response.error.as_deref(), Some("mode_revision_stale"));
    assert_eq!(response.receipt.unwrap().delivered_bytes, 0);
    assert!(
        h.broker.snapshot(&h.human).unwrap().sessions[0]
            .screen
            .lines
            .iter()
            .all(|line| !line.contains("SHOULD_NOT_APPEAR"))
    );
}
#[test]
fn takeover_during_human_pending_wait_refuses_old_writer_before_first_write() {
    let mut h = Harness::new();
    let session = h.shell();
    let (_, replacement) = h.broker.hello(1, Role::Human, None);
    let replacement = replacement.unwrap();
    h.broker
        .state
        .lock()
        .unwrap()
        .sessions
        .get_mut(&session.id)
        .unwrap()
        .pending_output += 1;
    let request = typed_input_request(100, &session, "SHOULD_NOT_APPEAR", InputPolicy::HumanLive);
    let endpoint = h.human.clone();
    let broker = Arc::clone(&h.broker);
    let input = std::thread::spawn(move || broker.request(&endpoint, request));
    wait_for_human_waiter(&h.broker);
    let takeover = h.broker.request(
        &replacement,
        Request {
            version: wire::VERSION.into(),
            id: 2,
            operation: Op::TakeControl {
                session: session.id.clone(),
                generation: session.generation,
                force: true,
            },
        },
    );
    assert!(takeover.ok);
    let response = input.join().unwrap();
    assert!(!response.ok);
    assert_eq!(
        response.error.as_deref(),
        Some("writer_or_generation_stale")
    );
    assert_eq!(response.receipt.unwrap().delivered_bytes, 0);
    assert_eq!(
        h.broker.snapshot(&h.human).unwrap().sessions[0]
            .owner
            .as_deref(),
        Some(replacement.as_str())
    );
}
#[test]
fn agent_observed_input_keeps_immediate_pending_output_refusal() {
    let mut h = Harness::new();
    let session = h.shell();
    assert!(
        h.human(Op::ReleaseControl {
            session: session.id.clone(),
            generation: session.generation
        })
        .ok
    );
    let session = h.broker.snapshot(&h.human).unwrap().sessions[0].clone();
    assert!(
        h.request(
            &h.agent.clone(),
            Op::TakeControl {
                session: session.id.clone(),
                generation: session.generation,
                force: false
            }
        )
        .ok
    );
    let session = h.broker.snapshot(&h.human).unwrap().sessions[0].clone();
    h.broker
        .state
        .lock()
        .unwrap()
        .sessions
        .get_mut(&session.id)
        .unwrap()
        .pending_output += 1;
    let response = h.broker.request(
        &h.agent,
        typed_input_request(100, &session, "SHOULD_NOT_APPEAR", InputPolicy::Observed),
    );
    assert!(!response.ok);
    assert_eq!(
        response.error.as_deref(),
        Some("canonical_output_pending_observe_again")
    );
    assert_eq!(response.receipt.unwrap().delivered_bytes, 0);
    assert_eq!(h.broker.human_waiters.load(Ordering::Acquire), 0);
    assert_eq!(h.broker.worker.outstanding(), 0);
}
