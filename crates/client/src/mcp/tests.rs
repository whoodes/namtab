use super::*;
fn request(id: Value, method: &str, params: Value) -> Vec<u8> {
    serde_json::to_vec(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params})).unwrap()
}
fn init(s: &mut Session) {
    let r=s.handle(&request(json!(1),"initialize",json!({"protocolVersion":"2025-11-25","clientInfo":{"name":"fixture","version":"0"},"capabilities":{}})),&mut |_,_|panic!("no broker calls at init")).unwrap();
    assert_eq!(r["result"]["protocolVersion"], "2025-11-25");
    assert!(
        s.handle(
            br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            &mut |_, _| panic!()
        )
        .is_none()
    );
}
#[test]
fn lifecycle_ids_notifications_and_unsupported_authority_never_dispatch() {
    let mut s = Session::default();
    let mut calls = 0;
    let mut call = |_: &str, _: Value| {
        calls += 1;
        Ok(json!({}))
    };
    assert!(
        s.handle(&request(json!(0), "tools/list", json!({})), &mut call)
            .unwrap()
            .get("error")
            .is_some()
    );
    init(&mut s);
    let r = request(
        json!(2),
        "tools/call",
        json!({"name":"namtab_observe","arguments":{}}),
    );
    assert_eq!(s.handle(&r, &mut call).unwrap()["result"]["isError"], false);
    assert!(s.handle(&r, &mut call).unwrap().get("error").is_some());
    assert!(
        s.handle(
            br#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"namtab_run"}}"#,
            &mut call
        )
        .is_none()
    );
    assert!(
        s.handle(
            &request(
                json!(3),
                "tools/call",
                json!({"name":"shutdown","arguments":{}})
            ),
            &mut call
        )
        .unwrap()
        .get("error")
        .is_some()
    );
    assert_eq!(calls, 1);
}
#[test]
fn exact_revision_and_agent_input_policy_cannot_be_widened() {
    assert!(tools::operation("namtab_run", json!({"task":"x","argv":["/bin/true"]})).is_err());
    assert!(
        tools::operation(
            "namtab_claim_terminal",
            json!({"session":"s","generation":1,"force":true})
        )
        .is_err()
    );
    let q=tools::operation("namtab_input",json!({"session":"s","generation":1,"revision":2,"mode_revision":3,"intent":{"kind":"key","key":"Enter","ctrl":false,"alt":false,"shift":false}})).unwrap();
    assert!(matches!(
        q,
        Op::Input {
            policy: p::InputPolicy::Observed,
            ..
        }
    ));
    assert!(tools::operation("namtab_input",json!({"session":"s","generation":1,"revision":2,"mode_revision":3,"intent":{"kind":"text","text":"x"},"policy":"human_live"})).is_err());
}
#[test]
fn complete_line_and_request_namespace_are_bounded() {
    assert!(line(&mut io::Cursor::new(b"{}")).is_err());
    assert!(line(&mut io::Cursor::new(vec![b'x'; MAX_LINE + 1])).is_err());
    assert_eq!(
        line(&mut io::Cursor::new(b"{}\r\n")).unwrap().unwrap(),
        b"{}\r\n"
    );
    let mut s = Session::default();
    for id in 0..MAX_REQUESTS {
        assert!(
            s.handle(&request(json!(id), "ping", json!({})), &mut |_, _| panic!())
                .unwrap()
                .get("result")
                .is_some()
        );
    }
    assert!(
        s.handle(
            &request(json!(999), "ping", json!({})),
            &mut |_, _| panic!()
        )
        .unwrap()
        .get("error")
        .is_some()
    );
    assert!(s.exhausted());
}
#[test]
fn tool_errors_are_not_success_and_unknown_effects_are_not_retried() {
    let mut s = Session::default();
    init(&mut s);
    let mut calls = 0;
    let mut call = |_: &str, _: Value| {
        calls += 1;
        Err(json!({"effects":"unknown"}))
    };
    let q = request(
        json!(2),
        "tools/call",
        json!({"name":"namtab_observe","arguments":{}}),
    );
    assert_eq!(s.handle(&q, &mut call).unwrap()["result"]["isError"], true);
    s.handle(&q, &mut call);
    assert_eq!(calls, 1);
}

