//! Explicit public human lifecycle client. Never accepts a PID or discovers sessions.
use project_x_client::{
    mk1::Client,
    mk1_protocol::{self as p, Op},
};
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if !(args.len() == 4 || args.len() == 6)
        || args[1] != "--socket"
        || !matches!(args[3].as_str(), "info" | "stop")
    {
        return Err("usage: namtabctl --socket PATH info | stop --epoch EXPECTED".into());
    }
    let stop = args[3] == "stop";
    if (stop && args.len() != 6)
        || (args.len() == 6 && (args[4] != "--epoch" || args[5].is_empty()))
    {
        return Err("stop requires exact --epoch; info optionally accepts it".into());
    }
    let mut c = Client::connect_lifecycle(Path::new(&args[2]))?;
    let s = c.observe_checked()?;
    if args.len() == 6 && c.epoch != args[5] {
        return Err("wrong broker epoch; no shutdown sent".into());
    }
    if !stop {
        println!(
            "{}",
            serde_json::json!({"version":s.version,"epoch":s.epoch,"capabilities":s.capabilities,"sessions":s.sessions.len(),"attempts":s.attempts.len(),"degraded":s.degraded})
        );
        return Ok(());
    }
    if !s.capabilities.iter().any(|x| x == p::HUMAN_SHUTDOWN) {
        return Err("broker lacks human_shutdown_v1; no PID/signal fallback".into());
    }
    let id = c.next_id();
    let mut response = c.call(Op::Shutdown {})?;
    if response
        .receipt
        .as_ref()
        .is_some_and(|r| r.endpoint != c.endpoint || r.id != id || r.operation != "shutdown")
    {
        return Err("shutdown receipt binding mismatch; completion unknown".into());
    }
    response.resume = None;
    println!("{}", serde_json::to_string(&response)?);
    if !response.ok
        || response
            .receipt
            .as_ref()
            .is_none_or(|r| r.state != "applied")
    {
        if response.receipt.as_ref().is_some_and(|r| {
            r.operation == "shutdown"
                && r.state == "unknown"
                && r.detail == "protective_shutdown_requested_receipt_not_durable"
        }) {
            std::process::exit(3);
        }
        return Err("shutdown refused/uncertain; do not retry blindly or signal a PID".into());
    }
    Ok(())
}
