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
    let operation = Op::Run { task_revision: None, task: "attributed".into(), argv: vec!["/bin/sh".into(), "-c".into(), "printf once >> counter; printf stdout; printf stderr >&2; (sleep 0.25; printf tail)& exit 0".into()] };
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
                task_revision: None,
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
            task_revision: None,
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
            task_revision: None,
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
    for _ in 0..13 {
        let (response, endpoint) = h.broker.hello(3, Role::Human, None);
        assert!(response.ok);
        h.broker.disconnect(&endpoint.unwrap());
    }
    h.broker.lifecycle_hello(1);
    h.broker.disconnect("lifecycle");
    assert_eq!(h.broker.snapshot(&h.human).unwrap().endpoints.len(), 16);
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
                        result: None,
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
                task_revision: None,
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
    let hello_response = exchange(&serde_json::to_vec(&hello).unwrap());
    assert!(hello_response.ok);
    let connected_endpoint = hello_response.endpoint.unwrap();
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
    let snapshot = response.snapshot.unwrap();
    assert!(snapshot.attempts.is_empty());
    let endpoint = snapshot
        .endpoints
        .iter()
        .find(|value| value.id == connected_endpoint)
        .unwrap();
    assert!(endpoint.connected);
    assert_eq!(endpoint.role, Role::Human);
    let serialized = serde_json::to_string(&snapshot.endpoints).unwrap();
    assert!(!serialized.contains("resume"));
    assert!(!serialized.contains("label"));
    drop(reader);
    drop(client);
    thread.join().unwrap();
    assert!(
        !h.broker
            .snapshot(&h.human)
            .unwrap()
            .endpoints
            .iter()
            .find(|value| value.id == connected_endpoint)
            .unwrap()
            .connected
    );
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

#[test]
fn repeated_socket_launches_preserve_ordinary_and_reserved_lifecycle_identities() {
    use std::io::{BufReader, Write};
    use std::os::unix::net::UnixStream;
    let mut h = Harness::new();
    let owned = h.shell();
    let old_token = h.broker.state.lock().unwrap().endpoints[&h.human]
        .resume
        .clone();
    h.broker.disconnect(&h.human);
    let exchange_connection = |role: Role, resume: Option<String>, read_receipt: bool| {
        let (mut client, server) = UnixStream::pair().unwrap();
        let broker = Arc::clone(&h.broker);
        let thread = std::thread::spawn(move || server::connection(server, broker));
        let mut reader = BufReader::new(client.try_clone().unwrap());
        let mut call = |id, operation| {
            let request = Request {
                version: wire::VERSION.into(),
                id,
                operation,
            };
            let mut body = serde_json::to_vec(&request).unwrap();
            body.push(b'\n');
            client.write_all(&body).unwrap();
            serde_json::from_slice::<Response>(
                &worker::line(&mut reader, wire::MAX_FRAME).unwrap().unwrap(),
            )
            .unwrap()
        };
        let hello = call(
            1,
            Op::Hello {
                label: "repeated local client".into(),
                role: role.clone(),
                resume,
            },
        );
        if hello.ok {
            let snapshot = call(2, Op::Observe {}).snapshot.unwrap();
            let own = snapshot
                .endpoints
                .iter()
                .find(|endpoint| Some(&endpoint.id) == hello.endpoint.as_ref())
                .unwrap();
            assert!(own.connected);
            assert_eq!(own.role, role);
            if read_receipt {
                assert_eq!(
                    call(3, Op::Receipt { request_id: 10 })
                        .receipt
                        .unwrap()
                        .operation,
                    "create_session"
                );
            }
        }
        drop(reader);
        drop(client);
        thread.join().unwrap();
        hello
    };
    for _ in 0..13 {
        let hello = exchange_connection(Role::Agent, None, false);
        assert!(hello.ok);
        let endpoint = hello.endpoint.unwrap();
        assert!(
            !h.broker
                .snapshot(&h.human)
                .unwrap()
                .endpoints
                .iter()
                .find(|value| value.id == endpoint)
                .unwrap()
                .connected
        );
    }
    h.broker.lifecycle_hello(1);
    h.broker.disconnect("lifecycle");
    let snapshot = h.broker.snapshot(&h.human).unwrap();
    assert_eq!(snapshot.endpoints.len(), 16);
    assert_eq!(snapshot.sessions[0].owner, owned.owner);
    assert_eq!(
        exchange_connection(Role::Agent, None, false)
            .error
            .as_deref(),
        Some("endpoint_capacity")
    );
    let resumed = exchange_connection(Role::Human, Some(old_token), true);
    assert!(resumed.ok);
    assert_eq!(resumed.endpoint.as_deref(), Some(h.human.as_str()));
    h.broker.lifecycle_hello(1);
    h.broker.disconnect("lifecycle");
    assert_eq!(h.broker.snapshot(&h.human).unwrap().endpoints.len(), 16);
}

