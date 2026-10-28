use crate::mk1::{AttemptTarget, Client};
use crate::mk1_protocol as p;
use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::sync::atomic::{AtomicUsize, Ordering};
static NEXT: AtomicUsize = AtomicUsize::new(0);
#[test]
fn cancellation_negotiates_and_fences_wrong_results_without_replay() {
    for case in [
        "success",
        "unsupported",
        "stale_origin",
        "stale_epoch",
        "receipt_id",
        "receipt_endpoint",
        "receipt_op",
        "result_attempt",
        "missing_receipt",
        "refused",
        "unknown_good",
        "unknown_other",
        "error_other",
    ] {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.artifacts")
            .join(format!(
                "c70-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("s");
        let listener = UnixListener::bind(&path).unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .unwrap();
            let mut read = BufReader::new(stream.try_clone().unwrap());
            for step in 0..3 {
                let mut line = String::new();
                read.read_line(&mut line).unwrap();
                let q: p::Request = serde_json::from_str(&line).unwrap();
                let mut r = p::Response {
                    id: q.id,
                    ok: true,
                    error: None,
                    receipt: None,
                    snapshot: None,
                    endpoint: None,
                    resume: None,
                    epoch: "a".repeat(64),
                };
                if step == 0 {
                    r.endpoint = Some("agent".into());
                    r.resume = Some("synthetic".into());
                }
                if step == 1 {
                    assert!(matches!(q.operation, p::Op::Observe {}));
                    r.snapshot=Some(serde_json::from_value(json!({"version":p::VERSION,"epoch":r.epoch,"endpoint":"agent","capabilities":if case=="unsupported"{vec![]}else{vec![p::ATTEMPT_CANCEL]},"endpoints":[],"sessions":[],"tasks":[],"artifacts":[],"receipts":[],"degraded":null,"attempts":[{"id":"attempt1","task":"test","task_revision":null,"origin":{"endpoint":"agent","request_id":if case=="stale_origin"{99}else{2}},"state":"running","exit_code":null,"stdout":"","stderr":"","truncated":false}]})).unwrap());
                }
                if step == 2 {
                    assert!(matches!(q.operation, p::Op::CancelAttempt { .. }));
                    r.ok = !["refused", "unknown_good", "unknown_other", "error_other"]
                        .contains(&case);
                    r.error = if r.ok {
                        None
                    } else {
                        Some("cancel_refused".into())
                    };
                    if case != "missing_receipt" {
                        r.receipt = Some(p::Receipt {
                            endpoint: if case == "receipt_endpoint" {
                                "other"
                            } else {
                                "agent"
                            }
                            .into(),
                            id: if case == "receipt_id" { 99 } else { q.id },
                            operation: if case == "receipt_op" {
                                "run"
                            } else {
                                "cancel_attempt"
                            }
                            .into(),
                            state: if case.starts_with("unknown_") {
                                "unknown"
                            } else if r.ok {
                                "applied"
                            } else {
                                "rejected"
                            }
                            .into(),
                            detail: "fixture".into(),
                            delivered_bytes: 0,
                            result: if r.ok
                                || ["unknown_good", "unknown_other", "error_other"].contains(&case)
                            {
                                Some(p::MutationResult::CancellationRequested {
                                    attempt: if ["result_attempt", "unknown_other", "error_other"]
                                        .contains(&case)
                                    {
                                        "other"
                                    } else {
                                        "attempt1"
                                    }
                                    .into(),
                                    status: p::CancelRequestStatus::Requested,
                                })
                            } else {
                                None
                            },
                        });
                    }
                }
                let mut bytes = serde_json::to_vec(&r).unwrap();
                bytes.push(b'\n');
                stream.write_all(&bytes).unwrap();
                if step == 1 && ["unsupported", "stale_origin", "stale_epoch"].contains(&case) {
                    break;
                }
            }
            // Client may be fenced or dropped; no implicit suffix/reconnect/retry.
            let mut rest = String::new();
            assert_eq!(read.read_line(&mut rest).unwrap(), 0);
        });
        let mut client = Client::connect(&path, "test", p::Role::Agent).unwrap();
        let target = AttemptTarget {
            epoch: if case == "stale_epoch" {
                "b".repeat(64)
            } else {
                "a".repeat(64)
            },
            attempt: "attempt1".into(),
            origin_endpoint: "agent".into(),
            origin_request_id: 2,
        };
        let result = client.cancel_attempt(&target);
        match case {
            "success" => assert!(result.unwrap().ok),
            "refused" | "unknown_good" => {
                assert!(!result.unwrap().ok);
                assert!(!client.transport_failed());
            }
            "unsupported" | "stale_origin" | "stale_epoch" => assert!(result.is_err()),
            _ => {
                assert!(result.is_err());
                assert!(client.transport_failed());
                assert!(client.call(p::Op::Observe {}).is_err());
            }
        }
        drop(client);
        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
