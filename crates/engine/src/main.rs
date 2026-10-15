//! Bounded EA1 pipe worker, not an application backend or security sandbox.
#![deny(unsafe_code)]
#[allow(unsafe_code)]
mod ffi;
use project_x_protocol::internal_engine::*;
use std::io::{self, Read, Write};

#[derive(Default)]
struct Outcome {
    frame: Option<Frame>,
    input: Option<Vec<u8>>,
    reply: Option<Reply>,
}
#[derive(Default)]
struct Worker {
    terminal: Option<ffi::Terminal>,
    stamp: Option<Stamp>,
    last_seq: u32,
    unknown: bool,
}
fn geometry(cols: u16, rows: u16) -> bool {
    cols > 0
        && rows > 0
        && cols <= 240
        && rows <= 100
        && usize::from(cols) * usize::from(rows) <= MAX_CELLS
}
fn identity(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_:".contains(&b))
}
impl Worker {
    fn response(&self, seq: u32) -> Response {
        Response {
            version: VERSION,
            seq,
            profile: PROFILE.into(),
            stamp: self.stamp.clone(),
            status: if self.unknown { "unknown" } else { "ok" }.into(),
            error: None,
            frame: None,
            input_bytes: None,
            reply: None,
        }
    }
    fn apply(&mut self, request: Request) -> Response {
        let seq = request.seq;
        let result = self.execute(request);
        let mut out = self.response(seq);
        match result {
            Ok(result) => {
                out.frame = result.frame;
                out.input_bytes = result.input;
                out.reply = result.reply;
            }
            Err(error) => {
                out.error = Some(error);
                out.status = if self.unknown { "unknown" } else { "rejected" }.into();
            }
        }
        out
    }
    fn execute(&mut self, r: Request) -> Result<Outcome, Error> {
        if r.version != VERSION {
            return Err(Error::Version);
        }
        if self.last_seq.checked_add(1) != Some(r.seq) {
            return Err(Error::Sequence);
        }
        // Consume even rejected logical requests. Never replay a non-idempotent work sequence.
        self.last_seq = r.seq;
        if let Operation::Init {
            session,
            incarnation,
            cols,
            rows,
        } = r.operation
        {
            if self.terminal.is_some() || self.stamp.is_some() || r.expected.is_some() {
                return Err(Error::Lifecycle);
            }
            if !identity(&session) || !identity(&incarnation) {
                return Err(Error::Identity);
            }
            if !geometry(cols, rows) {
                return Err(Error::Limit);
            }
            let terminal = ffi::Terminal::new(cols, rows)?;
            self.stamp = Some(Stamp {
                session,
                incarnation,
                revision: 0,
                geometry: 0,
            });
            self.terminal = Some(terminal);
            return Ok(Outcome::default());
        }
        if self.unknown {
            return Err(Error::Unknown);
        }
        let stamp = self.stamp.clone().ok_or(Error::Lifecycle)?;
        if r.expected.as_ref() != Some(&stamp) {
            return Err(Error::Stale);
        }
        if matches!(r.operation, Operation::Shutdown {}) {
            self.terminal.take();
            return Ok(Outcome::default());
        }
        let next_revision = if matches!(
            r.operation,
            Operation::Feed { .. } | Operation::Resize { .. }
        ) {
            stamp.revision.checked_add(1).ok_or(Error::Limit)?
        } else {
            stamp.revision
        };
        let next_geometry = if matches!(r.operation, Operation::Resize { .. }) {
            stamp.geometry.checked_add(1).ok_or(Error::Limit)?
        } else {
            stamp.geometry
        };
        let terminal = self.terminal.as_mut().ok_or(Error::Lifecycle)?;
        let result = (|| {
            let mut frame = None;
            let mut input = None;
            match r.operation {
                Operation::Observe {} => {
                    frame = Some(terminal.frame()?);
                }
                Operation::Feed { bytes } => {
                    if bytes.len() > MAX_BYTES {
                        return Err(Error::Limit);
                    }
                    terminal.feed(&bytes)?;
                    self.stamp.as_mut().unwrap().revision = next_revision;
                }
                Operation::Resize { cols, rows } => {
                    if !geometry(cols, rows) {
                        return Err(Error::Limit);
                    }
                    terminal.resize(cols, rows)?;
                    self.stamp.as_mut().unwrap().revision = next_revision;
                    let s = self.stamp.as_mut().unwrap();
                    s.geometry = next_geometry;
                }
                Operation::Encode { input: intent } => {
                    input = Some(terminal.encode(intent)?);
                }
                Operation::Init { .. } | Operation::Shutdown {} => unreachable!(),
            }
            let replies = terminal.replies()?;
            let reply = (!replies.is_empty()).then_some(Reply {
                work: r.seq,
                origin: "terminal_reply".into(),
                bytes: replies,
            });
            Ok(Outcome {
                frame,
                input,
                reply,
            })
        })();
        if matches!(result, Err(Error::Native)) {
            // No partial frame/reply escapes. Input remains disabled until this worker is replaced.
            self.unknown = true;
            self.terminal.take();
        }
        result
    }
}
fn read_frame(input: &mut impl Read) -> io::Result<Option<Vec<u8>>> {
    let mut len = [0; 4];
    match input.read(&mut len[..1])? {
        0 => return Ok(None),
        1 => {}
        _ => unreachable!(),
    }
    input.read_exact(&mut len[1..])?;
    let size = u32::from_be_bytes(len) as usize;
    if size == 0 || size > MAX_REQUEST {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "frame limit"));
    }
    let mut bytes = vec![0; size];
    input.read_exact(&mut bytes)?;
    Ok(Some(bytes))
}
fn run() -> io::Result<()> {
    let (mut input, mut output) = (io::stdin().lock(), io::stdout().lock());
    let mut worker = Worker::default();
    while let Some(bytes) = read_frame(&mut input)? {
        let (response, stop) = match serde_json::from_slice::<Request>(&bytes) {
            Ok(request) => {
                let stop = matches!(request.operation, Operation::Shutdown {});
                let response = worker.apply(request);
                let stop = stop && response.status == "ok";
                (response, stop)
            }
            // Lose framing/request trust: emit one metadata-only failure and close. No raw echo.
            Err(_) => {
                let mut r = worker.response(0);
                r.status = "rejected".into();
                r.error = Some(Error::Malformed);
                (r, true)
            }
        };
        let bytes = serde_json::to_vec(&response)?;
        if bytes.len() > MAX_RESPONSE {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "response limit"));
        }
        output.write_all(&(bytes.len() as u32).to_be_bytes())?;
        output.write_all(&bytes)?;
        output.flush()?;
        if stop {
            break;
        }
    }
    Ok(())
}
fn main() {
    if run().is_err() {
        eprintln!("EA1 transport failed; delivery/state may be unknown");
        std::process::exit(2);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transport_rejects_partial_or_oversized_before_allocation() {
        assert!(read_frame(&mut &[0, 0, 1][..]).is_err());
        assert!(read_frame(&mut &u32::MAX.to_be_bytes()[..]).is_err());
        assert!(read_frame(&mut &[0, 0, 0, 0][..]).is_err());
        assert!(read_frame(&mut &[0, 0, 0, 2, 1][..]).is_err());
        assert!(read_frame(&mut &[][..]).unwrap().is_none());
    }
    #[test]
    fn exhausted_counters_refuse_before_feed_or_resize() {
        let mut w = Worker::default();
        let init = Request {
            version: VERSION,
            seq: 1,
            expected: None,
            operation: Operation::Init {
                session: "test".into(),
                incarnation: "first".into(),
                cols: 12,
                rows: 5,
            },
        };
        assert!(w.apply(init).error.is_none());
        w.stamp.as_mut().unwrap().revision = u32::MAX;
        let r = Request {
            version: VERSION,
            seq: 2,
            expected: w.stamp.clone(),
            operation: Operation::Feed {
                bytes: b"X".to_vec(),
            },
        };
        assert!(matches!(w.apply(r).error, Some(Error::Limit)));
        assert_eq!(
            w.terminal.as_mut().unwrap().frame().unwrap().cells[0].text,
            ""
        );
        w.stamp.as_mut().unwrap().revision = 0;
        w.stamp.as_mut().unwrap().geometry = u32::MAX;
        let r = Request {
            version: VERSION,
            seq: 3,
            expected: w.stamp.clone(),
            operation: Operation::Resize { cols: 9, rows: 4 },
        };
        assert!(matches!(w.apply(r).error, Some(Error::Limit)));
        assert_eq!(w.terminal.as_mut().unwrap().frame().unwrap().cols, 12);
    }
    #[test]
    fn geometry_caps_do_not_overflow() {
        assert!(!geometry(u16::MAX, u16::MAX));
        assert!(!geometry(0, 5));
        assert!(geometry(80, 24));
    }
}
