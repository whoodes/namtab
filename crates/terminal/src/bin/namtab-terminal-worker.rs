//! Private inherited-stdio worker. No PTY, grant, filesystem or network API.
use project_x_terminal::{Engine, rpc::*};
use std::{
    collections::BTreeMap,
    io::{self, BufRead, Read, Write},
};

fn main() {
    if run().is_err() {
        // Do not log attacker-controlled terminal bytes or malformed JSON.
        eprintln!("terminal worker: invalid/failed private exchange; state unavailable");
        std::process::exit(2);
    }
}

fn run() -> Result<(), ()> {
    let mut sessions = BTreeMap::<String, Engine>::new();
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    let mut last_id = 0;
    loop {
        let mut line = Vec::new();
        let count = reader
            .by_ref()
            .take((MAX_REQUEST_BYTES + 1) as u64)
            .read_until(b'\n', &mut line)
            .map_err(|_| ())?;
        if count == 0 {
            return Ok(());
        }
        if count > MAX_REQUEST_BYTES || line.last() != Some(&b'\n') {
            return Err(());
        }
        let request: Request = serde_json::from_slice(&line).map_err(|_| ())?;
        if request.id <= last_id {
            return Err(());
        }
        last_id = request.id;
        let response = Response {
            id: request.id,
            result: apply(request.op, &mut sessions),
        };
        let bytes = serde_json::to_vec(&response).map_err(|_| ())?;
        if bytes.len() + 1 > MAX_RESPONSE_BYTES {
            return Err(());
        }
        writer
            .write_all(&bytes)
            .and_then(|_| writer.write_all(b"\n"))
            .and_then(|_| writer.flush())
            .map_err(|_| ())?;
    }
}

fn apply(
    operation: Operation,
    sessions: &mut BTreeMap<String, Engine>,
) -> Result<Reply, WorkerError> {
    let session = operation.session();
    if session.is_empty()
        || session.len() > 64
        || !session
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(WorkerError::InvalidSession);
    }
    if let Operation::New {
        session,
        cols,
        rows,
    } = operation
    {
        if sessions.contains_key(&session) {
            return Err(WorkerError::AlreadyExists);
        }
        if sessions.len() >= MAX_SESSIONS {
            return Err(WorkerError::Capacity);
        }
        let engine = Engine::new(cols, rows).map_err(WorkerError::Engine)?;
        let frame = engine.frame();
        sessions.insert(session, engine);
        return Ok(Reply::Frame(frame));
    }
    if let Operation::Close { session } = operation {
        sessions
            .remove(&session)
            .ok_or(WorkerError::UnknownSession)?;
        return Ok(Reply::Closed);
    }
    let engine = sessions
        .get_mut(session)
        .ok_or(WorkerError::UnknownSession)?;
    match operation {
        Operation::Feed { bytes, .. } => {
            let result = engine.feed(&bytes).map_err(WorkerError::Engine)?;
            Ok(Reply::Fed {
                frame: engine.frame(),
                replies: result.replies,
            })
        }
        Operation::Frame { .. } => Ok(Reply::Frame(engine.frame())),
        Operation::Encode { input, .. } => {
            let bytes = engine.encode(&input).map_err(WorkerError::Input)?;
            let (revision, mode_revision) = engine.revisions();
            Ok(Reply::Encoded {
                bytes,
                revision,
                mode_revision,
            })
        }
        Operation::Resize { cols, rows, .. } => {
            engine.resize(cols, rows).map_err(WorkerError::Engine)?;
            Ok(Reply::Frame(engine.frame()))
        }
        Operation::FinishOutput { .. } => {
            engine.finish_output().map_err(WorkerError::Engine)?;
            Ok(Reply::Frame(engine.frame()))
        }
        Operation::New { .. } | Operation::Close { .. } => unreachable!(),
    }
}
