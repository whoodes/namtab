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
            let gap = json!({"event":"trace_gap","lost":lost});
            let ok = (lost == 0 || append(&mut file, &mut bytes, &mut count, &gap, true))
                && append(&mut file, &mut bytes, &mut count, &value, true);
            if !ok {
                // Every ordinary/gap append reserves room for this final marker.
                append(
                    &mut file,
                    &mut bytes,
                    &mut count,
                    &json!({"event":"trace_truncated"}),
                    false,
                );
                break;
            }
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

// Reserve one small marker as part of the total record/byte bounds, including
// gaps. Write errors invalidate evidence; interaction paths never block here.
fn append(
    file: &mut impl Write,
    bytes: &mut usize,
    count: &mut usize,
    value: &Value,
    reserve: bool,
) -> bool {
    let Ok(mut line) = serde_json::to_vec(value) else {
        return false;
    };
    line.push(b'\n');
    let (byte_cap, record_cap) = if reserve {
        (1_048_576 - 128, 4095)
    } else {
        (1_048_576, 4096)
    };
    if line.len() > 4096 || *bytes + line.len() > byte_cap || *count >= record_cap {
        return false;
    }
    if file.write_all(&line).is_err() {
        return false;
    }
    *bytes += line.len();
    *count += 1;
    true
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normal_and_gap_records_reserve_a_counted_truncation_marker() {
        for value in [
            json!({"event":"trace_gap","lost":9}),
            json!({"event":"normal","metadata":"x".repeat(3000)}),
        ] {
            let mut file = Vec::new();
            let (mut bytes, mut count) = (0, 0);
            while append(&mut file, &mut bytes, &mut count, &value, true) {}
            assert!(append(
                &mut file,
                &mut bytes,
                &mut count,
                &json!({"event":"trace_truncated"}),
                false
            ));
            assert!(file.len() <= 1_048_576);
            assert!(count <= 4096);
            assert_eq!(file.len(), bytes);
            assert_eq!(file.iter().filter(|b| **b == b'\n').count(), count);
        }
    }
}