fn defined_revision(response: &Response, task: &str) -> String {
    assert!(response.ok, "{response:?}");
    let Some(MutationResult::TaskDefined {
        task: actual,
        revision,
    }) = response.receipt.as_ref().unwrap().result.as_ref()
    else {
        panic!("missing authoritative task result: {response:?}")
    };
    assert_eq!(actual, task);
    revision.clone()
}
fn started_attempt(response: &Response) -> String {
    assert!(response.ok, "{response:?}");
    let Some(MutationResult::AttemptStarted { attempt }) =
        response.receipt.as_ref().unwrap().result.as_ref()
    else {
        panic!("missing authoritative attempt result: {response:?}")
    };
    attempt.clone()
}
#[test]
fn two_agents_share_one_immutable_contract_with_exact_receipt_and_launch_attribution() {
    let h = Harness::new();
    let (_, second) = h.broker.hello(1, Role::Agent, None);
    let second = second.unwrap();
    let definition = Request {
        version: wire::VERSION.into(),
        id: 10,
        operation: Op::DefineTask {
            task: "shared".into(),
            goal: "GOAL_RAM_ONLY_two_independent_agents".into(),
            expected_artifacts: vec!["EXPECTED_RAM_ONLY/result.txt".into()],
        },
    };
    let defined = h.broker.request(&h.agent, definition.clone());
    let revision = defined_revision(&defined, "shared");
    assert_eq!(
        h.broker.request(&h.agent, definition.clone()).receipt,
        defined.receipt
    );
    let first_view = h.broker.snapshot(&h.agent).unwrap();
    let second_view = h.broker.snapshot(&second).unwrap();
    assert_eq!(first_view.tasks, second_view.tasks);
    assert_eq!(first_view.tasks.len(), 1);
    assert_eq!(first_view.tasks[0].revision, revision);
    assert!(
        second_view
            .capabilities
            .contains(&wire::SHARED_TASKS.into())
    );
    let same = h.broker.request(&second, definition.clone());
    assert_eq!(defined_revision(&same, "shared"), revision);
    let changed = Request {
        operation: Op::DefineTask {
            task: "shared".into(),
            goal: "changed contract".into(),
            expected_artifacts: vec![],
        },
        ..definition
    };
    assert_eq!(
        h.broker.request(&h.agent, changed.clone()).error.as_deref(),
        Some("mutation_id_conflict")
    );
    assert_eq!(
        h.broker
            .request(&second, Request { id: 11, ..changed })
            .error
            .as_deref(),
        Some("task_contract_immutable_conflict")
    );
    for (id, task, task_revision, error) in [
        (11, "shared", None, "task_revision_required_or_stale"),
        (
            12,
            "shared",
            Some(format!("sha256:{}", "0".repeat(64))),
            "task_revision_required_or_stale",
        ),
        (
            13,
            "unknown",
            Some(revision.clone()),
            "unknown_task_contract",
        ),
    ] {
        let refused = h.broker.request(
            &h.agent,
            Request {
                version: wire::VERSION.into(),
                id,
                operation: Op::Run {
                    task: task.into(),
                    task_revision,
                    argv: vec![
                        "/bin/sh".into(),
                        "-c".into(),
                        "touch MUST_NOT_LAUNCH".into(),
                    ],
                },
            },
        );
        assert_eq!(refused.error.as_deref(), Some(error));
        assert!(refused.receipt.unwrap().result.is_none());
    }
    assert!(!h.directory.join("MUST_NOT_LAUNCH").exists());
    assert!(h.broker.snapshot(&second).unwrap().attempts.is_empty());
    let run = |marker: &str| Request {
        version: wire::VERSION.into(),
        id: 20,
        operation: Op::Run {
            task: "shared".into(),
            task_revision: Some(revision.clone()),
            argv: vec![
                "/bin/sh".into(),
                "-c".into(),
                format!(
                    "printf {marker} >> launches; printf OUTPUT_{marker}; printf ERROR_{marker} >&2"
                ),
            ],
        },
    };
    let first_request = run("A");
    let first = h.broker.request(&h.agent, first_request.clone());
    let other = h.broker.request(&second, run("B"));
    let first_id = started_attempt(&first);
    let other_id = started_attempt(&other);
    assert_ne!(first_id, other_id);
    assert_eq!(
        h.broker.request(&h.agent, first_request).receipt,
        first.receipt
    );
    let snapshot = h.wait(|snapshot| {
        snapshot.attempts.len() == 2
            && snapshot
                .attempts
                .iter()
                .all(|attempt| attempt.state == "exited")
    });
    for (id, endpoint, marker) in [(&first_id, &h.agent, "A"), (&other_id, &second, "B")] {
        let attempt = snapshot
            .attempts
            .iter()
            .find(|attempt| &attempt.id == id)
            .unwrap();
        assert_eq!(
            attempt.origin,
            Some(wire::AttemptOrigin {
                endpoint: endpoint.clone(),
                request_id: 20
            })
        );
        assert_eq!(attempt.task_revision.as_deref(), Some(revision.as_str()));
        assert_eq!(attempt.task, "shared");
        assert_eq!(attempt.stdout, format!("OUTPUT_{marker}"));
        assert_eq!(attempt.stderr, format!("ERROR_{marker}"));
        assert_eq!(attempt.exit_code, Some(0));
    }
    for (endpoint, response) in [(&h.agent, &first), (&second, &other)] {
        assert_eq!(
            h.broker
                .request(
                    endpoint,
                    Request {
                        version: wire::VERSION.into(),
                        id: 21,
                        operation: Op::Receipt { request_id: 20 },
                    }
                )
                .receipt,
            response.receipt
        );
    }
    let launches = fs::read_to_string(h.directory.join("launches")).unwrap();
    assert_eq!(launches.len(), 2);
    assert_eq!(launches.matches('A').count(), 1);
    assert_eq!(launches.matches('B').count(), 1);
    let third = h.broker.request(&second, Request { id: 21, ..run("C") });
    assert_eq!(third.error.as_deref(), Some("attempt_capacity_two_total"));
    assert!(third.receipt.unwrap().result.is_none());
    assert_eq!(
        fs::read_to_string(h.directory.join("launches"))
            .unwrap()
            .len(),
        2
    );
    let journal = fs::read_to_string(h.directory.join("journal")).unwrap();
    for forbidden in [
        "GOAL_RAM_ONLY",
        "EXPECTED_RAM_ONLY",
        "OUTPUT_A",
        "printf",
        &revision,
        &first_id,
        &other_id,
    ] {
        assert!(
            !journal.contains(forbidden),
            "payload/result leaked into metadata journal"
        );
    }
}

