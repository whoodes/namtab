//! Canonical worker orchestration and sole final PTY write boundary.
use super::*;
impl Broker {
    pub(super) fn owner(
        session: &LiveSession,
        endpoint: &str,
        generation: u64,
    ) -> Result<(), String> {
        if session.view.owner.as_deref() != Some(endpoint) || session.view.generation != generation
        {
            return Err("writer_or_generation_stale".into());
        }
        if !session.view.screen.input_known || session.view.process != "running" {
            return Err("interpretation_or_process_unavailable".into());
        }
        Ok(())
    }
    pub(super) fn deliver(
        &self,
        endpoint: &str,
        operation: &Op,
    ) -> Result<(String, usize, String), String> {
        let Op::Input {
            session,
            generation,
            revision,
            mode_revision,
            policy,
            intent,
        } = operation
        else {
            return Err("not_input".into());
        };
        let (generation, revision, mode_revision) = (*generation, *revision, *mode_revision);
        {
            let state = self.state.lock().unwrap();
            let value = state.sessions.get(session).ok_or("unknown_session")?;
            Self::owner(value, endpoint, generation)?;
            Self::fresh(
                value,
                revision,
                mode_revision,
                policy,
                &state.endpoints[endpoint].role,
            )?;
        }
        let encoded = self.worker_call(Operation::Encode {
            session: session.into(),
            input: input(intent)?,
        })?;
        let Reply::Encoded {
            bytes,
            revision: encoded_revision,
            mode_revision: encoded_mode,
        } = encoded
        else {
            self.worker.stop();
            self.fence_worker();
            return Err("worker_reply_kind".into());
        };
        if bytes.len() > platform::MAX_WRITE_BYTES {
            return Err("encoded_input_exceeds_single_write_capacity".into());
        }
        let mut state = self.state.lock().unwrap();
        let role = state.endpoints[endpoint].role.clone();
        let value = state.sessions.get_mut(session).ok_or("unknown_session")?;
        if self.worker.failed() {
            return Err("worker_unavailable_before_final_write".into());
        }
        Self::owner(value, endpoint, generation)?;
        Self::fresh(value, revision, mode_revision, policy, &role)?;
        if value.pending_output != 0
            || value.view.screen.mode_revision != encoded_mode
            || (*policy == InputPolicy::Observed && value.view.screen.revision != encoded_revision)
        {
            return Err("output_or_mode_changed_before_final_write".into());
        }
        let delivered = match value.writer.write_once(&bytes) {
            Ok(n) => n,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => 0,
            Err(_) => {
                value.view.screen.input_known = false;
                value.view.owner = None;
                value.view.generation += 1;
                return Ok((
                    "unknown".into(),
                    0,
                    "pty_write_failed_extent_unknown_no_retry".into(),
                ));
            }
        };
        if delivered < bytes.len() {
            value.view.screen.input_known = false;
            value.view.owner = None;
            value.view.generation += 1;
            return Ok((
                if delivered == 0 {
                    "rejected"
                } else {
                    "partial"
                }
                .into(),
                delivered,
                "exact_prefix_only_input_continuation_fenced_no_automatic_suffix".into(),
            ));
        }
        Ok((
            "applied".into(),
            delivered,
            "accepted_by_local_pty_not_application_completion".into(),
        ))
    }
    pub(super) fn fresh(
        value: &LiveSession,
        revision: u64,
        mode_revision: u64,
        policy: &InputPolicy,
        role: &Role,
    ) -> Result<(), String> {
        if value.pending_output != 0 {
            return Err("canonical_output_pending_observe_again".into());
        }
        if value.view.screen.mode_revision != mode_revision {
            return Err("mode_revision_stale".into());
        }
        match policy {
            InputPolicy::Observed if value.view.screen.revision != revision => {
                Err("screen_revision_stale".into())
            }
            InputPolicy::HumanLive if *role != Role::Human => {
                Err("human_live_requires_human_endpoint".into())
            }
            _ => Ok(()),
        }
    }
    pub(super) fn spec(&self, program: OsString, args: Vec<OsString>) -> CommandSpec {
        let mut spec = CommandSpec {
            program,
            args,
            cwd: self.config.workspace.clone(),
            env: vec![
                ("PATH".into(), "/usr/bin:/bin".into()),
                ("LC_ALL".into(), "C.UTF-8".into()),
                ("TERM".into(), "dumb".into()),
                ("HOME".into(), self.config.workspace.as_os_str().to_owned()),
                ("INPUTRC".into(), "/dev/null".into()),
                ("BASH_ENV".into(), "/dev/null".into()),
                ("ENV".into(), "/dev/null".into()),
                ("HISTFILE".into(), "/dev/null".into()),
                ("PS1".into(), "mk1$ ".into()),
            ],
        };
        if let Some(path) = &self.config.terminfo {
            spec.env.retain(|(key, _)| key != "TERM");
            spec.env.push(("TERM".into(), "namtab-mk1".into()));
            spec.env
                .push(("TERMINFO".into(), path.as_os_str().to_owned()));
        }
        spec
    }
    pub(super) fn create_session(
        self: &Arc<Self>,
        name: &str,
    ) -> Result<(String, usize, String), String> {
        let id = {
            let mut state = self.state.lock().unwrap();
            if state.sessions.len() + state.reserved_sessions >= 2 {
                return Err("session_capacity_two_total".into());
            }
            state.reserved_sessions += 1;
            state.sequence += 1;
            format!("session{}", state.sequence)
        };
        let result = (|| {
            let Reply::Frame(frame) = self.worker_call(Operation::New {
                session: id.clone(),
                cols: 80,
                rows: 24,
            })?
            else {
                return Err("worker_reply_kind".into());
            };
            let spec = self.spec(
                "/bin/bash".into(),
                vec!["--noprofile".into(), "--norc".into()],
            );
            let process = platform::launch_pty(
                &self.config.pty_helper,
                &spec,
                Geometry { cols: 80, rows: 24 },
            )
            .map_err(|_| "pty_launch_failed")?;
            let mut state = self.state.lock().unwrap();
            state.sessions.insert(
                id.clone(),
                LiveSession {
                    view: Session {
                        id: id.clone(),
                        name: name.into(),
                        owner: None,
                        generation: 1,
                        process: "running".into(),
                        exit_code: None,
                        screen: screen(frame),
                    },
                    writer: process.writer,
                    pending_output: 0,
                    exited: false,
                },
            );
            drop(state);
            let broker = Arc::clone(self);
            let reader_id = id.clone();
            std::thread::spawn(move || {
                broker.pty_reader(reader_id, process.reader, process.process)
            });
            Ok((
                "applied".into(),
                0,
                if self.config.terminfo.is_some() {
                    "real_clean_bash_controlling_pty_started_private_terminfo_developer_host"
                } else {
                    "real_clean_bash_controlling_pty_started_term_dumb_developer_host"
                }
                .into(),
            ))
        })();
        self.state.lock().unwrap().reserved_sessions -= 1;
        if result.is_err() {
            let _ = self.worker_call(Operation::Close { session: id });
        }
        result
    }
    pub(super) fn pty_reader(
        self: Arc<Self>,
        id: String,
        mut reader: platform::PtyReader,
        mut process: ProcessHandle,
    ) {
        let mut bytes = [0_u8; 512];
        let mut stream_closed = false;
        loop {
            if self.stopping.load(Ordering::Acquire) {
                let _ = process.kill_and_wait();
                break;
            }
            if let Ok(Some(status)) = process.try_wait()
                && let Some(value) = self.state.lock().unwrap().sessions.get_mut(&id)
            {
                value.view.process = "draining".into();
                value.view.exit_code = status.code();
                value.exited = true;
            }
            if stream_closed {
                let mut state = self.state.lock().unwrap();
                let value = state.sessions.get_mut(&id).unwrap();
                if value.exited {
                    value.view.process = "exited".into();
                    break;
                }
                drop(state);
                std::thread::sleep(Duration::from_millis(20));
                continue;
            }
            match reader.read_chunk(&mut bytes, Duration::from_millis(100)) {
                Ok(ReadOutcome::Timeout) => continue,
                Ok(ReadOutcome::Data(n)) => {
                    if self.worker.failed() {
                        continue;
                    }
                    if let Some(value) = self.state.lock().unwrap().sessions.get_mut(&id) {
                        value.pending_output += 1;
                    }
                    match self.worker_call(Operation::Feed {
                        session: id.clone(),
                        bytes: bytes[..n].to_vec(),
                    }) {
                        Ok(Reply::Fed { frame, replies }) => {
                            let mut state = self.state.lock().unwrap();
                            if self.worker.failed() {
                                continue;
                            }
                            if !replies.is_empty() {
                                state.system_sequence += 1;
                                let seq = state.system_sequence;
                                if state
                                    .journal
                                    .record("system", seq, "query_reply", "reserved", 0)
                                    .is_err()
                                {
                                    state.degraded = Some("query_journal_failed".into());
                                    if let Some(value) = state.sessions.get_mut(&id) {
                                        value.view.screen.input_known = false;
                                        value.view.owner = None;
                                        value.view.generation += 1;
                                    }
                                    break;
                                }
                            }
                            let Some(value) = state.sessions.get_mut(&id) else {
                                break;
                            };
                            if frame.revision >= value.view.screen.revision {
                                value.view.screen = screen(frame);
                            }
                            value.pending_output = value.pending_output.saturating_sub(1);
                            if !replies.is_empty() {
                                let delivered = value.writer.write_once(&replies);
                                let (status, extent) = match delivered {
                                    Ok(n) if n == replies.len() => ("applied", n),
                                    Ok(n) => {
                                        value.view.screen.input_known = false;
                                        value.view.owner = None;
                                        value.view.generation += 1;
                                        (if n == 0 { "rejected" } else { "partial" }, n)
                                    }
                                    Err(_) => {
                                        value.view.screen.input_known = false;
                                        value.view.owner = None;
                                        value.view.generation += 1;
                                        ("unknown", 0)
                                    }
                                };
                                let seq = state.system_sequence;
                                if state
                                    .journal
                                    .record("system", seq, "query_reply", status, extent)
                                    .is_err()
                                {
                                    state.degraded = Some("query_completion_unknown".into());
                                }
                            }
                        }
                        _ => {
                            self.worker.stop();
                            self.fence_worker();
                            break;
                        }
                    }
                }
                Ok(ReadOutcome::Eof) => {
                    stream_closed = true;
                    if let Some(value) = self.state.lock().unwrap().sessions.get_mut(&id) {
                        value.view.screen.input_known = false;
                        value.view.owner = None;
                        value.view.generation += 1;
                    }
                }
                Err(_) => {
                    if let Some(value) = self.state.lock().unwrap().sessions.get_mut(&id) {
                        value.view.process = "unknown".into();
                        value.view.screen.input_known = false;
                        value.view.owner = None;
                        value.view.generation += 1;
                    }
                    break;
                }
            }
        }
    }
    pub(super) fn resize(
        &self,
        endpoint: &str,
        id: &str,
        generation: u64,
        cols: u16,
        rows: u16,
    ) -> Result<(String, usize, String), String> {
        if !(2..=160).contains(&cols) || !(2..=48).contains(&rows) {
            return Err("geometry_outside_mk1".into());
        }
        {
            let mut state = self.state.lock().unwrap();
            let value = state.sessions.get_mut(id).ok_or("unknown_session")?;
            Self::owner(value, endpoint, generation)?;
            if value.pending_output != 0 {
                return Err("canonical_output_pending".into());
            }
            value.pending_output += 1;
        }
        let result = self.worker_call(Operation::Resize {
            session: id.into(),
            cols,
            rows,
        });
        let mut state = self.state.lock().unwrap();
        let value = state.sessions.get_mut(id).ok_or("unknown_session")?;
        let frame = match result {
            Ok(Reply::Frame(frame)) => frame,
            _ => {
                value.pending_output = value.pending_output.saturating_sub(1);
                value.view.screen.input_known = false;
                return Err("resize_worker_unavailable".into());
            }
        };
        if Self::owner(value, endpoint, generation).is_err() {
            value.pending_output = value.pending_output.saturating_sub(1);
            value.view.screen.input_known = false;
            value.view.owner = None;
            value.view.generation += 1;
            return Ok((
                "unknown".into(),
                0,
                "takeover_during_canonical_resize_no_old_writer_ioctl".into(),
            ));
        }
        if value.writer.resize(Geometry { cols, rows }).is_err() {
            value.view.screen.input_known = false;
            value.view.owner = None;
            value.view.generation += 1;
            value.pending_output = value.pending_output.saturating_sub(1);
            return Ok((
                "unknown".into(),
                0,
                "canonical_resize_committed_ioctl_failed_no_rollback".into(),
            ));
        }
        if frame.revision >= value.view.screen.revision {
            value.view.screen = screen(frame);
        }
        value.pending_output = value.pending_output.saturating_sub(1);
        Ok((
            "applied".into(),
            0,
            "canonical_geometry_and_pty_resize_observed_repaint_async".into(),
        ))
    }
}
