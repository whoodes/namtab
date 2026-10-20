//! Independent outcome/call-spy assertions; this does not prove arbitrary OS interleavings.
use super::*;
use project_x_terminal::{Attributes, Cell, Diagnostics, Modes};
use std::collections::VecDeque;

#[derive(Default)]
struct Spy {
    writes: Vec<Vec<u8>>,
    outcomes: VecDeque<io::Result<usize>>,
    after_write: Option<Box<dyn FnMut() + Send>>,
}
impl WriteOnce for Spy {
    fn write_once(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.writes.push(bytes.to_vec());
        if let Some(after) = &mut self.after_write {
            after();
        }
        self.outcomes.pop_front().unwrap_or(Ok(bytes.len()))
    }
}
fn core() -> Core<Spy> {
    let frame = Frame {
        cols: 80,
        rows: 24,
        cells: vec![
            Cell {
                text: " ".into(),
                attrs: Attributes::default()
            };
            80 * 24
        ],
        cursor_col: 0,
        cursor_row: 0,
        cursor_visible: true,
        alternate: false,
        modes: Modes::default(),
        revision: 1,
        mode_revision: 1,
        diagnostics: Diagnostics::default(),
        closed: false,
        interpretation_known: true,
    };
    Core::new(
        "test-epoch".into(),
        frame,
        Spy::default(),
        "context-shared-0@1".into(),
        "task-shared-0@1".into(),
    )
}
fn queue(c: &mut Core<Spy>, index: usize, sequence: u16) -> Receipt {
    c.admit(
        c.endpoint(index),
        sequence,
        c.binding(index),
        [sequence as u8; 32],
        b"abc".to_vec(),
        Instant::now() + Duration::from_millis(250),
    )
    .unwrap()
}
fn dispatch(c: &mut Core<Spy>, index: usize, sequence: u16) {
    if let Some(completion) = c.start(c.endpoint(index), sequence).unwrap() {
        c.finish(completion).unwrap();
    }
}
fn transfer(c: &mut Core<Spy>, seq: u16) -> Receipt {
    c.transfer(c.endpoint(0), seq, c.binding(0), "run-1", [99; 32])
        .unwrap()
}
#[test]
fn full_once_dedupes_and_changed_body_conflicts_without_second_call() {
    let mut c = core();
    let original = queue(&mut c, 0, 1);
    dispatch(&mut c, 0, 1);
    let receipt = c.lookup(c.endpoint(0), 1).unwrap();
    assert_eq!(
        (receipt.delivery.clone(), receipt.accepted, receipt.calls),
        (Delivery::Full, Some(3), 1)
    );
    assert_eq!(receipt.origin, original.origin);
    assert_eq!(
        c.existing(c.endpoint(0), 1, [1; 32]).unwrap(),
        Some(receipt.clone())
    );
    assert_eq!(
        c.existing(c.endpoint(0), 1, [2; 32]),
        Err("consumed_key_body_conflict")
    );
    dispatch(&mut c, 0, 1);
    dispatch(&mut c, 0, 1);
    assert_eq!(c.writer.writes, vec![b"abc".to_vec()]);
}
#[test]
fn every_binding_field_is_checked_at_the_actual_call_boundary() {
    // Mutate admitted origin AFTER admission: catches guards missing a field.
    let base = serde_json::to_value(core().binding(0)).unwrap();
    for (field, value) in base.as_object().unwrap() {
        let mut c = core();
        queue(&mut c, 0, 1);
        let mut changed = base.clone();
        changed[field] = if value.is_string() {
            serde_json::json!("other")
        } else {
            serde_json::json!(2)
        };
        c.runs[0].records.get_mut(&1).unwrap().receipt.origin =
            serde_json::from_value(changed).unwrap();
        dispatch(&mut c, 0, 1);
        assert!(c.writer.writes.is_empty(), "missing final fence: {field}");
        assert_eq!(c.runs[0].records[&1].receipt.calls, 0, "{field}");
    }
}
#[test]
fn runtime_authority_changes_before_call_produce_zero_without_relabel() {
    type Change = fn(&mut Core<Spy>);
    let changes: &[Change] = &[
        |c| c.runs[0].open = false,
        |c| c.runs[0].permitted = false,
        |c| c.runs[0].control += 1,
        |c| c.runs[0].capability += 1,
        |c| c.holder = Some(1),
        |c| c.assignment += 1,
        |c| c.writer_generation += 1,
        |c| c.guard += 1,
        |c| c.epoch = "other-epoch".into(),
        |c| c.pending_output = true,
        |c| c.frame.revision += 1,
        |c| c.frame.mode_revision += 1,
        |c| c.frame.interpretation_known = false,
        |c| c.frame.modes.input_protocol_known = false,
        |c| c.frame.closed = true,
        |c| c.tainted = true,
    ];
    for change in changes {
        let mut c = core();
        let receipt = queue(&mut c, 0, 1);
        change(&mut c);
        dispatch(&mut c, 0, 1);
        assert!(c.writer.writes.is_empty());
        let now = &c.runs[0].records[&1].receipt;
        assert_eq!(now.origin, receipt.origin);
        assert_eq!(now.delivery, Delivery::Zero);
    }
}
#[test]
fn transfer_with_full_queue_fences_old_intents_and_new_holder_keeps_its_namespace() {
    let mut c = core();
    let old = queue(&mut c, 0, 1);
    queue(&mut c, 0, 2);
    assert_eq!(transfer(&mut c, 3).delivery, Delivery::Transferred);
    dispatch(&mut c, 0, 1);
    dispatch(&mut c, 0, 2);
    assert!(c.writer.writes.is_empty());
    assert_eq!(c.runs[0].records[&1].receipt.origin, old.origin);
    assert_eq!(queue(&mut c, 1, 1).delivery, Delivery::Queued);
    dispatch(&mut c, 1, 1);
    assert_eq!(c.runs[1].records[&1].receipt.origin.run, "run-1");
    assert_eq!(c.runs[1].records[&1].receipt.delivery, Delivery::Full);
}
#[test]
fn late_ack_refines_original_receipt_and_lane_never_interleaves() {
    let mut c = core();
    let original = queue(&mut c, 0, 1);
    let result = c.start(c.endpoint(0), 1).unwrap().unwrap(); // call occurred; ACK withheld.
    assert_eq!(transfer(&mut c, 2).delivery, Delivery::Transferred);
    assert_eq!(c.runs[0].records[&1].receipt.delivery, Delivery::Unknown);
    queue(&mut c, 1, 1);
    assert_eq!(
        c.start(c.endpoint(1), 1).err(),
        Some("byte_lane_unresolved")
    );
    assert_eq!(c.writer.writes.len(), 1);
    c.finish(result).unwrap();
    assert_eq!(c.runs[0].records[&1].receipt.delivery, Delivery::Full);
    assert_eq!(c.runs[0].records[&1].receipt.origin, original.origin);
    dispatch(&mut c, 1, 1);
    assert_eq!(c.writer.writes.len(), 2);
}
#[test]
fn partial_unknown_interrupted_and_invalid_extent_never_retry_or_rearm() {
    for outcome in [
        Ok(1),
        Err(io::Error::other("injected unknown")),
        Err(io::Error::from(io::ErrorKind::Interrupted)),
        Ok(4),
    ] {
        let mut c = core();
        c.writer.outcomes.push_back(outcome);
        queue(&mut c, 0, 1);
        dispatch(&mut c, 0, 1);
        assert!(c.tainted);
        assert!(!c.known());
        let receipt = c.lookup(c.endpoint(0), 1).unwrap();
        assert!(matches!(
            receipt.delivery,
            Delivery::Partial | Delivery::Unknown
        ));
        if receipt.delivery == Delivery::Partial {
            assert_eq!(receipt.accepted, Some(1));
        } else {
            assert_eq!(receipt.accepted, None);
        }
        dispatch(&mut c, 0, 1);
        assert_eq!(queue(&mut c, 0, 2).delivery, Delivery::Zero);
        dispatch(&mut c, 0, 2);
        // A fresh-looking frame is not an application parser recovery certificate.
        let frame = core().frame;
        c.canonical(frame, Vec::new()).unwrap_err();
        assert_eq!(c.writer.writes, vec![b"abc".to_vec()]);
    }
}
#[test]
fn proven_zero_is_terminal_for_original_intent_and_not_automatic_retry() {
    for outcome in [Ok(0), Err(io::Error::from(io::ErrorKind::WouldBlock))] {
        let mut c = core();
        c.writer.outcomes.push_back(outcome);
        queue(&mut c, 0, 1);
        dispatch(&mut c, 0, 1);
        assert_eq!(c.runs[0].records[&1].receipt.delivery, Delivery::Zero);
        assert_eq!(c.runs[0].records[&1].receipt.accepted, Some(0));
        dispatch(&mut c, 0, 1);
        assert_eq!(c.writer.writes.len(), 1);
        assert!(c.known()); // A distinct explicitly submitted intent remains possible.
    }
}
#[test]
fn disconnect_reconnect_does_not_revive_old_queue_endpoint_or_assignment() {
    let mut c = core();
    let endpoint = c.endpoint(0);
    let original = queue(&mut c, 0, 1);
    c.disconnect(endpoint).unwrap();
    let resumed = c.reconnect(0).unwrap();
    assert_ne!(endpoint.connection, resumed.connection);
    assert!(c.start(endpoint, 1).is_err());
    dispatch(&mut c, 0, 1);
    assert!(c.writer.writes.is_empty());
    assert_eq!(c.holder, None);
    assert_eq!(c.lookup(resumed, 1).unwrap().origin, original.origin);
    assert_eq!(queue(&mut c, 0, 2).delivery, Delivery::Zero);
}
#[test]
fn close_revoke_and_deadline_at_boundary_prevent_calls() {
    for revoke in [false, true] {
        let mut c = core();
        queue(&mut c, 0, 1);
        c.close_or_revoke(0, revoke).unwrap();
        dispatch(&mut c, 0, 1);
        assert!(c.writer.writes.is_empty());
        c.disconnect(c.endpoint(0)).unwrap();
        assert!(c.reconnect(0).is_err());
    }
    let mut c = core();
    queue(&mut c, 0, 1);
    c.runs[0].records.get_mut(&1).unwrap().deadline = Instant::now();
    dispatch(&mut c, 0, 1);
    assert!(c.writer.writes.is_empty());
}
#[test]
fn completions_require_the_exact_private_call_token() {
    let mut c = core();
    queue(&mut c, 0, 1);
    let completion = c.start(c.endpoint(0), 1).unwrap().unwrap();
    assert_eq!(
        c.finish(Completion {
            lane: Lane::Application(1, 1),
            token: completion.token,
            outcome: Ok(3)
        }),
        Err("completion_provenance_mismatch")
    );
    assert_eq!(
        c.finish(Completion {
            lane: completion.lane,
            token: completion.token + 1,
            outcome: Ok(3)
        }),
        Err("completion_provenance_mismatch")
    );
    assert_eq!(c.runs[0].records[&1].receipt.delivery, Delivery::InFlight);
    c.finish(completion).unwrap();
    assert_eq!(c.writer.writes.len(), 1);
}
#[test]
fn reply_authority_is_separate_but_waits_for_unresolved_application_call() {
    let mut c = core();
    queue(&mut c, 0, 1);
    let completion = c.start(c.endpoint(0), 1).unwrap().unwrap();
    let mut frame = c.frame.clone();
    frame.revision += 1;
    c.canonical(frame, b"\x1b[0n".to_vec()).unwrap();
    c.close_or_revoke(0, true).unwrap();
    c.close_or_revoke(1, false).unwrap();
    assert_eq!(c.flush_replies(), Err("byte_lane_unresolved"));
    assert_eq!(c.writer.writes.len(), 1);
    c.finish(completion).unwrap();
    c.flush_replies().unwrap();
    c.flush_replies().unwrap();
    assert_eq!(c.writer.writes, vec![b"abc".to_vec(), b"\x1b[0n".to_vec()]);
    assert_eq!(c.replies[0].delivery, Delivery::Full); // No application grant borrowed.
    assert_eq!(c.runs[0].records[&1].receipt.origin.run, "run-0");
}
#[test]
fn partial_reply_fences_every_byte_lane_and_is_never_sent_twice() {
    let mut c = core();
    c.writer.outcomes.push_back(Ok(2));
    let mut frame = c.frame.clone();
    frame.revision += 1;
    c.canonical(frame, b"\x1b[0n".to_vec()).unwrap();
    c.flush_replies().unwrap();
    c.flush_replies().unwrap();
    assert!(c.tainted);
    assert_eq!(c.replies[0].accepted, Some(2));
    assert_eq!(queue(&mut c, 0, 1).delivery, Delivery::Zero);
    dispatch(&mut c, 0, 1);
    assert_eq!(c.writer.writes.len(), 1);
}
#[test]
fn quotas_do_not_evict_or_reinterpret_missing_evidence_as_zero() {
    let mut c = core();
    queue(&mut c, 0, 1);
    queue(&mut c, 0, 2);
    assert_eq!(queue(&mut c, 0, 3).reason, "queue_capacity");
    for sequence in 4..=wire::RECEIPTS {
        queue(&mut c, 0, sequence);
    }
    assert_eq!(
        c.existing(c.endpoint(0), 33, [33; 32]),
        Err("namespace_exhausted_or_invalid")
    );
    assert_eq!(
        c.lookup(c.endpoint(1), 1),
        Err("receipt_missing_effects_unknown")
    );
    assert_eq!(c.runs[0].records.len(), 32);
    assert_eq!(c.runs[0].records[&1].receipt.delivery, Delivery::Queued);
    assert!(c.writer.writes.is_empty());
}
#[test]
fn malformed_or_cross_principal_keys_never_consume_a_decision() {
    let mut c = core();
    let mut binding = c.binding(0);
    binding.principal = "principal-1".into();
    assert!(
        c.admit(
            c.endpoint(0),
            1,
            binding,
            [0; 32],
            b"abc".to_vec(),
            Instant::now() + Duration::from_secs(1)
        )
        .is_err()
    );
    assert!(c.existing(c.endpoint(0), 2, [0; 32]).is_err());
    assert!(c.runs[0].records.is_empty());
    assert!(c.writer.writes.is_empty());
}
#[test]
fn real_mutex_orders_transfer_against_one_call_without_relabel_or_duplicate() {
    use std::sync::{Arc, Barrier};
    for _ in 0..64 {
        let mut c = core();
        let origin = queue(&mut c, 0, 1).origin;
        let shared = Arc::new(Mutex::new(c));
        let barrier = Arc::new(Barrier::new(2));
        std::thread::scope(|scope| {
            let (child_shared, child_barrier) = (Arc::clone(&shared), Arc::clone(&barrier));
            scope.spawn(move || {
                child_barrier.wait();
                let mut c = child_shared.lock().unwrap();
                dispatch(&mut c, 0, 1);
            });
            barrier.wait();
            assert_eq!(
                transfer(&mut shared.lock().unwrap(), 2).delivery,
                Delivery::Transferred
            );
        });
        let c = shared.lock().unwrap();
        let receipt = &c.runs[0].records[&1].receipt;
        assert_eq!(receipt.origin, origin);
        assert!(c.writer.writes.len() <= 1);
        assert_eq!(
            c.writer.writes.len(),
            usize::from(receipt.delivery == Delivery::Full)
        );
    }
}
#[test]
fn reply_or_revision_overflow_fails_closed_instead_of_discarding_as_complete() {
    let mut c = core();
    queue(&mut c, 0, 1);
    queue(&mut c, 0, 2);
    let mut frame = c.frame.clone();
    frame.revision += 1;
    assert_eq!(
        c.canonical(frame, b"\x1b[0n".to_vec()),
        Err("canonical_reply_capacity")
    );
    assert!(!c.known());
    dispatch(&mut c, 0, 1);
    assert!(c.writer.writes.is_empty());
    let mut c = core();
    c.frame.revision = REVISION_CAP + 1;
    assert!(!c.known());
    let mut c = core();
    c.assignment = u16::MAX;
    assert!(transfer(&mut c, 1).delivery == Delivery::Zero);
    assert!(!c.known());
}

