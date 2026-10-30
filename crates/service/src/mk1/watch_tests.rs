use super::*;
use crate::mk1::watch::Watch;
use wire::{WatchBody, WatchCoverage, WatchCursor, WatchRequest};
fn call(h: &Harness, w: &mut Watch, r: WatchRequest) -> Response {
    w.request(&h.broker, &h.agent, 91, &h.broker.epoch, &r)
}
fn open(h: &Harness, w: &mut Watch, s: &Session) -> wire::SessionWatch {
    let r = call(
        h,
        w,
        WatchRequest::Open {
            session: s.id.clone(),
        },
    );
    assert!(r.ok, "{r:?}");
    r.watch.unwrap()
}
fn next(h: &Harness, w: &mut Watch, f: &wire::SessionWatch) -> wire::SessionWatch {
    let r = call(
        h,
        w,
        WatchRequest::Next {
            subscription: f.subscription.clone(),
            cursor: f.cursor.clone(),
            credit: 1,
        },
    );
    assert!(r.ok, "{r:?}");
    r.watch.unwrap()
}
#[test]
fn complete_scope_changes_include_ownership_pending_and_degraded_not_just_screen() {
    let mut h = Harness::new();
    let session = h.shell();
    let mut w = Watch::default();
    let first = open(&h, &mut w, &session);
    assert!(matches!(first.body, WatchBody::Baseline { .. }));
    assert_eq!(first.cursor.sample, 1);
    let second = next(&h, &mut w, &first);
    assert!(matches!(second.body, WatchBody::Unchanged {}));
    assert_eq!(second.cursor.sample, 2);
    assert_eq!(second.cursor.revision, first.cursor.revision);
    // Real broker control changes generation without requiring terminal output.
    assert!(
        h.human(Op::TakeControl {
            session: session.id.clone(),
            generation: session.generation,
            force: true
        })
        .ok
    );
    let owner = next(&h, &mut w, &second);
    assert!(
        matches!(&owner.body,WatchBody::Replaced{from,view} if from==&second.cursor && view.screen.revision==session.screen.revision && view.generation!=session.generation)
    );
    // Deliberate publication-state injection; not an OS scheduling guarantee.
    {
        let mut s = h.broker.state.lock().unwrap();
        s.sessions.get_mut(&session.id).unwrap().pending_output = 1;
    }
    let pending = next(&h, &mut w, &owner);
    assert_eq!(pending.canonical_pending, Some(true));
    assert!(matches!(pending.body, WatchBody::Replaced { .. }));
    {
        let mut s = h.broker.state.lock().unwrap();
        s.sessions.get_mut(&session.id).unwrap().pending_output = 0;
        s.degraded = Some("injected".into());
    }
    let unknown = next(&h, &mut w, &pending);
    assert_eq!(unknown.degraded.as_deref(), Some("injected"));
    assert!(!unknown.history_complete);
    {
        let mut s = h.broker.state.lock().unwrap();
        s.degraded = None;
    }
    let clear = next(&h, &mut w, &unknown);
    // No poll between A->B->A: latest equality never attests transition history.
    {
        let mut s = h.broker.state.lock().unwrap();
        let v = &mut s.sessions.get_mut(&session.id).unwrap().view;
        let a = v.owner.clone();
        v.owner = Some("transient".into());
        v.owner = a;
    }
    let same = next(&h, &mut w, &clear);
    assert!(matches!(same.body, WatchBody::Unchanged {}));
    assert!(!same.history_complete);
    assert_eq!(same.coverage, WatchCoverage::LatestPublishedView);
}
#[test]
fn stale_bases_latch_resync_and_close_does_not_change_writer_or_journal() {
    let mut h = Harness::new();
    let session = h.shell();
    let journal = std::fs::read(h.directory.join("journal")).unwrap();
    let mut w = Watch::default();
    let f = open(&h, &mut w, &session);
    let current = next(&h, &mut w, &f);
    let stale = next(&h, &mut w, &f);
    assert!(matches!(stale.body, WatchBody::ResyncRequired {}));
    assert_eq!(stale.cursor, current.cursor);
    assert_eq!(stale.coverage, WatchCoverage::NoObservation);
    assert_eq!(stale.canonical_pending, None);
    let still = next(&h, &mut w, &current);
    assert!(matches!(still.body, WatchBody::ResyncRequired {}));
    let sync = call(
        &h,
        &mut w,
        WatchRequest::Resync {
            subscription: f.subscription.clone(),
        },
    )
    .watch
    .unwrap();
    assert!(matches!(sync.body, WatchBody::Baseline { .. }));
    assert_eq!(sync.cursor.sample, current.cursor.sample + 1);
    let closed = call(
        &h,
        &mut w,
        WatchRequest::Close {
            subscription: f.subscription.clone(),
        },
    );
    assert!(closed.ok);
    assert!(matches!(closed.watch.unwrap().body, WatchBody::Closed {}));
    assert_eq!(
        call(
            &h,
            &mut w,
            WatchRequest::Resync {
                subscription: f.subscription
            }
        )
        .error
        .as_deref(),
        Some("watch_unknown")
    );
    assert_eq!(
        h.broker.state.lock().unwrap().sessions[&session.id]
            .view
            .owner,
        session.owner
    );
    assert_eq!(std::fs::read(h.directory.join("journal")).unwrap(), journal);
}
#[test]
fn credit_epoch_namespace_and_open_budget_refusals_do_not_advance_or_mutate() {
    let mut h = Harness::new();
    let session = h.shell();
    let mut w = Watch::default();
    for _ in 0..20 {
        assert_eq!(
            call(
                &h,
                &mut w,
                WatchRequest::Open {
                    session: "missing".into()
                }
            )
            .error
            .as_deref(),
            Some("unknown_session")
        );
    }
    let f = open(&h, &mut w, &session);
    assert_eq!(
        call(
            &h,
            &mut w,
            WatchRequest::Open {
                session: session.id.clone()
            }
        )
        .error
        .as_deref(),
        Some("watch_already_open")
    );
    for credit in [0, 2, 255] {
        assert_eq!(
            call(
                &h,
                &mut w,
                WatchRequest::Next {
                    subscription: f.subscription.clone(),
                    cursor: f.cursor.clone(),
                    credit
                }
            )
            .error
            .as_deref(),
            Some("watch_request_invalid")
        );
    }
    let r = w.request(
        &h.broker,
        &h.agent,
        1,
        &"0".repeat(64),
        &WatchRequest::Resync {
            subscription: f.subscription.clone(),
        },
    );
    assert_eq!(r.error.as_deref(), Some("watch_epoch_stale"));
    let mut other = Watch::default();
    assert_eq!(
        call(
            &h,
            &mut other,
            WatchRequest::Close {
                subscription: f.subscription.clone()
            }
        )
        .error
        .as_deref(),
        Some("watch_unknown")
    );
    let g = open(&h, &mut other, &session);
    assert_ne!(g.subscription, f.subscription);
    assert_eq!(
        call(
            &h,
            &mut other,
            WatchRequest::Resync {
                subscription: f.subscription.clone()
            }
        )
        .error
        .as_deref(),
        Some("watch_unknown")
    );
    let second = next(&h, &mut w, &f);
    assert_eq!(second.cursor.sample, 2);
    assert!(
        call(
            &h,
            &mut w,
            WatchRequest::Close {
                subscription: f.subscription
            }
        )
        .ok
    );
    for _ in 1..wire::WATCH_MAX_OPENS {
        let f = open(&h, &mut w, &session);
        assert!(
            call(
                &h,
                &mut w,
                WatchRequest::Close {
                    subscription: f.subscription
                }
            )
            .ok
        );
    }
    assert_eq!(
        call(
            &h,
            &mut w,
            WatchRequest::Open {
                session: session.id.clone()
            }
        )
        .error
        .as_deref(),
        Some("watch_open_capacity")
    );
    let r = h.request(
        &h.agent.clone(),
        Op::SessionWatch {
            epoch: h.broker.epoch.clone(),
            request: WatchRequest::Open {
                session: session.id,
            },
        },
    );
    assert_eq!(r.error.as_deref(), Some("watch_requires_transport"));
    assert!(r.receipt.is_none());
}
#[test]
fn oversized_projection_refuses_without_advancing_cursor_and_artifacts_are_not_rehashed() {
    let mut h = Harness::new();
    let session = h.shell();
    let mut w = Watch::default();
    let f = open(&h, &mut w, &session);
    let path = h.directory.join("file");
    std::fs::write(&path, "A").unwrap();
    let r = h.human(Op::Artifact {
        task: "test".into(),
        path: "file".into(),
    });
    assert!(r.ok, "{r:?}");
    let old = h
        .broker
        .state
        .lock()
        .unwrap()
        .artifacts
        .values()
        .next()
        .unwrap()
        .revision
        .clone();
    std::fs::write(path, "B").unwrap();
    let saved = {
        let mut s = h.broker.state.lock().unwrap();
        let v = &mut s.sessions.get_mut(&session.id).unwrap().view;
        let old = v.clone();
        v.screen.lines = vec!["x".repeat(wire::WATCH_MAX_REPLY)];
        old
    };
    let r = call(
        &h,
        &mut w,
        WatchRequest::Next {
            subscription: f.subscription.clone(),
            cursor: f.cursor.clone(),
            credit: 1,
        },
    );
    assert_eq!(r.error.as_deref(), Some("watch_projection_capacity"));
    assert!(r.watch.is_none());
    h.broker
        .state
        .lock()
        .unwrap()
        .sessions
        .get_mut(&session.id)
        .unwrap()
        .view = saved;
    let now = next(&h, &mut w, &f);
    assert_eq!(now.cursor.sample, 2);
    assert!(matches!(now.body, WatchBody::Unchanged {}));
    assert_eq!(
        h.broker
            .state
            .lock()
            .unwrap()
            .artifacts
            .values()
            .next()
            .unwrap()
            .revision,
        old
    );
    h.broker
        .scoped_observation(
            &h.agent,
            &h.broker.epoch,
            &wire::ObservationScope::WorkExport {},
        )
        .unwrap();
    assert_ne!(
        h.broker
            .state
            .lock()
            .unwrap()
            .artifacts
            .values()
            .next()
            .unwrap()
            .revision,
        old
    );
}
#[test]
fn future_cut_wrong_revision_and_restricted_lifecycle_are_not_observations() {
    let mut h = Harness::new();
    let session = h.shell();
    let mut w = Watch::default();
    let f = open(&h, &mut w, &session);
    for cursor in [
        WatchCursor {
            sample: f.cursor.sample + 10,
            revision: f.cursor.revision.clone(),
        },
        WatchCursor {
            sample: f.cursor.sample,
            revision: format!("sha256:{}", "0".repeat(64)),
        },
    ] {
        let r = call(
            &h,
            &mut w,
            WatchRequest::Next {
                subscription: f.subscription.clone(),
                cursor,
                credit: 1,
            },
        );
        assert!(matches!(
            r.watch.unwrap().body,
            WatchBody::ResyncRequired {}
        ));
        assert!(
            call(
                &h,
                &mut w,
                WatchRequest::Resync {
                    subscription: f.subscription.clone()
                }
            )
            .ok
        );
    }
    let r = w.request(
        &h.broker,
        "lifecycle",
        1,
        &h.broker.epoch,
        &WatchRequest::Close {
            subscription: f.subscription,
        },
    );
    assert_eq!(r.error.as_deref(), Some("watch_lifecycle_unsupported"));
    assert!(r.receipt.is_none());
}