#[test]
fn four_contract_limit_preserves_existing_contracts_and_same_definition() {
    let mut h = Harness::new();
    let mut revisions = Vec::new();
    for n in 0..4 {
        let task = format!("task{n}");
        revisions.push(defined_revision(
            &h.human(Op::DefineTask {
                task: task.clone(),
                goal: format!("goal{n}"),
                expected_artifacts: vec![],
            }),
            &task,
        ));
    }
    let same = h.human(Op::DefineTask {
        task: "task0".into(),
        goal: "goal0".into(),
        expected_artifacts: vec![],
    });
    assert_eq!(defined_revision(&same, "task0"), revisions[0]);
    let full = h.human(Op::DefineTask {
        task: "task4".into(),
        goal: "fifth".into(),
        expected_artifacts: vec![],
    });
    assert_eq!(
        full.error.as_deref(),
        Some("task_contract_capacity_four_total")
    );
    assert!(full.receipt.unwrap().result.is_none());
    let snapshot = h.broker.snapshot(&h.agent).unwrap();
    assert_eq!(snapshot.tasks.len(), 4);
    assert_eq!(
        snapshot
            .tasks
            .iter()
            .map(|task| task.revision.clone())
            .collect::<Vec<_>>(),
        revisions
    );
    assert!(snapshot.degraded.is_none());
    assert!(snapshot.attempts.is_empty());
}

