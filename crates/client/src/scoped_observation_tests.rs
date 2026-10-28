use crate::{mk1::Client, mk1_protocol as p};
use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixListener,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
static NEXT: AtomicUsize = AtomicUsize::new(0);

#[test]
fn scoped_discovery_is_once_and_wrong_or_mixed_projections_fence_without_fallback() {
    for case in [
        "success",
        "invalid_target",
        "raw_epoch",
        "unsupported",
        "refused",
        "missing_discovery",
        "bad_discovery",
        "discovery_receipt",
        "profile",
        "version",
        "epoch",
        "endpoint",
        "scope",
        "session_id",
        "attempt_id",
        "missing",
        "receipt",
        "snapshot",
        "resume",
        "error_with_view",
    ] {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.artifacts")
            .join(format!(
                "sc-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("s");
        let listener = UnixListener::bind(&path).unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let observed = count.clone();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let expected = if ["invalid_target", "raw_epoch"].contains(&case) {
                1
            } else if ["success", "refused"].contains(&case) {
                4
            } else if [
                "unsupported",
                "missing_discovery",
                "bad_discovery",
                "discovery_receipt",
            ]
            .contains(&case)
            {
                2
            } else {
                3
            };
            for step in 0..expected {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let q: p::Request = serde_json::from_str(&line).unwrap();
                observed.fetch_add(1, Ordering::SeqCst);
                let mut r:p::Response=serde_json::from_value(serde_json::json!({"id":q.id,"ok":true,"error":null,"receipt":null,"snapshot":null,"endpoint":null,"resume":null,"epoch":"a".repeat(64)})).unwrap();
                if step == 0 {
                    assert!(matches!(q.operation, p::Op::Hello { .. }));
                    r.endpoint = Some("agent".into());
                    r.resume = Some("synthetic-resume".into());
                } else if step == 1 {
                    assert!(matches!(q.operation, p::Op::Observe {}));
                    if case != "missing_discovery" {
                        r.snapshot=Some(serde_json::from_value(serde_json::json!({"version":p::VERSION,"epoch":"a".repeat(64),"endpoint":if case=="bad_discovery"{"wrong"}else{"agent"},"capabilities":if case=="unsupported"{vec![]}else{vec![p::SCOPED_OBSERVE]},"tasks":[],"endpoints":[],"sessions":[],"attempts":[],"artifacts":[],"receipts":[],"degraded":null})).unwrap());
                    }
                } else {
                    assert!(matches!(q.operation, p::Op::ObserveScoped { .. }));
                    r.observation = Some(p::ScopedObservation {
                        version: p::VERSION.into(),
                        profile: p::OBSERVATION_PROFILE.into(),
                        epoch: "a".repeat(64),
                        endpoint: "agent".into(),
                        degraded: None,
                        view: p::ObservationView::Work {
                            endpoints: vec![],
                            capabilities: vec![p::SCOPED_OBSERVE.into()],
                            tasks: vec![],
                            sessions: vec![],
                            attempts: vec![],
                        },
                    });
                    if step == 2 {
                        match case {
                        "refused"=>{r.ok=false;r.error=Some("test_refusal".into());r.observation=None;},
                        "profile"=>r.observation.as_mut().unwrap().profile="other/0".into(),
                        "version"=>r.observation.as_mut().unwrap().version="other/1".into(),
                        "epoch"=>r.observation.as_mut().unwrap().epoch="b".repeat(64),
                        "endpoint"=>r.observation.as_mut().unwrap().endpoint="other".into(),
                        "scope"=>r.observation.as_mut().unwrap().view=p::ObservationView::Attempt{attempt:serde_json::from_value(serde_json::json!({"id":"other","task":"t","state":"running","exit_code":null,"stdout":"","stderr":"","truncated":false})).unwrap()},
                        "session_id"=>r.observation.as_mut().unwrap().view=p::ObservationView::Session{session:serde_json::from_value(serde_json::json!({"id":"other","name":"n","owner":null,"generation":1,"process":"running","exit_code":null,"screen":{"cols":2,"rows":2,"lines":["",""],"cursor_col":0,"cursor_row":0,"cursor_visible":true,"revision":1,"mode_revision":1,"alternate":false,"input_known":true,"diagnostics":[]}})).unwrap()},
                        "attempt_id"=>r.observation.as_mut().unwrap().view=p::ObservationView::Attempt{attempt:serde_json::from_value(serde_json::json!({"id":"other","task":"t","state":"running","exit_code":null,"stdout":"","stderr":"","truncated":false})).unwrap()},
                        "missing"=>r.observation=None,
                        "resume"=>r.resume=Some("unexpected".into()),
                        "snapshot"=>r.snapshot=Some(serde_json::from_value(serde_json::json!({"version":p::VERSION,"epoch":"a".repeat(64),"endpoint":"agent","endpoints":[],"sessions":[],"attempts":[],"artifacts":[],"receipts":[],"degraded":null})).unwrap()),
                        "error_with_view"=>{r.ok=false;r.error=Some("bad".into());}, _=>{}
                    }
                    }
                }
                if (step == 1 && case == "discovery_receipt") || (step == 2 && case == "receipt") {
                    r.receipt = Some(p::Receipt {
                        endpoint: "agent".into(),
                        id: q.id,
                        operation: "input".into(),
                        state: "applied".into(),
                        detail: "unexpected".into(),
                        delivered_bytes: 1,
                        result: None,
                    });
                }
                stream
                    .write_all(format!("{}\n", serde_json::to_string(&r).unwrap()).as_bytes())
                    .unwrap();
            }
            let mut tail = String::new();
            assert_eq!(
                reader.read_line(&mut tail).unwrap(),
                0,
                "unexpected fallback/retry"
            );
        });
        let mut c = Client::connect(&path, "test", p::Role::Agent).unwrap();
        let wanted = match case {
            "invalid_target" => p::ObservationScope::Session { session: "".into() },
            "session_id" => p::ObservationScope::Session {
                session: "session1".into(),
            },
            "attempt_id" => p::ObservationScope::Attempt {
                attempt: "attempt1".into(),
            },
            _ => p::ObservationScope::Work {},
        };
        let first = if case == "raw_epoch" {
            c.call(p::Op::ObserveScoped {
                epoch: "b".repeat(64),
                scope: p::ObservationScope::Work {},
            })
            .map(|r| r.observation.unwrap())
        } else {
            c.observe_scoped(wanted)
        };
        if ["invalid_target", "raw_epoch"].contains(&case) {
            assert_eq!(first.unwrap_err().kind(), std::io::ErrorKind::InvalidInput);
            assert!(!c.transport_failed());
        } else if case == "success" {
            assert!(first.is_ok());
            assert!(c.observe_scoped(p::ObservationScope::Work {}).is_ok());
        } else if case == "unsupported" {
            assert_eq!(first.unwrap_err().kind(), std::io::ErrorKind::Unsupported);
            assert!(c.observe_scoped(p::ObservationScope::Work {}).is_err());
            assert!(!c.transport_failed());
        } else if case == "refused" {
            assert!(first.is_err());
            assert!(!c.transport_failed());
            assert!(c.observe_scoped(p::ObservationScope::Work {}).is_ok());
        } else {
            assert!(first.is_err(), "{case}");
            assert!(c.transport_failed(), "{case}");
            assert!(c.observe_scoped(p::ObservationScope::Work {}).is_err());
        }
        drop(c);
        handle.join().unwrap();
        assert!(count.load(Ordering::SeqCst) <= 4);
    }
}

