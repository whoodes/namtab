//! Explicit scripted protocol workflow, not an autonomous model/provider adapter.
use project_x_client::{
    mk1::Client,
    mk1_protocol::{self as p, InputPolicy, Intent, MutationResult, Op, Role, Snapshot},
};
use std::{
    path::Path,
    time::{Duration, Instant},
};
fn call(c: &mut Client, op: Op) -> Result<p::Response, Box<dyn std::error::Error>> {
    let r = c.call(op)?;
    if !r.ok || r.receipt.as_ref().is_some_and(|r| r.state != "applied") {
        return Err(format!(
            "scripted operation refused/unknown, not retried: {}",
            r.error
                .unwrap_or_else(|| r.receipt.map_or("no result".into(), |r| r.detail))
        )
        .into());
    }
    Ok(r)
}
fn observe(c: &mut Client) -> Result<Snapshot, Box<dyn std::error::Error>> {
    call(c, Op::Observe {})?
        .snapshot
        .ok_or_else(|| "observation absent".into())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 || args[1] != "--socket" || args[3] != "--epoch" {
        return Err("usage: namtab-demo --socket PUBLIC_SOCKET --epoch OBSERVED_EPOCH".into());
    }
    let path = Path::new(&args[2]);
    let mut a = Client::connect(path, "scripted-A", Role::Agent)?;
    if a.epoch != args[4] {
        return Err("observed broker epoch changed; no scripted effects admitted".into());
    }
    let mut b = Client::connect(path, "scripted-B", Role::Agent)?;
    if b.epoch != args[4] {
        return Err("scripted endpoints do not share the observed epoch".into());
    }
    let s = observe(&mut a)?;
    if ![p::SHARED_TASKS, p::ATTEMPT_ORIGIN, p::RECEIPT_RESULTS]
        .iter()
        .all(|cap| s.capabilities.iter().any(|v| v == cap))
    {
        return Err("required advertised task/origin capabilities absent".into());
    }
    if !s.attempts.is_empty() || s.sessions.len() > 1 {
        return Err("Demo requires a fresh namespace with zero managed attempts and at most one shell; no capacity expansion or retry".into());
    }
    let task = "scripted-demo";
    let definition = call(
        &mut a,
        Op::DefineTask {
            task: task.into(),
            goal:
                "Two scripted clients produce independent observations for human review; no model."
                    .into(),
            expected_artifacts: vec!["scripted-result.txt".into()],
        },
    )?;
    let revision = match definition.receipt.and_then(|r| r.result) {
        Some(MutationResult::TaskDefined { revision, .. }) => revision,
        _ => return Err("typed task revision missing".into()),
    };
    let first=call(&mut a,Op::Run{task:task.into(),task_revision:Some(revision.clone()),argv:vec!["/bin/sh".into(),"-c".into(),"set -C; printf 'scripted A artifact\n' > scripted-result.txt || exit 2; printf 'producer stdout\n'; printf 'producer stderr\n' >&2".into()]})?;
    let second = call(
        &mut b,
        Op::Run {
            task: task.into(),
            task_revision: Some(revision.clone()),
            argv: vec![
                "/usr/bin/printf".into(),
                "independent scripted B stdout\n".into(),
            ],
        },
    )?;
    let ids: Vec<_> = [first, second]
        .into_iter()
        .map(|r| match r.receipt.and_then(|r| r.result) {
            Some(MutationResult::AttemptStarted { attempt }) => Ok(attempt),
            _ => Err("typed attempt result missing"),
        })
        .collect::<Result<_, _>>()?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let s = observe(&mut a)?;
        if ids.iter().all(|id| {
            s.attempts.iter().any(|a| {
                a.id == *id && a.state == "exited" && a.exit_code == Some(0) && !a.truncated
            })
        }) {
            break;
        }
        if Instant::now() >= deadline {
            return Err("scripted process result incomplete/failed; no retry".into());
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    call(
        &mut a,
        Op::Artifact {
            task: task.into(),
            path: "scripted-result.txt".into(),
        },
    )?;
    println!(
        "{}",
        serde_json::json!({"event":"scripted_attempts_observed","attempts":ids,"task_revision":revision,"limit":"file observation is not sealed causality or correctness; B is independent, not a consumer dependency"})
    );
    let before = observe(&mut a)?;
    let previous: Vec<_> = before.sessions.iter().map(|s| s.id.clone()).collect();
    call(
        &mut a,
        Op::CreateSession {
            name: "scripted-shell".into(),
        },
    )?;
    let s = observe(&mut a)?;
    let session = s
        .sessions
        .iter()
        .find(|s| !previous.contains(&s.id))
        .ok_or("new scripted shell unavailable")?;
    let id = session.id.clone();
    call(
        &mut a,
        Op::TakeControl {
            session: id.clone(),
            generation: session.generation,
            force: false,
        },
    )?;
    // No application input is sent until an explicitly observed stable startup cut.
    let until = Instant::now() + Duration::from_secs(3);
    let mut last = None;
    let shell = loop {
        let s = observe(&mut a)?;
        let shell = s
            .sessions
            .into_iter()
            .find(|s| s.id == id)
            .ok_or("scripted shell unavailable")?;
        if shell.screen.input_known && last == Some(shell.screen.revision) {
            break shell;
        }
        last = Some(shell.screen.revision);
        if Instant::now() >= until {
            return Err("startup cut unavailable; no input retried".into());
        }
        std::thread::sleep(Duration::from_millis(30));
    };
    let generation = shell.generation;
    println!(
        "{}",
        serde_json::json!({"event":"scripted_shell_owned","session":id,"endpoint":a.endpoint,"generation":generation,"instruction":"Select this shell, then human Take input. No model is running."})
    );
    let until = Instant::now() + Duration::from_secs(30);
    while Instant::now() < until {
        let s = observe(&mut a)?;
        let current = s
            .sessions
            .iter()
            .find(|s| s.id == id)
            .ok_or("scripted shell unavailable")?;
        if current.owner.as_deref() != Some(&a.endpoint) || current.generation != generation {
            let r = a.call(Op::Input {
                session: id.clone(),
                generation,
                revision: current.screen.revision,
                mode_revision: current.screen.mode_revision,
                policy: InputPolicy::Observed,
                intent: Intent::Text {
                    text: "STALE_SCRIPT_MUST_NOT_APPEAR".into(),
                },
            })?;
            let refused = !r.ok && r.receipt.as_ref().is_some_and(|r| r.delivered_bytes == 0);
            if !refused {
                return Err("stale-generation scripted input was not refused".into());
            }
            println!(
                "{}",
                serde_json::json!({"event":"human_takeover_observed","session":id,"stale_input_refused":true,"limit":"takeover fences mediated input, not process side effects"})
            );
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(75));
    }
    println!(
        "{}",
        serde_json::json!({"event":"scripted_clients_detached","limit":"No takeover observed in30s; shell/processes may continue, input owner remains explicit"})
    );
    Ok(())
}