#[test]
fn adhoc_attempt_never_gains_later_contract_revision() {
    let mut h = Harness::new();
    let started = h.human(Op::Run {
        task: "label".into(),
        argv: vec!["/bin/true".into()],
        task_revision: None,
    });
    let id = started_attempt(&started);
    let definition = h.human(Op::DefineTask {
        task: "label".into(),
        goal: "new immutable contract".into(),
        expected_artifacts: vec![],
    });
    let revision = defined_revision(&definition, "label");
    let snapshot = h.broker.snapshot(&h.agent).unwrap();
    assert_eq!(
        snapshot
            .attempts
            .iter()
            .find(|attempt| attempt.id == id)
            .unwrap()
            .task_revision,
        None
    );
    let refused = h.human(Op::Run {
        task: "label".into(),
        argv: vec!["/bin/true".into()],
        task_revision: None,
    });
    assert_eq!(
        refused.error.as_deref(),
        Some("task_revision_required_or_stale")
    );
    let valid = h.human(Op::Run {
        task: "label".into(),
        argv: vec!["/bin/true".into()],
        task_revision: Some(revision),
    });
    assert_ne!(started_attempt(&valid), id);
}

#[test]
fn unknown_completion_preserves_exact_attempt_reference_without_reexecution() {
    let h = Harness::new();
    let request = Request {
        version: wire::VERSION.into(),
        id: 10,
        operation: Op::Run {
            task: "ad-hoc".into(),
            task_revision: None,
            argv: vec!["/bin/sh".into(), "-c".into(), "printf once >> once".into()],
        },
    };
    assert!(h.broker.reserve(&h.agent, &request).unwrap().is_none());
    let outcome = h.broker.apply(&h.agent, request.id, &request.operation);
    assert!(outcome.is_ok());
    h.broker.state.lock().unwrap().journal.exhaust_for_test();
    let response = h.broker.finish(&h.agent, request.id, outcome);
    assert!(!response.ok);
    let receipt = response.receipt.clone().unwrap();
    assert_eq!(receipt.state, "unknown");
    let Some(MutationResult::AttemptStarted { attempt }) = &receipt.result else {
        panic!("lost attempt correlation")
    };
    let snapshot = h.wait(|snapshot| snapshot.attempts[0].state == "exited");
    assert_eq!(snapshot.attempts[0].id, *attempt);
    assert_eq!(
        snapshot.attempts[0].origin,
        Some(wire::AttemptOrigin {
            endpoint: h.agent.clone(),
            request_id: 10
        })
    );
    assert_eq!(h.broker.request(&h.agent, request).receipt, Some(receipt));
    assert_eq!(
        fs::read_to_string(h.directory.join("once")).unwrap(),
        "once"
    );
}