#[test]
fn an_unrelated_run_disconnect_does_not_close_current_holder_work() {
    for revoke in [false, true] {
        let mut c = core();
        queue(&mut c, 0, 1);
        if revoke {
            c.close_or_revoke(1, true).unwrap();
        } else {
            c.disconnect(c.endpoint(1)).unwrap();
        }
        dispatch(&mut c, 0, 1);
        assert_eq!(c.runs[0].records[&1].receipt.delivery, Delivery::Full);
        assert_eq!(c.writer.writes.len(), 1);
    }
}
#[test]
fn expired_experiment_cannot_start_another_write_even_with_fresh_operation_ttl() {
    let mut c = core();
    queue(&mut c, 0, 1);
    c.expires = Instant::now();
    dispatch(&mut c, 0, 1);
    assert!(c.writer.writes.is_empty());
    let mut c = core();
    let mut binding = c.binding(0);
    binding.context = "a".repeat(65);
    assert_eq!(
        c.admit(
            c.endpoint(0),
            1,
            binding,
            [0; 32],
            b"abc".to_vec(),
            Instant::now() + Duration::from_millis(250)
        ),
        Err("binding_outside_profile")
    );
}

#[test]
fn pending_reply_cannot_be_overtaken_by_a_new_application_call() {
    let mut c = core();
    let mut frame = c.frame.clone();
    frame.revision += 1;
    c.canonical(frame, b"\x1b[0n".to_vec()).unwrap();
    queue(&mut c, 0, 1); // Fresh admission after the output revision advanced.
    assert_eq!(
        c.start(c.endpoint(0), 1).err(),
        Some("canonical_reply_pending")
    );
    c.flush_replies().unwrap();
    dispatch(&mut c, 0, 1);
    assert_eq!(c.writer.writes, vec![b"\x1b[0n".to_vec(), b"abc".to_vec()]);
}
#[test]
fn expired_reply_and_duplicate_completion_cannot_create_effects() {
    let mut c = core();
    let mut frame = c.frame.clone();
    frame.revision += 1;
    c.canonical(frame, b"\x1b[0n".to_vec()).unwrap();
    c.expires = Instant::now();
    assert_eq!(c.flush_replies(), Err("experiment_deadline_expired"));
    assert!(c.writer.writes.is_empty());
    let mut c = core();
    queue(&mut c, 0, 1);
    let completion = c.start(c.endpoint(0), 1).unwrap().unwrap();
    let duplicate = Completion {
        lane: completion.lane,
        token: completion.token,
        outcome: Ok(0),
    };
    c.finish(completion).unwrap();
    assert_eq!(c.finish(duplicate), Err("no_active_call"));
    assert_eq!(c.runs[0].records[&1].receipt.accepted, Some(3));
}

