use super::*;
fn core() -> Core {
    Core::new("issued-epoch".into(), digest(b"fixture-context"))
}
fn queue(c: &mut Core, run: Run, sequence: u16) -> Receipt {
    c.queue(
        run,
        sequence,
        c.binding(run),
        c.artifact
            .as_ref()
            .map(|a| a.0.clone())
            .filter(|_| run == Run::Consumer),
        format!("body-{sequence}"),
    )
    .unwrap()
}
fn outcome(code: i32, drained: bool, bytes: &[u8]) -> process::Outcome {
    process::Outcome {
        exit_code: Some(code),
        drained,
        stdout: bytes.into(),
        counts: [bytes.len(), 0],
    }
}
fn produce(c: &mut Core) -> Receipt {
    queue(c, Run::Producer, 1);
    let (r, _) = c.launch(Run::Producer, 1, |_, _| Ok(())).unwrap();
    c.complete(&r, &outcome(0, true, CONTENT)).unwrap()
}
#[test]
fn missing_dependency_and_cross_endpoint_never_admit() {
    let mut c = core();
    let b = c.binding(Run::Consumer);
    assert!(c.queue(Run::Consumer, 1, b, None, "x".into()).is_err());
    assert!(
        c.queue(Run::Consumer, 1, c.binding(Run::Producer), None, "x".into())
            .is_err()
    );
    assert!(c.records.iter().all(|r| r.is_empty()));
}
#[test]
fn artifact_requires_success_complete_and_expected_content() {
    for o in [
        outcome(1, true, CONTENT),
        outcome(0, false, CONTENT),
        outcome(0, true, b"wrong"),
    ] {
        let mut c = core();
        queue(&mut c, Run::Producer, 1);
        let (r, _) = c.launch(Run::Producer, 1, |_, _| Ok(())).unwrap();
        c.complete(&r, &o).unwrap();
        assert!(c.artifact.is_none());
    }
}
#[test]
fn dependency_rechecked_at_actual_spawn() {
    let mut c = core();
    produce(&mut c);
    queue(&mut c, Run::Consumer, 1);
    c.artifact.as_mut().unwrap().0.revision = 2;
    let (r, child) = c
        .launch::<()>(Run::Consumer, 1, |_, _| panic!("stale launch"))
        .unwrap();
    assert_eq!(r.state, State::Fenced);
    assert!(child.is_none());
}
#[test]
fn literal_artifact_is_consumed_once_and_immutable() {
    let mut c = core();
    produce(&mut c);
    let queued = queue(&mut c, Run::Consumer, 1);
    let (r, _) = c
        .launch(Run::Consumer, 1, |r, bytes| {
            assert_eq!(bytes, Some(CONTENT));
            assert_eq!(r.dependency, queued.dependency);
            Ok(())
        })
        .unwrap();
    c.complete(&r, &outcome(0, true, b"consumed")).unwrap();
    let (r, _) = c
        .launch::<()>(Run::Consumer, 1, |_, _| panic!("repeat dispatch"))
        .unwrap();
    assert_eq!(r.calls, 1);
    assert_eq!(r.dependency, queued.dependency);
}
#[test]
fn takeover_fences_queued_and_active_publication_not_child_effects() {
    let mut c = core();
    queue(&mut c, Run::Producer, 1);
    c.takeover(1).unwrap();
    let (r, _) = c
        .launch::<()>(Run::Producer, 1, |_, _| panic!("queued survived"))
        .unwrap();
    assert_eq!(r.calls, 0);
    assert_eq!(r.state, State::Fenced);
    let mut c = core();
    let queued = queue(&mut c, Run::Producer, 1);
    let (r, _) = c.launch(Run::Producer, 1, |_, _| Ok(())).unwrap();
    c.takeover(1).unwrap();
    c.output(&r.origin, Stream::Stderr, 0, b"late").unwrap();
    let result = c.complete(&r, &outcome(0, true, CONTENT)).unwrap();
    assert_eq!(result.state, State::Completed);
    assert_eq!(result.origin, queued.origin);
    assert!(c.artifact.is_none());
}
#[test]
fn stale_or_forged_takeover_is_not_authority() {
    let mut c = core();
    let request = wire::Request {
        version: wire::Version::V0,
        id: 1,
        operation: Operation::Takeover {
            expected_control: 1,
        },
    };
    assert!(
        c.request(Endpoint::Run(Run::Producer), request)
            .result
            .is_err()
    );
    assert_eq!(c.control, 1);
    assert!(c.takeover(2).is_err());
    c.takeover(1).unwrap();
    assert!(c.takeover(1).is_err());
}
#[test]
fn stale_context_or_generation_refuses_before_effect() {
    for field in 0..5 {
        let mut c = core();
        let mut b = c.binding(Run::Producer);
        match field {
            0 => b.context = "old".into(),
            1 => b.epoch = "old".into(),
            2 => b.control = 0,
            3 => b.connection = 2,
            _ => b.attempt = "old".into(),
        };
        assert!(c.queue(Run::Producer, 1, b, None, "x".into()).is_err());
        assert!(c.records[0].is_empty());
    }
}
#[test]
fn duplicate_admission_never_revives_after_takeover() {
    let mut c = core();
    let original = queue(&mut c, Run::Producer, 1);
    c.takeover(1).unwrap();
    let dup = c
        .queue(
            Run::Producer,
            1,
            original.origin.clone(),
            None,
            "body-1".into(),
        )
        .unwrap();
    assert_eq!(dup.state, State::Fenced);
    assert_eq!(dup.origin, original.origin);
    assert!(
        c.queue(Run::Producer, 1, original.origin, None, "changed".into())
            .is_err()
    );
}
#[test]
fn unknown_launch_is_never_retried() {
    let mut c = core();
    queue(&mut c, Run::Producer, 1);
    let (r, child): (_, Option<()>) = c
        .launch(Run::Producer, 1, |_, _| {
            Err(std::io::Error::other("injected ambiguous spawn"))
        })
        .unwrap();
    assert_eq!(r.state, State::Unknown);
    assert!(child.is_none());
    let (r, _) = c
        .launch::<()>(Run::Producer, 1, |_, _| panic!("unknown retried"))
        .unwrap();
    assert_eq!(r.calls, 1);
}
#[test]
fn capacity_is_explicit_and_takeover_reserved() {
    let mut c = core();
    for _ in 0..wire::MAX_EVENTS {
        c.event("test", None, None, 0, &[]).unwrap();
    }
    assert!(c.event("overflow", None, None, 0, &[]).is_err());
    assert!(!c.known);
    assert_eq!(c.takeover(1).unwrap(), 2);
    let v = c.observe(Endpoint::Human, 0).unwrap();
    assert!(v.human && !v.admission_known);
    assert!(v.more);
}
#[test]
fn scoped_pagination_advances_over_hidden_events_without_loss() {
    let mut c = core();
    for _ in 0..20 {
        c.event(
            "private",
            Some(c.binding(Run::Producer)),
            Some(Stream::Stderr),
            0,
            b"secret",
        )
        .unwrap();
    }
    for _ in 0..20 {
        c.event("shared", None, None, 0, &[]).unwrap();
    }
    let page = c.observe(Endpoint::Run(Run::Consumer), 0).unwrap();
    assert_eq!(page.events.len(), wire::PAGE);
    assert!(page.events.iter().all(|e| e.origin.is_none()));
    assert_eq!(page.cursor, 36);
    let tail = c
        .observe(Endpoint::Run(Run::Consumer), page.cursor)
        .unwrap();
    assert_eq!(tail.events.len(), 4);
    assert!(!tail.more);
    assert!(c.observe(Endpoint::Human, 41).is_err());
}
#[test]
fn completion_must_match_original_attempt() {
    let mut c = core();
    queue(&mut c, Run::Producer, 1);
    let (mut r, _) = c.launch(Run::Producer, 1, |_, _| Ok(())).unwrap();
    r.origin.context = "forged".into();
    assert!(c.complete(&r, &outcome(0, true, CONTENT)).is_err());
    assert!(c.artifact.is_none());
}
#[test]
fn expired_queue_never_launches() {
    let mut c = core();
    queue(&mut c, Run::Producer, 1);
    c.expires = Instant::now();
    let (r, _) = c
        .launch::<()>(Run::Producer, 1, |_, _| panic!("expired"))
        .unwrap();
    assert_eq!(r.calls, 0);
}
#[test]
fn output_cap_and_origin_checks_are_not_success() {
    let mut c = core();
    queue(&mut c, Run::Producer, 1);
    let (r, _) = c.launch(Run::Producer, 1, |_, _| Ok(())).unwrap();
    assert!(
        c.output(&c.binding(Run::Consumer), Stream::Stderr, 0, b"wrong")
            .is_err()
    );
    assert!(
        c.output(&r.origin, Stream::Stderr, wire::STREAM_BYTES, b"overflow")
            .is_err()
    );
    assert!(!c.known);
}

