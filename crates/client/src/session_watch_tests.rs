use crate::{mk1::Client, mk1_protocol as p};
use serde_json::{Value, json};
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
fn view() -> Value {
    json!({"id":"session1","name":"synthetic","owner":null,"generation":1,"process":"running","exit_code":null,"screen":{"cols":80,"rows":24,"lines":vec!["";24],"cursor_col":0,"cursor_row":0,"cursor_visible":true,"revision":1,"mode_revision":1,"alternate":false,"input_known":true,"diagnostics":[]}})
}
fn frame(sample: u64, kind: &str) -> Value {
    let body = match kind {
        "baseline" => json!({"kind":kind,"view":view()}),
        "replaced" => {
            json!({"kind":kind,"from":{"sample":1,"revision":format!("sha256:{}","a".repeat(64))},"view":view()})
        }
        _ => json!({"kind":kind}),
    };
    json!({"version":p::VERSION,"profile":p::SESSION_WATCH_PROFILE,"epoch":"a".repeat(64),"endpoint":"agent","subscription":"sub-1","session":"session1","cursor":{"sample":sample,"revision":format!("sha256:{}","a".repeat(64))},"coverage":if kind=="closed"||kind=="resync_required"{"no_observation"}else{"latest_published_view"},"history_complete":false,"canonical_pending":if kind=="closed"||kind=="resync_required"{Value::Null}else{json!(false)},"degraded":null,"body":body})
}
#[test]
fn transport_contracts_fail_closed_without_automatic_credit_or_resync() {
    let baseline_faults = [
        "profile",
        "version",
        "inner_epoch",
        "endpoint",
        "subscription",
        "session",
        "sample",
        "revision",
        "history",
        "coverage",
        "pending",
        "target",
        "snapshot",
        "observation",
        "receipt",
        "resume",
        "error",
        "oversize",
        "missing",
        "outer_epoch",
        "truncated",
    ];
    let next_faults = [
        "next_cut",
        "from",
        "changed_same_revision",
        "unchanged_revision",
        "next_subscription",
        "no_observation_data",
        "refusal_with_view",
        "unchanged_pending",
    ];
    let close_faults = ["close_kind", "close_cursor", "close_pending"];
    let all: Vec<_> = [
        "success",
        "changed_success",
        "resync_required",
        "unsupported",
        "invalid_target",
        "raw_epoch",
        "invalid_credit",
        "refused",
        "unrequested",
    ]
    .into_iter()
    .chain(baseline_faults)
    .chain(next_faults)
    .chain(close_faults)
    .collect();
    for case in all {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.artifacts")
            .join(format!(
                "sw-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("s");
        let listener = UnixListener::bind(&path).unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let seen = count.clone();
        let expected = if case == "invalid_target" {
            1
        } else if ["unsupported", "raw_epoch", "unrequested"].contains(&case) {
            2
        } else if baseline_faults.contains(&case) {
            3
        } else if case == "refused" {
            5
        } else if ["success", "changed_success", "resync_required"].contains(&case)
            || close_faults.contains(&case)
        {
            6
        } else {
            4
        };
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut reader = BufReader::new(socket.try_clone().unwrap());
            for step in 0..expected {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                assert!(!line.is_empty(), "{case} step{step}");
                let q: p::Request = serde_json::from_str(&line).unwrap();
                seen.fetch_add(1, Ordering::SeqCst);
                let mut r = json!({"id":q.id,"ok":true,"error":null,"receipt":null,"snapshot":null,"endpoint":null,"resume":null,"epoch":"a".repeat(64)});
                if step == 0 {
                    assert!(matches!(q.operation, p::Op::Hello { .. }));
                    r["endpoint"] = json!("agent");
                    r["resume"] = json!("RAM-only-synthetic");
                } else if matches!(q.operation, p::Op::Observe {}) {
                    r["snapshot"] = json!({"version":p::VERSION,"epoch":"a".repeat(64),"endpoint":"agent","capabilities":if case=="unsupported"{vec![]}else{vec![p::SESSION_WATCH]},"endpoints":[],"tasks":[],"sessions":[],"attempts":[],"artifacts":[],"receipts":[],"degraded":null});
                    if case == "unrequested" {
                        r["watch"] = frame(1, "baseline");
                    }
                } else {
                    let p::Op::SessionWatch { request, .. } = &q.operation else {
                        panic!("wrong route");
                    };
                    r["watch"] = match request {
                        p::WatchRequest::Open { .. } => frame(1, "baseline"),
                        p::WatchRequest::Next { cursor, credit, .. } => {
                            assert_eq!(*credit, 1);
                            assert_eq!(cursor.sample, 1);
                            if case == "changed_success" {
                                let mut f = frame(2, "replaced");
                                f["cursor"]["revision"] =
                                    json!(format!("sha256:{}", "b".repeat(64)));
                                f["body"]["view"]["generation"] = json!(2);
                                f
                            } else if case == "resync_required" {
                                frame(1, "resync_required")
                            } else {
                                frame(2, "unchanged")
                            }
                        }
                        p::WatchRequest::Resync { .. } => {
                            frame(if case == "resync_required" { 2 } else { 3 }, "baseline")
                        }
                        p::WatchRequest::Close { .. } => frame(
                            if case == "invalid_credit" {
                                1
                            } else if case == "resync_required" {
                                2
                            } else {
                                3
                            },
                            "closed",
                        ),
                    };
                    if step == 2 {
                        match case {
                            "profile" => r["watch"]["profile"] = json!("other/0"),
                            "version" => r["watch"]["version"] = json!("other/0"),
                            "inner_epoch" => r["watch"]["epoch"] = json!("b".repeat(64)),
                            "endpoint" => r["watch"]["endpoint"] = json!("other"),
                            "subscription" => r["watch"]["subscription"] = json!(""),
                            "session" => r["watch"]["session"] = json!("session2"),
                            "sample" => r["watch"]["cursor"]["sample"] = json!(2),
                            "revision" => r["watch"]["cursor"]["revision"] = json!("invalid"),
                            "history" => r["watch"]["history_complete"] = json!(true),
                            "coverage" => r["watch"]["coverage"] = json!("no_observation"),
                            "pending" => r["watch"]["canonical_pending"] = Value::Null,
                            "target" => r["watch"]["body"]["view"]["id"] = json!("session2"),
                            "snapshot" => r["snapshot"] = json!({}),
                            "observation" => r["observation"] = json!({}),
                            "receipt" => r["receipt"] = json!({}),
                            "resume" => r["resume"] = json!("unrequested-cap"),
                            "error" => r["error"] = json!("contradiction"),
                            "oversize" => {
                                r["watch"]["body"]["view"]["name"] =
                                    json!("x".repeat(p::WATCH_MAX_REPLY))
                            }
                            "missing" => r["watch"] = Value::Null,
                            "outer_epoch" => r["epoch"] = json!("b".repeat(64)),
                            "truncated" => {
                                socket.write_all(b"{\"id\":3").unwrap();
                                break;
                            }
                            _ => {}
                        }
                    }
                    if step == 3 {
                        match case {
                            "refused" => {
                                r["ok"] = json!(false);
                                r["error"] = json!("watch_projection_capacity");
                                r["watch"] = Value::Null;
                            }
                            "refusal_with_view" => {
                                r["ok"] = json!(false);
                                r["error"] = json!("refused");
                            }
                            "unchanged_pending" => r["watch"]["canonical_pending"] = json!(true),
                            "next_cut" => r["watch"]["cursor"]["sample"] = json!(4),
                            "from" => {
                                r["watch"] = frame(2, "replaced");
                                r["watch"]["cursor"]["revision"] =
                                    json!(format!("sha256:{}", "b".repeat(64)));
                                r["watch"]["body"]["from"]["sample"] = json!(9);
                            }
                            "changed_same_revision" => r["watch"] = frame(2, "replaced"),
                            "unchanged_revision" => {
                                r["watch"]["cursor"]["revision"] =
                                    json!(format!("sha256:{}", "b".repeat(64)))
                            }
                            "next_subscription" => r["watch"]["subscription"] = json!("other-sub"),
                            "no_observation_data" => {
                                r["watch"] = frame(1, "resync_required");
                                r["watch"]["canonical_pending"] = json!(false);
                            }
                            _ => {}
                        }
                    }
                }
                if step == 5 {
                    match case {
                        "close_kind" => r["watch"] = frame(3, "baseline"),
                        "close_cursor" => r["watch"]["cursor"]["sample"] = json!(4),
                        "close_pending" => r["watch"]["canonical_pending"] = json!(false),
                        _ => {}
                    }
                }
                let mut bytes = serde_json::to_vec(&r).unwrap();
                bytes.push(b'\n');
                if socket.write_all(&bytes).is_err() {
                    break;
                }
            }
        });
        let mut client = Client::connect(&path, "synthetic", p::Role::Agent).unwrap();
        if case == "invalid_target" {
            assert!(client.session_watch_open("").is_err());
        } else if case == "raw_epoch" {
            client.discover_scoped().unwrap();
            let id = client.next_id();
            assert!(
                client
                    .call(p::Op::SessionWatch {
                        epoch: "b".repeat(64),
                        request: p::WatchRequest::Open {
                            session: "session1".into()
                        }
                    })
                    .is_err()
            );
            assert_eq!(client.next_id(), id);
        } else {
            let first = client.session_watch_open("session1");
            if ["unsupported", "unrequested"].contains(&case) || baseline_faults.contains(&case) {
                assert!(first.is_err(), "{case}");
                if case != "unsupported" {
                    assert!(client.transport_failed(), "{case}");
                    let n = client.next_id();
                    assert!(client.call(p::Op::Observe {}).is_err());
                    assert_eq!(client.next_id(), n);
                }
            } else {
                let first = first.unwrap();
                let before = client.next_id();
                for field in ["epoch", "endpoint", "subscription", "session"] {
                    let mut wrong = first.clone();
                    match field {
                        "epoch" => wrong.epoch = "other".into(),
                        "endpoint" => wrong.endpoint = "other".into(),
                        "subscription" => wrong.subscription = "other".into(),
                        _ => wrong.session = "other".into(),
                    }
                    assert!(client.session_watch_next(&wrong).is_err(), "{field}");
                    assert_eq!(client.next_id(), before);
                }

                if case == "invalid_credit" {
                    let id = client.next_id();
                    assert!(
                        client
                            .call(p::Op::SessionWatch {
                                epoch: client.epoch.clone(),
                                request: p::WatchRequest::Next {
                                    subscription: first.subscription,
                                    cursor: first.cursor,
                                    credit: 0
                                }
                            })
                            .is_err()
                    );
                    assert_eq!(client.next_id(), id);
                    client.session_watch_close().unwrap();
                } else {
                    let next = client.session_watch_next(&first);
                    if next_faults.contains(&case) {
                        assert!(next.is_err(), "{case}");
                        assert!(client.transport_failed(), "{case}");
                    } else if case == "refused" {
                        assert!(next.is_err());
                        assert!(!client.transport_failed());
                        client.observe_checked().unwrap();
                    } else {
                        let next = next.unwrap();
                        assert!(match case {
                            "changed_success" => matches!(next.body, p::WatchBody::Replaced { .. }),
                            "resync_required" =>
                                matches!(next.body, p::WatchBody::ResyncRequired {}),
                            _ => matches!(next.body, p::WatchBody::Unchanged {}),
                        });
                        let id = client.next_id();
                        assert!(client.session_watch_next(&first).is_err());
                        assert_eq!(client.next_id(), id);
                        let baseline = client.session_watch_resync().unwrap();
                        assert_eq!(
                            baseline.cursor.sample,
                            if case == "resync_required" { 2 } else { 3 }
                        );
                        let closed = client.session_watch_close();
                        if close_faults.contains(&case) {
                            assert!(closed.is_err());
                            assert!(client.transport_failed());
                        } else {
                            closed.unwrap_or_else(|e| panic!("close {case}: {e}"));
                        }
                        assert!(client.session_watch_next(&baseline).is_err());
                    }
                }
            }
        }
        drop(client);
        server.join().unwrap();
        assert_eq!(count.load(Ordering::SeqCst), expected, "{case}");
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}

#[test]
fn closed_subscription_ids_cannot_rebrand_an_old_ack_but_fresh_reopen_works() {
    for reuse in [true, false] {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.artifacts")
            .join(format!(
                "swr-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("s");
        let listener = UnixListener::bind(&path).unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let seen = count.clone();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut reader = BufReader::new(socket.try_clone().unwrap());
            for step in 0..if reuse { 5 } else { 6 } {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let q: p::Request = serde_json::from_str(&line).unwrap();
                seen.fetch_add(1, Ordering::SeqCst);
                let mut r = json!({"id":q.id,"ok":true,"error":null,"receipt":null,"snapshot":null,"endpoint":null,"resume":null,"epoch":"a".repeat(64)});
                if step == 0 {
                    r["endpoint"] = json!("agent");
                    r["resume"] = json!("synthetic");
                } else if step == 1 {
                    r["snapshot"] = json!({"version":p::VERSION,"epoch":"a".repeat(64),"endpoint":"agent","capabilities":[p::SESSION_WATCH],"tasks":[],"endpoints":[],"sessions":[],"attempts":[],"artifacts":[],"receipts":[],"degraded":null});
                } else {
                    r["watch"] = frame(
                        if step == 5 { 2 } else { 1 },
                        if step == 3 {
                            "closed"
                        } else if step == 5 {
                            "unchanged"
                        } else {
                            "baseline"
                        },
                    );
                    if step >= 4 && !reuse {
                        r["watch"]["subscription"] = json!("sub-fresh");
                    }
                }
                let mut bytes = serde_json::to_vec(&r).unwrap();
                bytes.push(b'\n');
                socket.write_all(&bytes).unwrap();
            }
        });
        let mut c = Client::connect(&path, "synthetic", p::Role::Agent).unwrap();
        let old = c.session_watch_open("session1").unwrap();
        c.session_watch_close().unwrap();
        let reopened = c.session_watch_open("session1");
        if reuse {
            assert!(reopened.is_err());
            assert!(c.transport_failed());
            let n = c.next_id();
            assert!(c.session_watch_next(&old).is_err());
            assert_eq!(c.next_id(), n);
        } else {
            let new = reopened.unwrap();
            let n = c.next_id();
            assert!(c.session_watch_next(&old).is_err());
            assert_eq!(c.next_id(), n);
            assert!(matches!(
                c.session_watch_next(&new).unwrap().body,
                p::WatchBody::Unchanged {}
            ));
        }
        drop(c);
        server.join().unwrap();
        assert_eq!(count.load(Ordering::SeqCst), if reuse { 5 } else { 6 });
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}

#[test]
fn successful_open_identifier_memory_is_bounded_before_next_dispatch() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.artifacts")
        .join(format!(
            "swb-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("s");
    let listener = UnixListener::bind(&path).unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let seen = count.clone();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut reader = BufReader::new(socket.try_clone().unwrap());
        for step in 0..2 + usize::from(p::WATCH_MAX_OPENS) * 2 {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let q: p::Request = serde_json::from_str(&line).unwrap();
            seen.fetch_add(1, Ordering::SeqCst);
            let mut r = json!({"id":q.id,"ok":true,"error":null,"receipt":null,"snapshot":null,"endpoint":null,"resume":null,"epoch":"a".repeat(64)});
            if step == 0 {
                r["endpoint"] = json!("agent");
                r["resume"] = json!("synthetic");
            } else if step == 1 {
                r["snapshot"] = json!({"version":p::VERSION,"epoch":"a".repeat(64),"endpoint":"agent","capabilities":[p::SESSION_WATCH],"tasks":[],"endpoints":[],"sessions":[],"attempts":[],"artifacts":[],"receipts":[],"degraded":null});
            } else {
                r["watch"] = frame(1, if step % 2 == 0 { "baseline" } else { "closed" });
                r["watch"]["subscription"] = json!(format!("sub-{}", (step - 2) / 2));
            }
            let mut bytes = serde_json::to_vec(&r).unwrap();
            bytes.push(b'\n');
            socket.write_all(&bytes).unwrap();
        }
    });
    let mut c = Client::connect(&path, "synthetic", p::Role::Agent).unwrap();
    for _ in 0..p::WATCH_MAX_OPENS {
        c.session_watch_open("session1").unwrap();
        c.session_watch_close().unwrap();
    }
    let n = c.next_id();
    assert!(c.session_watch_open("session1").is_err());
    assert_eq!(c.next_id(), n);
    assert!(!c.transport_failed());
    drop(c);
    server.join().unwrap();
    assert_eq!(
        count.load(Ordering::SeqCst),
        2 + usize::from(p::WATCH_MAX_OPENS) * 2
    );
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_dir(dir).unwrap();
}