#[test]
fn late_partial_after_transfer_fences_new_owner_without_rewriting_old_origin() {
    let mut c = core();
    c.writer.outcomes.push_back(Ok(1));
    let origin = queue(&mut c, 0, 1).origin;
    let completion = c.start(c.endpoint(0), 1).unwrap().unwrap();
    transfer(&mut c, 2);
    queue(&mut c, 1, 1);
    c.finish(completion).unwrap();
    dispatch(&mut c, 1, 1);
    assert_eq!(c.runs[0].records[&1].receipt.origin, origin);
    assert_eq!(c.runs[0].records[&1].receipt.delivery, Delivery::Partial);
    assert_eq!(c.runs[1].records[&1].receipt.delivery, Delivery::Zero);
    assert_eq!(c.writer.writes.len(), 1);
    assert!(!c.known());
}
#[test]
fn missing_ack_stays_unknown_until_original_completion_after_reconnect() {
    let mut c = core();
    let old_endpoint = c.endpoint(0);
    let origin = queue(&mut c, 0, 1).origin;
    let completion = c.start(old_endpoint, 1).unwrap().unwrap();
    c.disconnect(old_endpoint).unwrap();
    let resumed = c.reconnect(0).unwrap();
    assert_eq!(c.lookup(resumed, 1).unwrap().delivery, Delivery::Unknown);
    assert!(c.start(resumed, 1).unwrap().is_none());
    assert_eq!(c.writer.writes.len(), 1);
    c.finish(completion).unwrap();
    assert_eq!(c.lookup(resumed, 1).unwrap().delivery, Delivery::Full);
    assert_eq!(c.lookup(resumed, 1).unwrap().origin, origin);
    assert_eq!(c.holder, None);
}