#[test]
fn shutdown_is_human_only_and_fences_future_mutations() {
    let mut h = Harness::new();
    let s = h.shell();
    let r = h.request(&h.agent.clone(), Op::Shutdown {});
    assert!(!r.ok);
    assert!(!h.broker.shutdown_requested.load(Ordering::SeqCst));
    assert!(h.human(Op::Shutdown {}).ok);
    assert!(h.broker.shutdown_requested.load(Ordering::SeqCst));
    let s = h
        .broker
        .snapshot(&h.human)
        .unwrap()
        .sessions
        .into_iter()
        .find(|x| x.id == s.id)
        .unwrap();
    assert!(s.owner.is_none());
    assert!(
        !h.human(Op::CreateSession {
            name: "too late".into()
        })
        .ok
    );
}

#[test]
fn lifecycle_reuses_one_restricted_slot_after_ordinary_capacity() {
    let mut h = Harness::new();
    for _ in 0..13 {
        let (_, id) = h.broker.hello(1, Role::Agent, None);
        let id = id.unwrap();
        h.broker.disconnect(&id);
    }
    assert!(h.broker.hello(1, Role::Agent, None).1.is_none());
    for _ in 0..32 {
        let (r, id) = h.broker.lifecycle_hello(1);
        assert!(r.ok);
        let id = id.unwrap();
        assert!(
            !h.request(
                &id,
                Op::CreateSession {
                    name: "forbidden".into()
                }
            )
            .ok
        );
        h.broker.disconnect(&id);
    }
    assert_eq!(h.broker.state.lock().unwrap().endpoints.len(), 16);
    let (_, id) = h.broker.lifecycle_hello(1);
    assert!(h.request(&id.unwrap(), Op::Shutdown {}).ok);
}
#[test]
fn protective_shutdown_survives_worker_and_journal_loss_without_false_durability() {
    let mut h = Harness::new();
    h.broker.worker.stop();
    h.broker.state.lock().unwrap().journal.exhaust_for_test();
    let r = h.human(Op::Shutdown {});
    assert!(!r.ok);
    let r = r.receipt.unwrap();
    assert_eq!(r.state, "unknown");
    assert_eq!(
        r.detail,
        "protective_shutdown_requested_receipt_not_durable"
    );
    assert!(h.broker.owns_shutdown(&h.human, r.id));
    assert!(!h.broker.owns_shutdown(&h.agent, r.id));
    let queried = h.request(&h.agent.clone(), Op::Receipt { request_id: r.id });
    assert!(queried.receipt.is_none());
}

#[test]
fn previously_reserved_claim_cannot_regrant_after_shutdown_linearizes() {
    let mut h = Harness::new();
    let s = h.shell();
    assert!(h.human(Op::Shutdown {}).ok);
    let newer = h.broker.snapshot(&h.human).unwrap().sessions[0].generation;
    let result = h.broker.apply(
        &h.agent,
        999,
        &Op::TakeControl {
            session: s.id,
            generation: newer,
            force: false,
        },
    );
    assert_eq!(result.unwrap_err(), "grant_fenced_by_degraded_or_shutdown");
    assert!(
        h.broker.snapshot(&h.human).unwrap().sessions[0]
            .owner
            .is_none()
    );
}