#[test]
fn target_syntax_has_finite_byte_and_control_bounds() {
    // Target validity is pure, independently covering Unicode byte cap/control characters.
    for id in ["".to_owned(), "é".repeat(41), "\n".into(), "x".repeat(81)] {
        assert!(!p::ObservationScope::Session { session: id }.valid());
    }
    assert!(
        p::ObservationScope::Session {
            session: "session1".into()
        }
        .valid()
    );
}

#[test]
fn live_resume_renegotiates_support_on_the_new_transport() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.artifacts")
        .join(format!(
            "sr-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("s");
    let listener = UnixListener::bind(&path).unwrap();
    let server = std::thread::spawn(move || {
        for generation in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            for step in 0..if generation == 0 { 2 } else { 3 } {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let q: p::Request = serde_json::from_str(&line).unwrap();
                let mut r:p::Response=serde_json::from_value(serde_json::json!({"id":q.id,"ok":true,"error":null,"receipt":null,"snapshot":null,"endpoint":null,"resume":null,"epoch":"a".repeat(64)})).unwrap();
                match step {
                    0 => {
                        let p::Op::Hello { resume, .. } = q.operation else {
                            panic!("hello required")
                        };
                        assert_eq!(resume.is_some(), generation == 1);
                        r.endpoint = Some("agent".into());
                        r.resume = Some("synthetic-resume".into());
                    }
                    1 => {
                        assert!(matches!(q.operation, p::Op::Observe {}));
                        r.snapshot=Some(serde_json::from_value(serde_json::json!({"version":p::VERSION,"epoch":"a".repeat(64),"endpoint":"agent","capabilities":if generation==0 {vec![]}else{vec![p::SCOPED_OBSERVE]},"tasks":[],"endpoints":[],"sessions":[],"attempts":[],"artifacts":[],"receipts":[],"degraded":null})).unwrap());
                    }
                    2 => {
                        assert!(matches!(q.operation, p::Op::ObserveScoped { .. }));
                        r.observation = Some(p::ScopedObservation {
                            version: p::VERSION.into(),
                            profile: p::OBSERVATION_PROFILE.into(),
                            epoch: "a".repeat(64),
                            endpoint: "agent".into(),
                            degraded: None,
                            view: p::ObservationView::Work {
                                endpoints: vec![],
                                capabilities: vec![p::SCOPED_OBSERVE.into()],
                                tasks: vec![],
                                sessions: vec![],
                                attempts: vec![],
                            },
                        });
                    }
                    _ => unreachable!(),
                }
                stream
                    .write_all(format!("{}\n", serde_json::to_string(&r).unwrap()).as_bytes())
                    .unwrap();
            }
            let mut extra = String::new();
            assert_eq!(reader.read_line(&mut extra).unwrap(), 0);
        }
    });
    let mut c = Client::connect(&path, "first", p::Role::Agent).unwrap();
    assert_eq!(
        c.observe_scoped(p::ObservationScope::Work {})
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::Unsupported
    );
    let resume = c.resume.clone();
    let next = c.next_id();
    c.close_transport();
    drop(c);
    let mut c = Client::connect_resume(&path, "same", p::Role::Agent, Some(resume), next).unwrap();
    assert!(c.observe_scoped(p::ObservationScope::Work {}).is_ok());
    drop(c);
    server.join().unwrap();
}
