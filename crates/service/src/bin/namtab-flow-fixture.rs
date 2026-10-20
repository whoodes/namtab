//! Synthetic child for the flow experiment, not a user command runner.
use std::io::{self, Write};
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [mode] if ["producer", "noisy", "nonzero", "overflow"].contains(&mode.as_str()) => {
            io::stdout().write_all(b"namtab-artifact-v1")?;
            if mode == "nonzero" {
                std::process::exit(3);
            }
            if mode == "overflow" {
                io::stderr().write_all(&[b'X'; 8192])?;
                return Ok(());
            }
            if mode == "producer" {
                io::stderr().write_all(b"producer-diagnostic")?;
            } else {
                for _ in 0..32 {
                    io::stderr().write_all(&[b'N'; 64])?;
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
            }
            Ok(())
        }
        _ => Err(io::Error::other(
            "expected producer, noisy, nonzero or overflow fixture mode",
        )),
    }
}
