//! A single bounded actor owns the private worker exchange. No core is constructed here.
use project_x_terminal::rpc::{self, Operation, Reply};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

type Job = (rpc::Request, mpsc::SyncSender<Result<Reply, String>>);
pub struct Worker {
    tx: mpsc::SyncSender<Job>,
    process: Arc<Mutex<Child>>,
    failed: AtomicBool,
    sequence: AtomicU64,
    enqueue: Mutex<()>,
    outstanding: AtomicUsize,
}

pub fn line(reader: &mut impl BufRead, limit: usize) -> io::Result<Option<Vec<u8>>> {
    let mut bytes = Vec::new();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return if bytes.is_empty() {
                Ok(None)
            } else {
                Err(io::Error::other("truncated frame"))
            };
        }
        let end = available.iter().position(|&b| b == b'\n');
        let n = end.map_or(available.len(), |n| n + 1);
        if bytes
            .len()
            .checked_add(n)
            .is_none_or(|size| size > limit + 1)
        {
            return Err(io::Error::other("frame capacity"));
        }
        bytes.extend_from_slice(&available[..n]);
        reader.consume(n);
        if end.is_some() {
            bytes.pop();
            return Ok(Some(bytes));
        }
    }
}

impl Worker {
    pub fn start(path: &Path, workspace: &Path) -> io::Result<Self> {
        let mut process = Command::new(path)
            .current_dir(workspace)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let mut input = process
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("worker stdin"))?;
        let output = process
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("worker stdout"))?;
        let process = Arc::new(Mutex::new(process));
        let (response_tx, response_rx) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let mut reader = BufReader::new(output);
            loop {
                let parsed = line(&mut reader, rpc::MAX_RESPONSE_BYTES)
                    .and_then(|body| body.ok_or_else(|| io::Error::other("worker EOF")))
                    .and_then(|body| {
                        serde_json::from_slice::<rpc::Response>(&body).map_err(io::Error::other)
                    });
                let stop = parsed.is_err();
                if response_tx.send(parsed).is_err() || stop {
                    break;
                }
            }
        });
        let (tx, rx) = mpsc::sync_channel::<Job>(8);
        std::thread::spawn(move || {
            while let Ok((request, answer)) = rx.recv() {
                let result = (|| {
                    let body = serde_json::to_vec(&request).map_err(|_| "worker_encode")?;
                    if body.len() > rpc::MAX_REQUEST_BYTES {
                        return Err("worker_request_capacity");
                    }
                    input
                        .write_all(&body)
                        .and_then(|()| input.write_all(b"\n"))
                        .and_then(|()| input.flush())
                        .map_err(|_| "worker_write")?;
                    let response = response_rx
                        .recv()
                        .map_err(|_| "worker_channel")?
                        .map_err(|_| "worker_frame")?;
                    if response.id != request.id {
                        return Err("worker_correlation");
                    }
                    if response
                        .result
                        .as_ref()
                        .is_ok_and(|reply| !valid_reply(reply))
                    {
                        return Err("worker_projection_bounds");
                    }
                    response.result.map_err(|_| "worker_refused")
                })()
                .map_err(str::to_owned);
                let _ = answer.send(result);
            }
        });
        Ok(Self {
            tx,
            process,
            failed: AtomicBool::new(false),
            sequence: AtomicU64::new(1),
            enqueue: Mutex::new(()),
            outstanding: AtomicUsize::new(0),
        })
    }
    pub fn failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }
    pub fn stop(&self) {
        self.failed.store(true, Ordering::Release);
        if let Ok(mut child) = self.process.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    pub fn call(&self, op: Operation) -> Result<Reply, String> {
        if self.failed() {
            return Err("worker_unavailable".into());
        }
        let enqueue = self.enqueue.lock().unwrap();
        let id = self.sequence.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::sync_channel(1);
        if self.tx.try_send((rpc::Request { id, op }, tx)).is_err() {
            return Err("worker_queue_full".into());
        }
        self.outstanding.fetch_add(1, Ordering::Release);
        drop(enqueue);
        let result = match rx.recv_timeout(Duration::from_secs(2)) {
            Ok(Ok(reply)) => Ok(reply),
            Ok(Err(error)) if error == "worker_refused" => Err(error),
            _ => {
                self.stop();
                Err("worker_unavailable".into())
            }
        };
        self.outstanding.fetch_sub(1, Ordering::Release);
        result
    }
    #[cfg(test)]
    pub fn process_id(&self) -> u32 {
        self.process.lock().unwrap().id()
    }
    #[cfg(test)]
    pub fn outstanding(&self) -> usize {
        self.outstanding.load(Ordering::Acquire)
    }
}
fn valid_reply(reply: &Reply) -> bool {
    let frame = match reply {
        Reply::Frame(frame) | Reply::Fed { frame, .. } => Some(frame),
        _ => None,
    };
    if let Some(frame) = frame {
        if !(2..=160).contains(&frame.cols)
            || !(2..=48).contains(&frame.rows)
            || frame.cells.len() != usize::from(frame.cols) * usize::from(frame.rows)
            || frame.cursor_col >= frame.cols
            || frame.cursor_row >= frame.rows
            || frame.revision == 0
            || frame.mode_revision == 0
        {
            return false;
        }
        if frame.cells.iter().any(|cell| {
            let mut chars = cell.text.chars();
            let c = chars.next();
            chars.next().is_some()
                || !c.is_some_and(|c| {
                    c == '\u{fffd}'
                        || (' '..='~').contains(&c)
                        || ('\u{a0}'..='\u{ff}').contains(&c) && c != '\u{ad}'
                })
        }) {
            return false;
        }
    }
    !matches!(reply, Reply::Fed { replies, .. } if replies.len() > 4096)
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_lines_reject_truncation_and_one_beyond() {
        assert_eq!(line(&mut &b"abc\n"[..], 3).unwrap(), Some(b"abc".to_vec()));
        assert!(line(&mut &b"abcd\n"[..], 3).is_err());
        assert!(line(&mut &b"abc"[..], 3).is_err());
    }
}
