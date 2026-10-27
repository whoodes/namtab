//! Optional, bounded synthetic-test metadata. Never records keys, intent text,
//! argv, output or resume tokens. No filesystem work occurs on interaction paths.
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::{self, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, SyncSender},
    },
    time::Instant,
};
struct Trace {
    sender: SyncSender<Value>,
    started: Instant,
}
static TRACE: OnceLock<Trace> = OnceLock::new();
static NEXT: AtomicU64 = AtomicU64::new(1);
static LOST: AtomicU64 = AtomicU64::new(0);
pub fn init(path: &Path) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)?;
    let (tx, rx) = mpsc::sync_channel::<Value>(128);
    std::thread::spawn(move || {
        let (mut bytes, mut count) = (0, 0);
        for value in rx {
            let lost = LOST.swap(0, Ordering::SeqCst);
            if lost > 0 {
                let _ = writeln!(file, "{}", json!({"event":"trace_gap","lost":lost}));
            }
            let Ok(mut line) = serde_json::to_vec(&value) else {
                break;
            };
            line.push(b'\n');
            if line.len() > 4096 || bytes + line.len() > 1_048_576 || count >= 4096 {
                let _ = writeln!(file, "{}", json!({"event":"trace_truncated"}));
                break;
            }
            if file.write_all(&line).is_err() {
                break;
            }
            bytes += line.len();
            count += 1;
        }
    });
    TRACE
        .set(Trace {
            sender: tx,
            started: Instant::now(),
        })
        .map_err(|_| io::Error::other("trace already initialized"))
}
pub fn emit(data: Value) -> u64 {
    let Some(trace) = TRACE.get() else {
        return 0;
    };
    let sequence = NEXT.fetch_add(1, Ordering::SeqCst);
    let value =
        json!({"sequence":sequence,"elapsed_us":trace.started.elapsed().as_micros(),"data":data});
    if trace.sender.try_send(value).is_err() {
        LOST.fetch_add(1, Ordering::SeqCst);
    }
    sequence
}
