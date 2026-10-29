use super::*;
fn object(properties: Value, required: Value) -> Value {
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}
fn text(max: usize) -> Value {
    json!({"type":"string","minLength":1,"maxLength":max})
}
fn number() -> Value {
    json!({"type":"integer","minimum":0,"maximum":p::MAX_ID})
}
pub(super) fn catalog() -> Vec<Value> {
    let intent = json!({"oneOf":[object(json!({"kind":{"const":"text"},"text":text(8192)}),json!(["kind","text"])),object(json!({"kind":{"const":"paste"},"text":text(8192)}),json!(["kind","text"])),object(json!({"kind":{"const":"key"},"key":text(32),"ctrl":{"type":"boolean"},"alt":{"type":"boolean"},"shift":{"type":"boolean"}}),json!(["kind","key","ctrl","alt","shift"]))]});
    let rows = [
        (
            "namtab_observe",
            "Read scoped work facts with fresh registered artifact metadata, one attempt or one terminal. No inference of agent reasoning. Truncated previews are not full capture.",
            {
                let mut schema = object(json!({"attempt":text(80),"session":text(80)}), json!([]));
                schema["maxProperties"] = json!(1);
                schema
            },
            true,
        ),
        (
            "namtab_task",
            "Define one immutable shared task contract; preserve returned exact revision for run. Not task acceptance.",
            object(
                json!({"task":text(80),"goal":text(512),"expected_artifacts":{"type":"array","maxItems":2,"items":text(512)}}),
                json!(["task", "goal", "expected_artifacts"]),
            ),
            false,
        ),
        (
            "namtab_run",
            "Start literal argv with exact task revision. Receipt means start, not completion. Poll exact attempt; never retry uncertain results. Developer host, not sandbox.",
            object(
                json!({"task":text(80),"task_revision":text(71),"argv":{"type":"array","minItems":1,"maxItems":32,"items":{"type":"string","maxLength":8192}}}),
                json!(["task", "task_revision", "argv"]),
            ),
            false,
        ),
        (
            "namtab_create_terminal",
            "Create one real PTY; shares existing two-lifetime-session limit.",
            object(json!({"name":text(80)}), json!(["name"])),
            false,
        ),
        (
            "namtab_claim_terminal",
            "Claim only an unowned terminal at exact generation. Cannot force human takeover.",
            object(
                json!({"session":text(80),"generation":number()}),
                json!(["session", "generation"]),
            ),
            false,
        ),
        (
            "namtab_release_terminal",
            "Release exact owned generation; does not stop running processes.",
            object(
                json!({"session":text(80),"generation":number()}),
                json!(["session", "generation"]),
            ),
            false,
        ),
        (
            "namtab_input",
            "Deliver one observed-context intent: exact session, writer generation, screen revision, mode revision. No HumanLive/raw bypass. Never retry partial/unknown input.",
            object(
                json!({"session":text(80),"generation":number(),"revision":number(),"mode_revision":number(),"intent":intent}),
                json!([
                    "session",
                    "generation",
                    "revision",
                    "mode_revision",
                    "intent"
                ]),
            ),
            false,
        ),
        (
            "namtab_artifact",
            "Observe bounded workspace file revision; not sealed causality, correctness or human approval.",
            object(
                json!({"task":text(80),"path":text(4096)}),
                json!(["task", "path"]),
            ),
            false,
        ),
        (
            "namtab_receipt",
            "Read a known broker request receipt in this exact endpoint namespace; not another endpoint's request.",
            object(json!({"request_id":number()}), json!(["request_id"])),
            true,
        ),
    ];
    rows.into_iter().map(|(name,description,input,read)|json!({"name":name,"description":description,"inputSchema":input,"annotations":{"readOnlyHint":read,"destructiveHint":!read,"idempotentHint":false,"openWorldHint":true}})).collect()
}
pub(super) fn operation(name: &str, args: Value) -> Result<Op, &'static str> {
    let schema = catalog()
        .into_iter()
        .find(|t| t["name"] == name)
        .ok_or("unknown_tool")?;
    let obj = args.as_object().ok_or("arguments_object")?;
    let allowed = schema["inputSchema"]["properties"].as_object().unwrap();
    if obj.keys().any(|k| !allowed.contains_key(k))
        || schema["inputSchema"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|k| !obj.contains_key(k.as_str().unwrap()))
    {
        return Err("argument_fields");
    }
    let mut args = args;
    let op = match name {
        "namtab_observe" => {
            let map = args.as_object().unwrap();
            if map.len() > 1
                || map
                    .values()
                    .any(|v| !v.as_str().is_some_and(|s| !s.is_empty() && s.len() <= 80))
            {
                return Err("one_observation_target");
            }
            // Validation marker only: Bridge resolves the exact negotiated scope
            // after checking its transport. This is never a legacy read fallback.
            return Ok(Op::Observe {});
        }
        "namtab_task" => "define_task",
        "namtab_run" => {
            if !args["task_revision"].is_string() {
                return Err("exact_revision_required");
            }
            "run"
        }
        "namtab_create_terminal" => "create_session",
        "namtab_claim_terminal" => {
            args["force"] = json!(false);
            "take_control"
        }
        "namtab_release_terminal" => "release_control",
        "namtab_input" => {
            args["policy"] = json!("observed");
            "input"
        }
        "namtab_artifact" => "artifact",
        "namtab_receipt" => "receipt",
        _ => return Err("unknown_tool"),
    };
    args["op"] = json!(op);
    let raw = serde_json::to_vec(&json!({"version":p::VERSION,"id":1,"operation":args}))
        .map_err(|_| "encoding")?;
    let q = p::decode(&raw)?;
    // MK1's original decoder bounds only selected identifiers; enforce this
    // adapter's declared exact-int resource profile as well.
    for key in ["generation", "revision", "mode_revision", "request_id"] {
        if let Some(v) = args.get(key)
            && v.as_u64().is_none_or(|n| n > p::MAX_ID)
        {
            return Err("bounded_integer");
        }
    }
    for key in ["session", "name", "task"] {
        if let Some(v) = args.get(key)
            && !v
                .as_str()
                .is_some_and(|s| !s.is_empty() && s.len() <= 80 && !s.chars().any(char::is_control))
        {
            return Err("bounded_identity");
        }
    }
    Ok(q.operation)
}
pub(super) fn observation(s: p::ScopedObservation) -> Result<Value, &'static str> {
    match s.view {
        p::ObservationView::Attempt { attempt: a } => {
            let stdout: String = a.stdout.chars().take(4096).collect();
            let stderr: String = a.stderr.chars().take(4096).collect();
            Ok(
                json!({"attempt":a.id,"task":a.task,"task_revision":a.task_revision,"origin":a.origin,"state":a.state,"exit_code":a.exit_code,"stdout_preview":stdout,"stderr_preview":stderr,"preview_truncated":stdout.len()<a.stdout.len()||stderr.len()<a.stderr.len(),"source_truncated":a.truncated}),
            )
        }
        p::ObservationView::Session { session } => Ok(serde_json::to_value(session).unwrap()),
        p::ObservationView::WorkExport {
            capabilities,
            tasks,
            sessions,
            attempts,
            artifacts,
            ..
        } => Ok(
            json!({"epoch":s.epoch,"degraded":s.degraded,"capabilities":capabilities,"tasks":tasks,"sessions":sessions.into_iter().map(|x|json!({"id":x.id,"name":x.name,"owner":x.owner,"generation":x.generation,"process":x.process,"exit_code":x.exit_code})).collect::<Vec<_>>(),"attempts":attempts.into_iter().map(|a|json!({"id":a.id,"task":a.task,"task_revision":a.task_revision,"origin":a.origin,"state":a.state,"exit_code":a.exit_code,"source_truncated":a.source_truncated})).collect::<Vec<_>>(),"artifacts":artifacts}),
        ),
        p::ObservationView::Work { .. } => Err("fresh_work_export_required"),
    }
}