#[test]
fn cancel_is_exact_authorized_deduplicated_and_not_completion() {
    let mut h = Harness::new();
    let agent = h.agent.clone();
    let human = h.human.clone();
    assert!(
        h.request(
            &agent,
            Op::Run {
                task: "owner".into(),
                task_revision: None,
                argv: vec!["/bin/sleep".into(), "20".into()]
            }
        )
        .ok
    );
    assert!(
        h.human(Op::Run {
            task: "peer".into(),
            task_revision: None,
            argv: vec!["/bin/sleep".into(), "20".into()]
        })
        .ok
    );
    let snap = h.broker.snapshot(&human).unwrap();
    let a = snap.attempts.iter().find(|a| a.task == "owner").unwrap();
    let b = snap.attempts.iter().find(|a| a.task == "peer").unwrap();
    let operation = |a: &Attempt| {
        let o = a.origin.as_ref().unwrap();
        Op::CancelAttempt {
            epoch: snap.epoch.clone(),
            attempt: a.id.clone(),
            origin_endpoint: o.endpoint.clone(),
            origin_request_id: o.request_id,
        }
    };
    assert!(!h.request(&agent, operation(b)).ok);
    let mut bad = operation(a);
    if let Op::CancelAttempt { epoch, .. } = &mut bad {
        *epoch = "b".repeat(64);
    }
    assert!(!h.human(bad).ok);
    let mut bad = operation(a);
    if let Op::CancelAttempt {
        origin_request_id, ..
    } = &mut bad
    {
        *origin_request_id += 99;
    }
    assert!(!h.human(bad).ok);
    assert!(
        h.broker
            .snapshot(&human)
            .unwrap()
            .attempts
            .iter()
            .all(|a| a.cancellation.is_none())
    );
    let id = h.next;
    let first = h.request(&agent, operation(a));
    assert!(first.ok);
    let receipt = first.receipt.unwrap();
    assert!(matches!(
        receipt.result,
        Some(MutationResult::CancellationRequested {
            status: wire::CancelRequestStatus::Requested,
            ..
        })
    ));
    let duplicate = h.broker.request(
        &agent,
        Request {
            version: wire::VERSION.into(),
            id,
            operation: operation(a),
        },
    );
    assert_eq!(duplicate.receipt.as_ref(), Some(&receipt));
    let conflict = h.broker.request(
        &agent,
        Request {
            version: wire::VERSION.into(),
            id,
            operation: operation(b),
        },
    );
    assert!(!conflict.ok);
    let exited = h.wait(|s| {
        s.attempts
            .iter()
            .any(|x| x.id == a.id && x.state == "exited")
    });
    let seen = exited.attempts.iter().find(|x| x.id == a.id).unwrap();
    assert_eq!(seen.exit_signal, Some(9));
    let c = seen.cancellation.as_ref().unwrap();
    assert_eq!(c.status, wire::CancellationState::ExitObserved);
    assert_eq!(c.request_id, id);
    assert_eq!(c.requested_by, agent);
    assert_eq!(
        exited.attempts.iter().find(|x| x.id == b.id).unwrap().state,
        "running"
    );
    let again = h.human(operation(a));
    assert!(matches!(
        again.receipt.unwrap().result,
        Some(MutationResult::CancellationRequested {
            status: wire::CancelRequestStatus::AlreadyExited,
            ..
        })
    ));
    assert_eq!(
        h.broker
            .snapshot(&human)
            .unwrap()
            .attempts
            .iter()
            .find(|x| x.id == a.id)
            .unwrap()
            .cancellation
            .as_ref(),
        Some(c)
    );
}

#[test]
fn cancellation_unknown_or_degraded_never_publishes_another_signal() {
    let mut h = Harness::new();
    let human = h.human.clone();
    assert!(
        h.human(Op::Run {
            task: "unknown".into(),
            task_revision: None,
            argv: vec!["/bin/sleep".into(), "20".into()]
        })
        .ok
    );
    let snap = h.broker.snapshot(&human).unwrap();
    let a = &snap.attempts[0];
    let o = a.origin.as_ref().unwrap();
    let op = Op::CancelAttempt {
        epoch: snap.epoch,
        attempt: a.id.clone(),
        origin_endpoint: o.endpoint.clone(),
        origin_request_id: o.request_id,
    };
    h.broker
        .state
        .lock()
        .unwrap()
        .attempts
        .get_mut(&a.id)
        .unwrap()
        .view
        .state = "unknown".into();
    assert!(!h.human(op.clone()).ok);
    assert!(
        !h.broker.state.lock().unwrap().attempts[&a.id]
            .cancel
            .load(Ordering::SeqCst)
    );
    h.broker.state.lock().unwrap().degraded = Some("injected".into());
    assert!(!h.human(op).ok);
    assert!(
        !h.broker.state.lock().unwrap().attempts[&a.id]
            .cancel
            .load(Ordering::SeqCst)
    );
}

