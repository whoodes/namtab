//! Fake peers exercise the real adapter/public transport, not a mocked tool callback.
use super::*;
use std::{
    os::unix::net::UnixListener,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
fn peer(case: &'static str) -> (std::path::PathBuf, std::thread::JoinHandle<Vec<Value>>) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.artifacts")
        .join(format!(
            "ms-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("s");
    let listener = UnixListener::bind(&path).unwrap();
    let handle = std::thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        let mut reader = io::BufReader::new(s.try_clone().unwrap());
        let mut requests = vec![];
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 {
                break;
            }
            let q: Value = serde_json::from_str(&line).unwrap();
            requests.push(q.clone());
            let n = requests.len();
            let mut r = json!({"id":q["id"],"ok":true,"error":null,"receipt":null,"snapshot":null,"endpoint":"fixture-agent","resume":null,"epoch":"fixture-epoch"});
            if n == 1 {
                assert_eq!(q["operation"]["op"], "hello");
                r["resume"] = json!("synthetic-ram-only");
            } else if n == 2 {
                assert_eq!(q["operation"]["op"], "observe");
                let mut caps = vec![
                    p::SHARED_TASKS,
                    p::ATTEMPT_ORIGIN,
                    p::RECEIPT_RESULTS,
                    p::SCOPED_OBSERVE,
                    p::SCOPED_WORK_EXPORT,
                ];
                if case == "unsupported" {
                    caps.retain(|x| *x != p::SCOPED_OBSERVE)
                }
                if case == "missing_export_cap" {
                    caps.retain(|x| *x != p::SCOPED_WORK_EXPORT)
                }
                r["snapshot"] = json!({"version":p::VERSION,"epoch":"fixture-epoch","endpoint":"fixture-agent","capabilities":caps,"endpoints":[],"tasks":[],"sessions":[],"attempts":[],"artifacts":[],"receipts":[],"degraded":null});
                if case == "discovery_missing" {
                    r["snapshot"] = Value::Null
                }
                if case == "discovery_resume" {
                    r["resume"] = json!("must-not-export")
                }
                if case == "discovery_receipt" {
                    r["receipt"] = json!({"endpoint":"fixture-agent","id":2,"operation":"run","state":"applied","detail":"not a read","delivered_bytes":0})
                }
            } else {
                assert_eq!(q["operation"]["op"], "observe_scoped");
                assert_eq!(q["operation"]["epoch"], "fixture-epoch");
                if case == "disconnect" {
                    break;
                }
                let scope = &q["operation"]["scope"];
                let kind = scope["kind"].as_str().unwrap();
                let view = match kind {
                    "work_export" => {
                        json!({"kind":kind,"capabilities":[],"tasks":[],"sessions":[],"attempts":[],"artifacts":[]})
                    }
                    "attempt" => {
                        json!({"kind":kind,"attempt":{"id":scope["attempt"],"task":"fixture","task_revision":null,"origin":{"endpoint":"producer","request_id":12},"state":"running","exit_code":null,"stdout":"ONLY_SELECTED","stderr":"ERR","truncated":false}})
                    }
                    "session" => {
                        json!({"kind":kind,"session":{"id":scope["session"],"name":"fixture","owner":null,"generation":1,"process":"running","exit_code":null,"screen":{"cols":2,"rows":1,"lines":["ok"],"cursor_col":0,"cursor_row":0,"cursor_visible":true,"revision":3,"mode_revision":2,"alternate":false,"input_known":true,"diagnostics":[]}}})
                    }
                    _ => panic!("unexpected scope"),
                };
                r["observation"] = json!({"version":p::VERSION,"profile":p::OBSERVATION_PROFILE,"epoch":"fixture-epoch","endpoint":"fixture-agent","degraded":null,"view":view});
                if n == 3 {
                    match case {
                        "profile" => r["observation"]["profile"] = json!("wrong/0"),
                        "epoch" => r["epoch"] = json!("stale-epoch"),
                        "endpoint" => r["observation"]["endpoint"] = json!("other"),
                        "target" => {
                            r["observation"]["view"]["attempt"]["id"] = json!("other-attempt")
                        }
                        "scope" | "work_scope" => {
                            r["observation"]["view"] = json!({"kind":"work","endpoints":[],"capabilities":[],"tasks":[],"sessions":[],"attempts":[]})
                        }
                        "mixed" => {
                            r["receipt"] = json!({"endpoint":"fixture-agent","id":q["id"],"operation":"run","state":"applied","detail":"unrequested","delivered_bytes":0})
                        }
                        "missing" => r["observation"] = Value::Null,
                        "refused" => {
                            r["ok"] = json!(false);
                            r["error"] = json!("unknown_attempt");
                            r["observation"] = Value::Null;
                        }
                        _ => {}
                    }
                }
            }
            let mut bytes = serde_json::to_vec(&r).unwrap();
            bytes.push(b'\n');
            s.write_all(&bytes).unwrap();
        }
        requests
    });
    (path, handle)
}
#[test]
fn mcp_reads_negotiate_once_export_exact_scopes_and_fence_bad_peers_without_fallback() {
    for case in [
        "success",
        "unsupported",
        "missing_export_cap",
        "discovery_missing",
        "discovery_resume",
        "discovery_receipt",
        "profile",
        "epoch",
        "endpoint",
        "target",
        "scope",
        "work_scope",
        "mixed",
        "missing",
        "refused",
        "disconnect",
    ] {
        let (path, server) = peer(case);
        let connected = Bridge::connect(&path, "fixture-epoch");
        let count = if case == "unsupported"
            || case == "missing_export_cap"
            || case.starts_with("discovery_")
        {
            assert!(connected.is_err(), "{case}");
            2
        } else {
            let mut b = connected.unwrap();
            let args = if case == "work_scope" {
                json!({})
            } else {
                json!({"attempt":"attempt-A"})
            };
            let r = b.call("namtab_observe", args);
            if case == "success" {
                let r = r.unwrap();
                assert_eq!(r["profile"], PROFILE);
                assert_eq!(r["binding"]["broker_request_id"], 3);
                assert_eq!(
                    r["observation_or_response"]["stdout_preview"],
                    "ONLY_SELECTED"
                );
                assert_eq!(
                    r["observation_or_response"]["origin"]["endpoint"],
                    "producer"
                );
                assert!(!r.to_string().contains("synthetic-ram-only"));
                let work = b.call("namtab_observe", json!({})).unwrap();
                assert_eq!(work["binding"]["broker_request_id"], 4);
                assert_eq!(work["observation_or_response"]["artifacts"], json!([]));
                let session = b
                    .call("namtab_observe", json!({"session":"session-A"}))
                    .unwrap();
                assert_eq!(session["observation_or_response"]["id"], "session-A");
                assert_eq!(session["binding"]["broker_request_id"], 5);
                5
            } else if case == "refused" {
                assert_eq!(r.unwrap_err()["error"], "unknown_attempt");
                assert!(!b.client.transport_failed());
                b.call("namtab_observe", json!({})).unwrap();
                4
            } else {
                assert_eq!(
                    r.unwrap_err()["error"],
                    "broker_transport_unknown",
                    "{case}"
                );
                assert!(b.client.transport_failed());
                assert_eq!(
                    b.call("namtab_observe", json!({})).unwrap_err()["error"],
                    "connection_fenced"
                );
                3
            }
        };
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), count, "{case}");
        assert_eq!(
            requests
                .iter()
                .filter(|r| r["operation"]["op"] == "observe")
                .count(),
            1
        );
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(path.parent().unwrap()).unwrap();
    }
}
#[test]
fn a_fresh_bridge_renegotiates_and_does_not_replay_the_old_connection() {
    // Two explicit starts, each own endpoint/namespace/discovery; no automatic reconnect.
    for case in ["success", "missing_export_cap"] {
        let (path, server) = peer(case);
        let bridge = Bridge::connect(&path, "fixture-epoch");
        if case == "success" {
            let mut b = bridge.unwrap();
            b.call("namtab_observe", json!({})).unwrap();
        } else {
            assert!(bridge.is_err())
        }
        let requests = server.join().unwrap();
        assert_eq!(requests[0]["operation"]["resume"], Value::Null);
        assert_eq!(requests[1]["operation"]["op"], "observe");
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(path.parent().unwrap()).unwrap();
    }
}
