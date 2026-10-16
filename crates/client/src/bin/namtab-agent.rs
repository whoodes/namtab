//! A real protocol client, not a model integration or shell-output interpreter.
use project_x_client::{
    mk1::Client,
    mk1_protocol::{Op, Role},
};
use std::io::{self, BufRead, Read};
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
                argv: args[5..].to_vec(),
                task_revision: None,
            })?)?
        );
        return Ok(());
    }
    if args.len() != 3 {
        return Err("unknown command".into());
    }
    let mut input = io::stdin().lock();
    loop {
        let Some(line) = operation_line(&mut input)? else {
            break;
        };
        let op: Op = serde_json::from_slice(&line)?;
        println!("{}", serde_json::to_string(&c.call(op)?)?);
    }
    Ok(())
}

fn operation_line(input: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    let n = input.take(65_537).read_until(b'\n', &mut line)?;
    if n == 0 {
        return Ok(None);
    }
    if n > 65_536 || line.last() != Some(&b'\n') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "operation must be a complete bounded JSON line",
        ));
    }
    Ok(Some(line))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stdin_requires_complete_bounded_lines_without_consuming_a_suffix() {
        let mut input = io::Cursor::new(b"{\"op\":\"observe\"}\r\n{\"op\":\"observe\"}\n");
        assert_eq!(
            operation_line(&mut input).unwrap().unwrap(),
            b"{\"op\":\"observe\"}\r\n"
        );
        assert!(operation_line(&mut input).unwrap().is_some());
        assert!(operation_line(&mut input).unwrap().is_none());
        assert!(operation_line(&mut io::Cursor::new(b"{}")).is_err());
        let data = vec![b'x'; 65_538];
        let mut too_long = io::Cursor::new(data);
        assert!(operation_line(&mut too_long).is_err());
        assert_eq!(too_long.position(), 65_537);
    }
}
