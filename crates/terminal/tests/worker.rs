use project_x_terminal::{Input, Key, KeyPhase, Modifiers, rpc::*};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, Command, Stdio},
};

struct Worker {
    child: Child,
    input: std::process::ChildStdin,
    output: BufReader<std::process::ChildStdout>,
}

impl Worker {
    fn new() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_namtab-terminal-worker"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        Self {
            input: child.stdin.take().unwrap(),
            output: BufReader::new(child.stdout.take().unwrap()),
            child,
        }
    }
    fn call(&mut self, id: u64, op: Operation) -> Response {
        self.input
            .write_all(&serde_json::to_vec(&Request { id, op }).unwrap())
            .unwrap();
        self.input.write_all(b"\n").unwrap();
        self.input.flush().unwrap();
        let mut line = String::new();
        self.output.read_line(&mut line).unwrap();
        assert!(line.len() <= MAX_RESPONSE_BYTES);
        let response: Response = serde_json::from_str(&line).unwrap();
        assert_eq!(response.id, id);
        response
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn real_worker_exchange_keeps_two_session_states_and_single_reply_authority() {
    let mut worker = Worker::new();
    for (id, name) in [(1, "a"), (2, "b")] {
        assert!(matches!(
            worker
                .call(
                    id,
                    Operation::New {
                        session: name.into(),
                        cols: 80,
                        rows: 24
                    }
                )
                .result,
            Ok(Reply::Frame(_))
        ));
    }
    assert_eq!(
        worker
            .call(
                3,
                Operation::New {
                    session: "c".into(),
                    cols: 80,
                    rows: 24
                }
            )
            .result,
        Err(WorkerError::Capacity)
    );
    let fed = worker.call(
        4,
        Operation::Feed {
            session: "a".into(),
            bytes: b"live\x1b[?1h\x1b[6n".to_vec(),
        },
    );
    match fed.result.unwrap() {
        Reply::Fed { frame, replies } => {
            assert_eq!(replies, b"\x1b[1;5R");
            assert!(frame.modes.application_cursor);
            assert_eq!(frame.cells[0].text, "l");
        }
        other => panic!("unexpected {other:?}"),
    }
    let encoded = worker.call(
        5,
        Operation::Encode {
            session: "a".into(),
            input: Input::Key {
                key: Key::Up,
                modifiers: Modifiers::default(),
                phase: KeyPhase::Press,
            },
        },
    );
    assert!(matches!(encoded.result,Ok(Reply::Encoded{bytes,..}) if bytes==b"\x1bOA"));
    let other = worker.call(
        6,
        Operation::Frame {
            session: "b".into(),
        },
    );
    assert!(
        matches!(other.result,Ok(Reply::Frame(frame)) if !frame.modes.application_cursor && frame.cells[0].text==" ")
    );
    assert_eq!(
        worker
            .call(
                7,
                Operation::Feed {
                    session: "a".into(),
                    bytes: vec![b'x'; 513]
                }
            )
            .result,
        Err(WorkerError::Engine(project_x_terminal::Error::FeedTooLarge))
    );
    assert_eq!(
        worker
            .call(
                8,
                Operation::Close {
                    session: "a".into()
                }
            )
            .result,
        Ok(Reply::Closed)
    );
    assert_eq!(
        worker
            .call(
                9,
                Operation::Frame {
                    session: "a".into()
                }
            )
            .result,
        Err(WorkerError::UnknownSession)
    );
}

#[test]
fn malformed_or_oversize_private_requests_fail_closed_without_echoing_payload() {
    for bytes in [
        b"{bad private payload}\n".to_vec(),
        vec![b'a'; MAX_REQUEST_BYTES + 1],
    ] {
        let mut worker = Worker::new();
        worker.input.write_all(&bytes).unwrap();
        worker.input.flush().unwrap();
        let status = worker.child.wait().unwrap();
        assert_eq!(status.code(), Some(2));
        let mut line = String::new();
        assert_eq!(worker.output.read_line(&mut line).unwrap(), 0);
    }
}

#[test]
fn maximum_frame_fits_response_budget_and_invalid_names_are_rejected() {
    let mut worker = Worker::new();
    assert_eq!(
        worker
            .call(
                1,
                Operation::New {
                    session: "bad/name".into(),
                    cols: 80,
                    rows: 24
                }
            )
            .result,
        Err(WorkerError::InvalidSession)
    );
    let reply = worker.call(
        2,
        Operation::New {
            session: "max".into(),
            cols: 160,
            rows: 48,
        },
    );
    assert!(matches!(reply.result,Ok(Reply::Frame(frame)) if frame.cells.len()==7680));
    assert!(
        matches!(worker.call(3,Operation::Resize{session:"max".into(),cols:2,rows:2}).result,Ok(Reply::Frame(frame)) if frame.cells.len()==4)
    );
}

#[test]
fn repeated_private_request_id_is_never_reexecuted() {
    let mut worker = Worker::new();
    worker.call(
        1,
        Operation::New {
            session: "a".into(),
            cols: 80,
            rows: 24,
        },
    );
    worker
        .input
        .write_all(
            &serde_json::to_vec(&Request {
                id: 1,
                op: Operation::Feed {
                    session: "a".into(),
                    bytes: b"non-idempotent".to_vec(),
                },
            })
            .unwrap(),
        )
        .unwrap();
    worker.input.write_all(b"\n").unwrap();
    worker.input.flush().unwrap();
    assert_eq!(worker.child.wait().unwrap().code(), Some(2));
    let mut line = String::new();
    assert_eq!(worker.output.read_line(&mut line).unwrap(), 0);
}

#[test]
fn real_worker_finishes_partial_output_without_claiming_process_exit() {
    let mut worker = Worker::new();
    worker
        .call(
            1,
            Operation::New {
                session: "eof".into(),
                cols: 80,
                rows: 24,
            },
        )
        .result
        .unwrap();
    worker
        .call(
            2,
            Operation::Feed {
                session: "eof".into(),
                bytes: vec![0xc3],
            },
        )
        .result
        .unwrap();
    match worker
        .call(
            3,
            Operation::FinishOutput {
                session: "eof".into(),
            },
        )
        .result
        .unwrap()
    {
        Reply::Frame(frame) => {
            assert!(frame.closed);
            assert_eq!(frame.diagnostics.invalid_utf8, 1);
        }
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(
        worker
            .call(
                4,
                Operation::Encode {
                    session: "eof".into(),
                    input: Input::Text { text: "x".into() }
                }
            )
            .result,
        Err(WorkerError::Input(project_x_terminal::InputError::Closed))
    );
    assert!(matches!(
        worker
            .call(
                5,
                Operation::FinishOutput {
                    session: "eof".into()
                }
            )
            .result,
        Ok(Reply::Frame(_))
    ));
}