#[test]
fn response_stays_bounded_at_largest_issued_projection() {
    let mut c = Core::new("a".repeat(32), "b".repeat(64));
    produce(&mut c);
    queue(&mut c, Run::Consumer, 1);
    for _ in c.events.len()..wire::MAX_EVENTS {
        c.event(
            "output",
            Some(c.binding(Run::Producer)),
            Some(Stream::Stderr),
            4000,
            &[b'x'; 256],
        )
        .unwrap();
    }
    let response = wire::Response {
        version: wire::Version::V0,
        id: 65535,
        result: Ok(Body::Observation {
            value: c.observe(Endpoint::Human, 0).unwrap(),
        }),
    };
    assert!(serde_json::to_vec(&response).unwrap().len() <= wire::MAX_RESPONSE);
}
#[test]
fn actual_mutex_orders_spawn_and_human_control_without_relabelling() {
    use std::sync::{Arc, Mutex, mpsc::channel};
    let c = Arc::new(Mutex::new(core()));
    queue(&mut c.lock().unwrap(), Run::Producer, 1);
    let (started, rx) = channel();
    let (release, wait) = channel();
    let launchcore = Arc::clone(&c);
    let launch = std::thread::spawn(move || {
        launchcore
            .lock()
            .unwrap()
            .launch(Run::Producer, 1, |_, _| {
                started.send(()).unwrap();
                wait.recv().unwrap();
                Ok(())
            })
            .unwrap()
            .0
    });
    rx.recv().unwrap();
    let human = Arc::clone(&c);
    let takeover = std::thread::spawn(move || human.lock().unwrap().takeover(1).unwrap());
    release.send(()).unwrap();
    let original = launch.join().unwrap();
    assert_eq!(takeover.join().unwrap(), 2);
    let result = c
        .lock()
        .unwrap()
        .complete(&original, &outcome(0, true, CONTENT))
        .unwrap();
    assert_eq!(result.origin.control, 1);
    assert!(c.lock().unwrap().artifact.is_none());
}
#[test]
fn artifact_bytes_cannot_diverge_from_the_pinned_reference() {
    let mut c = core();
    produce(&mut c);
    queue(&mut c, Run::Consumer, 1);
    c.artifact.as_mut().unwrap().1[0] = b'X';
    let (r, _) = c
        .launch::<()>(Run::Consumer, 1, |_, _| panic!("corrupt artifact launched"))
        .unwrap();
    assert_eq!(r.calls, 0);
    assert_eq!(r.state, State::Fenced);
}

