//! Fixed managed children; bounded output collection never holds the control lock.
use super::*;
use project_x_platform::local_process::{CommandSpec, ManagedProcess};
use std::{
    io::Read,
    path::Path,
    sync::{
        Arc, Mutex,
        mpsc::{RecvTimeoutError, sync_channel},
    },
    thread,
};
pub(super) fn spec(
    cwd: &Path,
    run: Run,
    artifact: Option<&[u8]>,
    producer_mode: &str,
    fixture: &Path,
) -> std::io::Result<CommandSpec> {
    let mut s = CommandSpec::new(
        if run == Run::Producer {
            fixture.as_os_str()
        } else {
            std::ffi::OsStr::new("/usr/bin/printf")
        },
        cwd,
    );
    s.args = if run == Run::Producer {
        vec![producer_mode.into()]
    } else {
        let bytes = artifact.ok_or_else(|| std::io::Error::other("missing_artifact"))?;
        if bytes.len() > wire::MAX_ARTIFACT || !bytes.iter().all(|b| (32..=126).contains(b)) {
            return Err(std::io::Error::other("artifact_profile"));
        }
        vec![
            "consumed:%s\n".into(),
            std::str::from_utf8(bytes)
                .map_err(std::io::Error::other)?
                .into(),
        ]
    };
    s.env = vec![
        ("PATH".into(), "/usr/bin:/bin".into()),
        ("LC_ALL".into(), "C".into()),
    ];
    Ok(s)
}
pub(super) struct Outcome {
    pub exit_code: Option<i32>,
    pub drained: bool,
    pub stdout: Vec<u8>,
    pub counts: [usize; 2],
}
enum Chunk {
    Data(Stream, Vec<u8>),
    End(Stream, bool),
}
pub(super) fn collect(
    core: &Arc<Mutex<Core>>,
    origin: &Receipt,
    mut child: ManagedProcess,
) -> Outcome {
    let (tx, rx) = sync_channel(8);
    let readers = [
        (
            Stream::Stdout,
            Box::new(child.stdout) as Box<dyn Read + Send>,
        ),
        (
            Stream::Stderr,
            Box::new(child.stderr) as Box<dyn Read + Send>,
        ),
    ];
    let threads: Vec<_> = readers
        .into_iter()
        .map(|(stream, mut reader)| {
            let tx = tx.clone();
            thread::spawn(move || {
                let mut buffer = [0; 256];
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) => {
                            let _ = tx.send(Chunk::End(stream, true));
                            break;
                        }
                        Ok(n) => {
                            if tx.send(Chunk::Data(stream, buffer[..n].to_vec())).is_err() {
                                break;
                            }
                        }
                        Err(_) => {
                            let _ = tx.send(Chunk::End(stream, false));
                            break;
                        }
                    }
                }
            })
        })
        .collect();
    drop(tx);
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut ends = [false; 2];
    let mut counts = [0; 2];
    let mut stdout = Vec::new();
    let mut code = None;
    let mut exited = false;
    let mut good = true;
    while !(ends.iter().all(|v| *v) && exited) {
        if Instant::now() >= deadline {
            good = false;
            break;
        }
        match child.process.try_wait() {
            Ok(Some(status)) => {
                code = status.code();
                exited = true;
            }
            Ok(None) => {}
            Err(_) => {
                good = false;
                break;
            }
        }
        match rx.recv_timeout(Duration::from_millis(10)) {
            Ok(Chunk::Data(stream, bytes)) => {
                let i = if stream == Stream::Stdout { 0 } else { 1 };
                let offset = counts[i];
                if core
                    .lock()
                    .unwrap()
                    .output(&origin.origin, stream, offset, &bytes)
                    .is_err()
                {
                    good = false;
                    break;
                }
                counts[i] += bytes.len();
                if i == 0 {
                    if stdout.len() + bytes.len() > wire::MAX_ARTIFACT {
                        good = false;
                        break;
                    }
                    stdout.extend_from_slice(&bytes);
                }
            }
            Ok(Chunk::End(stream, ok)) => {
                ends[if stream == Stream::Stdout { 0 } else { 1 }] = true;
                if !ok {
                    good = false;
                    break;
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                if !ends.iter().all(|v| *v) {
                    good = false;
                    break;
                }
                thread::sleep(Duration::from_millis(1));
            }
        }
    }
    drop(rx);
    if !good || !exited {
        let _ = child.process.kill_and_wait();
    }
    for thread in threads {
        if thread.join().is_err() {
            good = false;
        }
    }
    Outcome {
        exit_code: code,
        drained: good && ends.iter().all(|v| *v) && exited,
        stdout,
        counts,
    }
}