#[test]
fn cancel_completion_persistence_failure_keeps_uncertainty_and_first_origin() {
    let mut h = Harness::new();
    let human = h.human.clone();
    assert!(
        h.human(Op::Run {
            task: "journal-gap".into(),
            task_revision: None,
            argv: vec!["/bin/sleep".into(), "20".into()]
        })
        .ok
    );
    let snap = h.broker.snapshot(&human).unwrap();
    let a = &snap.attempts[0];
    let origin = a.origin.as_ref().unwrap();
    let id = h.next;
    let q = Request {
        version: wire::VERSION.into(),
        id,
        operation: Op::CancelAttempt {
            epoch: snap.epoch,
            attempt: a.id.clone(),
            origin_endpoint: origin.endpoint.clone(),
            origin_request_id: origin.request_id,
        },
    };
    assert!(h.broker.reserve(&human, &q).unwrap().is_none());
    let outcome = h.broker.apply(&human, id, &q.operation);
    assert!(outcome.is_ok());
    h.broker.state.lock().unwrap().journal.exhaust_for_test();
    let r = h.broker.finish(&human, id, outcome);
    assert!(!r.ok);
    assert_eq!(r.receipt.unwrap().state, "unknown");
    let after = h.wait(|s| s.attempts[0].state == "exited");
    let c = after.attempts[0].cancellation.as_ref().unwrap();
    assert_eq!(c.requested_by, human);
    assert_eq!(c.request_id, id);
    assert_eq!(c.status, wire::CancellationState::ExitObserved);
    // Observation may prove direct-child exit even though the durable request
    // outcome remains unknown. It does not repair or relabel that receipt.
    let queried = h.broker.request(
        &human,
        Request {
            version: wire::VERSION.into(),
            id: id + 1,
            operation: Op::Receipt { request_id: id },
        },
    );
    assert_eq!(queried.receipt.unwrap().state, "unknown");
}

#[test]
fn pending_cancel_coalesces_and_never_rearms_consumed_flag() {
    let mut h = Harness::new();
    let human = h.human.clone();
    // An inert metadata fixture represents the monitor not yet acknowledging
    // a request. It is not evidence of a running OS process.
    let view:Attempt=serde_json::from_value(serde_json::json!({"id":"fixture-pending","task":"fixture","origin":{"endpoint":human,"request_id":7},"task_revision":null,"state":"running","exit_code":null,"stdout":"","stderr":"","truncated":false})).unwrap();
    let flag = Arc::new(AtomicBool::new(false));
    h.broker.state.lock().unwrap().attempts.insert(
        view.id.clone(),
        LiveAttempt {
            view,
            stdout: vec![],
            stderr: vec![],
            stdout_eof: false,
            stderr_eof: false,
            exited: false,
            cancel: Arc::clone(&flag),
        },
    );
    let op = Op::CancelAttempt {
        epoch: h.broker.epoch.clone(),
        attempt: "fixture-pending".into(),
        origin_endpoint: human,
        origin_request_id: 7,
    };
    let first = h.human(op.clone());
    assert!(first.ok);
    let id = first.id;
    assert!(flag.swap(false, Ordering::AcqRel));
    let repeated = h.human(op);
    assert!(matches!(
        repeated.receipt.unwrap().result,
        Some(MutationResult::CancellationRequested {
            status: wire::CancelRequestStatus::AlreadyRequested,
            ..
        })
    ));
    assert!(!flag.load(Ordering::Acquire));
    assert_eq!(
        h.broker.snapshot(&h.human).unwrap().attempts[0]
            .cancellation
            .as_ref()
            .unwrap()
            .request_id,
        id
    );
}
