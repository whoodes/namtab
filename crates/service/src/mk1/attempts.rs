//! Managed argv attempts retain bounded stream projections while always draining.
use super::*;
use std::os::unix::process::ExitStatusExt;
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
        let life = ChildActive::new(&self.active_children);
        let process = platform::launch_managed(&spec).map_err(|_| "managed_exec_failed");
        let mut state = self.state.lock().unwrap();
        state.reserved_attempts -= 1;
        let process = process?;
        let cancel = Arc::new(AtomicBool::new(false));
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
                    cancellation: None,
                    exit_signal: None,
                },
                stdout: Vec::new(),
                stderr: Vec::new(),
                stdout_eof: false,
                stderr_eof: false,
                exited: false,
                cancel: Arc::clone(&cancel),
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
            let _life = life;
            let mut process = process.process;
            loop {
                let cancelled = cancel.swap(false, Ordering::AcqRel);
                let result = if broker.stopping.load(Ordering::Acquire) || cancelled {
                    process.kill_and_wait().map(Some)
                } else {
                    process.try_wait()
                };
                match result {
                    Ok(Some(status)) => {
                        let mut state = broker.state.lock().unwrap();
                        let value = state.attempts.get_mut(&id).unwrap();
                        value.exited = true;
                        value.view.exit_code = status.code();
                        value.view.exit_signal = status.signal();
                        if let Some(c) = &mut value.view.cancellation {
                            c.status = wire::CancellationState::ExitObserved;
                        }
                        Self::attempt_state(value);
                        break;
                    }
                    Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                    Err(_) => {
                        let mut state = broker.state.lock().unwrap();
                        let value = state.attempts.get_mut(&id).unwrap();
                        value.view.state = "unknown".into();
                        if let Some(c) = &mut value.view.cancellation {
                            c.status = wire::CancellationState::Unknown;
                        }
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
    pub(super) fn cancel_attempt(
        &self,
        endpoint: &str,
        request_id: u64,
        epoch: &str,
        id: &str,
        origin_endpoint: &str,
        origin_request_id: u64,
    ) -> Result<(String, usize, String), String> {
        if epoch != self.epoch {
            return Err("cancel_epoch_stale".into());
        }
        let mut state = self.state.lock().unwrap();
        if state.degraded.is_some() || self.stopping.load(Ordering::SeqCst) {
            return Err("cancel_degraded_no_automatic_namespace_shutdown".into());
        }
        let human = state.endpoints[endpoint].role == Role::Human;
        let value = state.attempts.get_mut(id).ok_or("unknown_attempt")?;
        if !value
            .view
            .origin
            .as_ref()
            .is_some_and(|o| o.endpoint == origin_endpoint && o.request_id == origin_request_id)
        {
            return Err("cancel_origin_stale".into());
        }
        if !human && origin_endpoint != endpoint {
            return Err("cancel_requires_launch_owner_or_human".into());
        }
        let status = if value.exited {
            wire::CancelRequestStatus::AlreadyExited
        } else if value.view.state == "unknown" {
            return Err("cancel_outcome_unknown_inspect_no_retry".into());
        } else if value.view.cancellation.is_some() {
            wire::CancelRequestStatus::AlreadyRequested
        } else {
            value.view.cancellation = Some(wire::AttemptCancellation {
                requested_by: endpoint.into(),
                request_id,
                status: wire::CancellationState::Requested,
            });
            value.cancel.store(true, Ordering::Release);
            wire::CancelRequestStatus::Requested
        };
        state
            .endpoints
            .get_mut(endpoint)
            .unwrap()
            .decisions
            .get_mut(&request_id)
            .unwrap()
            .receipt
            .result = Some(MutationResult::CancellationRequested {
            attempt: id.into(),
            status,
        });
        Ok((
            "applied".into(),
            0,
            "exact_attempt_cancel_requested_not_completion_or_descendant_stop".into(),
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