#[test]
fn fenced_unresolved_call_still_occupies_capacity_and_duplicate_feed_cannot_reply() {
    let mut c = core();
    queue(&mut c, 0, 1);
    let completion = c.start(c.endpoint(0), 1).unwrap().unwrap();
    transfer(&mut c, 2);
    queue(&mut c, 1, 1);
    assert_eq!(queue(&mut c, 1, 2).reason, "queue_capacity");
    assert_eq!(c.queued(), 2);
    c.finish(completion).unwrap();
    let mut c = core();
    let mut frame = c.frame.clone();
    frame.revision += 1;
    c.canonical(frame.clone(), b"\x1b[0n".to_vec()).unwrap();
    c.flush_replies().unwrap();
    assert_eq!(
        c.canonical(frame, b"\x1b[0n".to_vec()),
        Err("canonical_discontinuity")
    );
    assert_eq!(c.writer.writes.len(), 1);
    assert!(!c.known());
}

#[test]
fn refused_nonholder_transfer_cannot_close_another_runs_queued_intent() {
    let mut c = core();
    queue(&mut c, 0, 1);
    let original = c.runs[0].records[&1].receipt.clone();
    let receipt = c
        .transfer(c.endpoint(1), 1, c.binding(1), "run-0", [11; 32])
        .unwrap();
    assert_eq!(receipt.delivery, Delivery::Zero);
    assert_eq!(c.runs[0].records[&1].receipt, original);
    dispatch(&mut c, 0, 1);
    assert_eq!(c.writer.writes.len(), 1);
}

#[test]
fn every_reply_rechecks_deadline_after_the_previous_call_returns() {
    use std::sync::Arc;
    let mut c = core();
    for _ in 0..2 {
        let mut frame = c.frame.clone();
        frame.revision += 1;
        c.canonical(frame, b"\x1b[0n".to_vec()).unwrap();
    }
    let clock = Arc::new(Mutex::new(c.expires - Duration::from_millis(1)));
    let after = Arc::clone(&clock);
    let expired = c.expires;
    c.writer.after_write = Some(Box::new(move || *after.lock().unwrap() = expired));
    assert_eq!(
        c.flush_replies_with_clock(|| *clock.lock().unwrap()),
        Err("experiment_deadline_expired")
    );
    assert_eq!(c.writer.writes.len(), 1);
    assert_eq!(c.replies[0].delivery, Delivery::Full);
    assert_eq!(c.replies[1].delivery, Delivery::Zero);
    assert!(!c.known());
}