#[test]
fn result_and_artifact_reserve_journal_slots_atomically() {
    for occupied in [126, 127, 128] {
        let mut c = core();
        queue(&mut c, Run::Producer, 1);
        let (origin, _) = c.launch(Run::Producer, 1, |_, _| Ok(())).unwrap();
        while c.events.len() < occupied {
            c.event("fill", None, None, 0, &[]).unwrap();
        }
        let receipt = c.complete(&origin, &outcome(0, true, CONTENT)).unwrap();
        assert_eq!(receipt.exit_code, Some(0));
        assert!(receipt.drained);
        assert_eq!(receipt.calls, 1);
        if occupied == 126 {
            assert_eq!(receipt.state, State::Completed);
            assert!(c.artifact.is_some());
            assert_eq!(c.events.len(), 128);
        } else {
            assert_eq!(receipt.state, State::Failed);
            assert_eq!(receipt.reason, "event_capacity_result_incomplete");
            assert!(!c.known);
            assert!(c.artifact.is_none());
            assert_eq!(c.events.len(), occupied);
        }
        assert_eq!(c.takeover(1).unwrap(), 2);
        let (duplicate, _) = c
            .launch::<()>(Run::Producer, 1, |_, _| panic!("journal failure retried"))
            .unwrap();
        assert_eq!(duplicate, receipt);
    }
}
