//! A real protocol client, not a model integration or shell-output interpreter.
use project_x_client::{
    mk1::Client,
    mk1_protocol::{Op, Role},
};
use std::io::{self, BufRead};
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 || args[1] != "--socket" {
        return Err("usage: namtab-agent --socket PATH [observe | run TASK PROGRAM ARG...] (otherwise read operation JSON lines)".into());
    }
    let mut c = Client::connect(Path::new(&args[2]), "agent-cli", Role::Agent)?;
    if args.get(3).map(String::as_str) == Some("observe") {
        println!("{}", serde_json::to_string(&c.call(Op::Observe {})?)?);
        return Ok(());
    }
    if args.get(3).map(String::as_str) == Some("run") {
        if args.len() < 6 {
            return Err("run needs TASK PROGRAM [ARG...]".into());
        }
        println!(
            "{}",
            serde_json::to_string(&c.call(Op::Run {
                task: args[4].clone(),
                argv: args[5..].to_vec()
            })?)?
        );
        return Ok(());
    }
    if args.len() != 3 {
        return Err("unknown command".into());
    }
    for line in io::stdin().lock().lines() {
        let line = line?;
        if line.len() > 65_536 {
            return Err("operation too large".into());
        }
        let op: Op = serde_json::from_str(&line)?;
        println!("{}", serde_json::to_string(&c.call(op)?)?);
    }
    Ok(())
}
