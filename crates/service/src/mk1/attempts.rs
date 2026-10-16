//! Managed argv attempts retain bounded stream projections while always draining.
use super::*;
impl Broker {
    pub(super) fn run(
        self: &Arc<Self>,
        endpoint: &str,
        request_id: u64,
        task: &str,
        task_revision: Option<&str>,
        argv: &[String],
    ) -> Result<(String, usize, String), String> {
        let id = {
            let mut state = self.state.lock().unwrap();
            match (state.tasks.get(task), task_revision) {
                (Some(contract), Some(revision)) if contract.revision == revision => {}
                (Some(_), _) => return Err("task_revision_required_or_stale".into()),
                (None, Some(_)) => return Err("unknown_task_contract".into()),
                (None, None) => {}
            }
            if state.attempts.len() + state.reserved_attempts >= 2 {
                return Err("attempt_capacity_two_total".into());
            }
            state.reserved_attempts += 1;
            state.sequence += 1;
            format!("attempt{}", state.sequence)
        };
        let spec = self.spec(
            argv[0].clone().into(),
            argv[1..].iter().map(OsString::from).collect(),
        );
        let process = platform::launch_managed(&spec).map_err(|_| "managed_exec_failed");
        let mut state = self.state.lock().unwrap();
        state.reserved_attempts -= 1;
        let process = process?;
        state.attempts.insert(
            id.clone(),
            LiveAttempt {
                view: Attempt {
                    id: id.clone(),
                    task: task.into(),
                    origin: Some(wire::AttemptOrigin {
                        endpoint: endpoint.into(),
                        request_id,
                    }),
                    task_revision: task_revision.map(str::to_owned),
                    state: "running".into(),
                    exit_code: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    truncated: false,
                },
                stdout: Vec::new(),
                stderr: Vec::new(),
                stdout_eof: false,
                stderr_eof: false,
                exited: false,
            },
        );
        // Correlate at authoritative publication, before another endpoint can
        // observe this attempt. Receipt state still describes completion certainty.
        state
            .endpoints
            .get_mut(endpoint)
            .unwrap()
            .decisions
            .get_mut(&request_id)
            .unwrap()
            .receipt
            .result = Some(MutationResult::AttemptStarted {
            attempt: id.clone(),
        });
        drop(state);
        for (is_stderr, mut stream) in [
            (false, Box::new(process.stdout) as Box<dyn Read + Send>),
            (true, Box::new(process.stderr) as Box<dyn Read + Send>),
        ] {
            let broker = Arc::clone(self);
            let id = id.clone();
            std::thread::spawn(move || {
                let mut bytes = [0_u8; 4096];
                loop {
                    match stream.read(&mut bytes) {
                        Ok(0) => break,
                        Ok(n) => {
                            let mut state = broker.state.lock().unwrap();
                            let value = state.attempts.get_mut(&id).unwrap();
                            let target = if is_stderr {
                                &mut value.stderr
                            } else {
                                &mut value.stdout
                            };
                            let take = n.min(65_536 - target.len());
                            target.extend_from_slice(&bytes[..take]);
                            value.view.truncated |= take < n;
                        }
                        Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                        Err(_) => {
                            broker
                                .state
                                .lock()
                                .unwrap()
                                .attempts
                                .get_mut(&id)
                                .unwrap()
                                .view
                                .state = "unknown".into();
                            break;
                        }
                    }
                }
                let mut state = broker.state.lock().unwrap();
                let value = state.attempts.get_mut(&id).unwrap();
                if is_stderr {
                    value.stderr_eof = true;
                } else {
                    value.stdout_eof = true;
                }
                Self::attempt_state(value);
            });
        }
        let broker = Arc::clone(self);
        std::thread::spawn(move || {
            let mut process = process.process;
            loop {
                if broker.stopping.load(Ordering::Acquire) {
                    let _ = process.kill_and_wait();
                }
                match process.try_wait() {
                    Ok(Some(status)) => {
                        let mut state = broker.state.lock().unwrap();
                        let value = state.attempts.get_mut(&id).unwrap();
                        value.exited = true;
                        value.view.exit_code = status.code();
                        Self::attempt_state(value);
                        break;
                    }
                    Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                    Err(_) => {
                        broker
                            .state
                            .lock()
                            .unwrap()
                            .attempts
                            .get_mut(&id)
                            .unwrap()
                            .view
                            .state = "unknown".into();
                        break;
                    }
                }
            }
        });
        Ok((
            "applied".into(),
            0,
            "managed_argv_started_separate_streams_not_completion".into(),
        ))
    }
    pub(super) fn attempt_state(value: &mut LiveAttempt) {
        if value.view.state != "unknown" {
            value.view.state = if !value.exited {
                "running"
            } else if value.stdout_eof && value.stderr_eof {
                "exited"
            } else {
                "draining"
            }
            .into();
        }
    }
}