#[test]
fn known_tool_validation_errors_are_tool_results_without_dispatch() {
    let mut s = Session::default();
    init(&mut s);
    let r = s
        .handle(
            &request(
                json!(2),
                "tools/call",
                json!({"name":"namtab_run","arguments":{"task":"fixture","argv":["/bin/true"]}}),
            ),
            &mut |_, _| panic!("invalid args cannot dispatch"),
        )
        .unwrap();
    assert_eq!(r["result"]["isError"], true);
    assert_eq!(
        r["result"]["structuredContent"]["effects"],
        "none_dispatched"
    );
    let schema = tools::catalog()
        .into_iter()
        .find(|x| x["name"] == "namtab_observe")
        .unwrap();
    assert_eq!(schema["inputSchema"]["maxProperties"], 1);
    assert!(
        tools::operation(
            "namtab_task",
            json!({"task":"fixture","goal":"synthetic","expected_artifacts":["a".repeat(100)]})
        )
        .is_ok()
    );
}
#[test]
fn wrong_or_missing_broker_receipts_fence_the_real_client() {
    use std::{
        io::BufReader,
        os::unix::net::UnixListener,
        sync::atomic::{AtomicUsize, Ordering},
    };
    static ID: AtomicUsize = AtomicUsize::new(0);
    for case in [
        "endpoint",
        "id",
        "operation",
        "missing",
        "snapshot_epoch",
        "snapshot_endpoint",
        "snapshot_version",
    ] {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.artifacts")
            .join(format!(
                "mcp-r-{}-{}",
                std::process::id(),
                ID.fetch_add(1, Ordering::Relaxed)
            ));
        std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("s");
        let listener = UnixListener::bind(&path).unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
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
                    epoch: "fixture-epoch".into(),
                };
                if step == 0 {
                    r.endpoint = Some("fixture-agent".into());
                    r.resume = Some("synthetic-ram-only".into());
                }
                if step == 1 {
                    r.snapshot = Some(p::Snapshot {
                        version: p::VERSION.into(),
                        epoch: "fixture-epoch".into(),
                        endpoint: "fixture-agent".into(),
                        capabilities: vec![
                            p::SHARED_TASKS.into(),
                            p::ATTEMPT_ORIGIN.into(),
                            p::RECEIPT_RESULTS.into(),
                        ],
                        endpoints: vec![],
                        tasks: vec![],
                        sessions: vec![],
                        attempts: vec![],
                        artifacts: vec![],
                        receipts: vec![],
                        degraded: None,
                    });
                }
                if step == 2 && case != "missing" {
                    r.receipt = Some(p::Receipt {
                        endpoint: if case == "endpoint" {
                            "wrong"
                        } else {
                            "fixture-agent"
                        }
                        .into(),
                        id: if case == "id" { 999 } else { q.id },
                        operation: if case == "operation" {
                            "run"
                        } else {
                            "create_session"
                        }
                        .into(),
                        state: "applied".into(),
                        detail: "synthetic bad reply".into(),
                        delivered_bytes: 0,
                        result: None,
                    });
                }
                if step == 2 && case.starts_with("snapshot_") {
                    let mut snap: p::Snapshot = serde_json::from_value(json!({"version":p::VERSION,"epoch":"fixture-epoch","endpoint":"fixture-agent","capabilities":[],"endpoints":[],"tasks":[],"sessions":[],"attempts":[],"artifacts":[],"receipts":[],"degraded":null})).unwrap();
                    match case {
                        "snapshot_epoch" => snap.epoch = "other-epoch".into(),
                        "snapshot_endpoint" => snap.endpoint = "other-agent".into(),
                        "snapshot_version" => snap.version = "unknown".into(),
                        _ => unreachable!(),
                    }
                    r.snapshot = Some(snap);
                }
                let mut raw = serde_json::to_vec(&r).unwrap();
                raw.push(b'\n');
                stream.write_all(&raw).unwrap();
            }
        });
        let mut bridge = Bridge::connect(&path, "fixture-epoch").unwrap();
        let e = bridge
            .call("namtab_create_terminal", json!({"name":"synthetic"}))
            .unwrap_err();
        assert_eq!(
            e["error"],
            if case.starts_with("snapshot_") {
                "broker_transport_unknown"
            } else {
                "receipt_binding_mismatch"
            }
        );
        assert!(bridge.client.transport_failed());
        assert_eq!(
            bridge.call("namtab_observe", json!({})).unwrap_err()["error"],
            "connection_fenced"
        );
        assert_eq!(bridge.mutations.get(&3).unwrap(), "create_session");
        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}

#[test]
fn protocol_versions_negotiate_and_tools_wait_for_initialized() {
    for (requested, selected) in [("2025-06-18", "2025-06-18"), ("unsupported", "2025-11-25")] {
        let mut s = Session::default();
        let r=s.handle(&request(json!(1),"initialize",json!({"protocolVersion":requested,"clientInfo":{"name":"fixture","version":"0"},"capabilities":{}})),&mut |_,_|panic!()).unwrap();
        assert_eq!(r["result"]["protocolVersion"], selected);
        let r = s
            .handle(
                &request(json!(2), "tools/list", json!({})),
                &mut |_, _| panic!(),
            )
            .unwrap();
        assert!(r.get("error").is_some());
    }
}
#[test]
fn observations_scope_work_and_mark_preview_truncation() {
    let s:p::Snapshot=serde_json::from_value(json!({"version":p::VERSION,"epoch":"fixture","endpoint":"agent","endpoints":[],"capabilities":[],"tasks":[],"sessions":[],"artifacts":[],"receipts":[],"degraded":null,"attempts":[{"id":"a","task":"t","task_revision":null,"origin":{"endpoint":"agent","request_id":3},"state":"running","exit_code":null,"stdout":"PRIVATE_STREAM".repeat(400),"stderr":"ERR","truncated":false}]})).unwrap();
    let summary = tools::observation(s.clone(), &json!({})).unwrap();
    assert!(!summary.to_string().contains("PRIVATE_STREAM"));
    assert!(summary.get("endpoints").is_none());
    assert!(summary.get("receipts").is_none());
    let detail = tools::observation(s.clone(), &json!({"attempt":"a"})).unwrap();
    assert_eq!(
        detail["stdout_preview"].as_str().unwrap().chars().count(),
        4096
    );
    assert_eq!(detail["preview_truncated"], true);
    assert_eq!(detail["source_truncated"], false);
    assert!(tools::observation(s, &json!({"attempt":"missing"})).is_err());
}
