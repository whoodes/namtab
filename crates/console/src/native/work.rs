//! Ephemeral work selection/confirmation, never engine permission or execution policy.
use super::{Model, View};
use project_x_client::{mk1::AttemptTarget, mk1_protocol::Snapshot};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkItem {
    Attempt(String),
    Terminal(String),
    Task(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalTarget {
    pub epoch: String,
    pub session: String,
    pub generation: u64,
}
impl Model {
    pub fn work_items(s: &Snapshot) -> Vec<WorkItem> {
        s.attempts
            .iter()
            .map(|a| WorkItem::Attempt(a.id.clone()))
            .chain(s.sessions.iter().map(|s| WorkItem::Terminal(s.id.clone())))
            .chain(s.tasks.iter().map(|t| WorkItem::Task(t.task.clone())))
            .collect()
    }
    pub fn select_work(&mut self, delta: i8, edge: Option<bool>) {
        self.pause(
            "Work selection changed; application input paused. No implicit terminal association.",
        );
        if let Some(s) = &self.snapshot {
            let items = Self::work_items(s);
            if items.is_empty() {
                return;
            }
            let i = items
                .iter()
                .position(|x| Some(x) == self.work_selection.as_ref())
                .unwrap_or(0);
            let n = match edge {
                Some(false) => 0,
                Some(true) => items.len() - 1,
                None if delta < 0 => i.saturating_sub(1),
                None => (i + 1).min(items.len() - 1),
            };
            self.work_selection = Some(items[n].clone());
        }
    }
    pub fn update_work(&mut self, s: &Snapshot) {
        if self.work_selection.is_none() {
            self.work_selection = Self::work_items(s).into_iter().next();
        }
        if let Some(target) = &self.cancel_confirmation {
            let same = AttemptTarget::observed(s, &target.attempt).is_ok_and(|x| x == *target)
                && self.work_selection == Some(WorkItem::Attempt(target.attempt.clone()))
                && s.degraded.is_none()
                && self.view == View::Work
                && s.attempts.iter().any(|a| {
                    a.id == target.attempt && a.state == "running" && a.cancellation.is_none()
                });
            if !same {
                self.pause("Cancellation target/support/state changed; confirmation dismissed. Inspect again.");
            }
        }
    }
    pub fn open_cancel(&mut self) -> Result<(), &'static str> {
        if !self.online || !self.focused || self.view != View::Work || self.form.is_some() {
            return Err("Cancellation needs focused live Work view; no request sent");
        }
        let Some(WorkItem::Attempt(id)) = &self.work_selection else {
            return Err("Select an attempt row before cancellation");
        };
        let s = self.snapshot.as_ref().ok_or("No canonical observation")?;
        if s.degraded.is_some() {
            return Err("Degraded broker; no cancellation queued");
        }
        let a = s
            .attempts
            .iter()
            .find(|a| &a.id == id)
            .ok_or("Selected attempt unavailable")?;
        if a.state != "running" || a.cancellation.is_some() {
            return Err("No unrequested running attempt; inspect exit/unknown state");
        }
        let target = AttemptTarget::observed(s, id)
            .map_err(|_| "Cancellation unsupported or origin unknown; no fallback")?;
        self.pause("Confirm one direct-child cancellation: plain Enter once, Escape dismiss. Other effects may continue.");
        super::trace::emit(
            serde_json::json!({"event":"cancel_confirmation_open","attempt":target.attempt,"origin_endpoint":target.origin_endpoint,"origin_request_id":target.origin_request_id,"focus_generation":self.fence.get()}),
        );
        self.cancel_confirmation = Some(target);
        Ok(())
    }
    pub fn confirm_cancel(&mut self) -> Result<AttemptTarget, &'static str> {
        let target = self
            .cancel_confirmation
            .take()
            .ok_or("No cancellation confirmation")?;
        if !self.online
            || !self.focused
            || self.view != View::Work
            || self.work_selection != Some(WorkItem::Attempt(target.attempt.clone()))
        {
            return Err("Cancellation context changed; nothing queued");
        }
        let s = self.snapshot.as_ref().ok_or("No canonical observation")?;
        if s.degraded.is_some()
            || !AttemptTarget::observed(s, &target.attempt).is_ok_and(|x| x == target)
        {
            return Err("Cancellation target/support changed; nothing queued");
        }
        super::trace::emit(
            serde_json::json!({"event":"cancel_confirmation_accepted","attempt":target.attempt,"focus_generation":self.fence.get()}),
        );
        Ok(target)
    }
    pub fn terminal_target(&self) -> Result<TerminalTarget, &'static str> {
        if !self.online || !self.focused {
            return Err("Terminal control requires focused live observation");
        }
        let id = if self.view == View::Work {
            match &self.work_selection {
                Some(WorkItem::Terminal(id)) => id,
                _ => return Err("Select a terminal row; managed attempts do not imply a PTY"),
            }
        } else {
            &self.selected
        };
        let s = self.snapshot.as_ref().ok_or("No canonical observation")?;
        let session = s
            .sessions
            .iter()
            .find(|s| &s.id == id)
            .ok_or("Selected terminal unavailable")?;
        Ok(TerminalTarget {
            epoch: s.epoch.clone(),
            session: session.id.clone(),
            generation: session.generation,
        })
    }
    pub fn open_work_terminal(&mut self) -> Result<(), &'static str> {
        let target = self.terminal_target()?;
        self.selected = target.session;
        self.switch(View::Terminal);
        Ok(())
    }
}

impl Model {
    /// Bounded roster + selected detail only. All facts come from a snapshot;
    /// selecting a managed attempt never invents an associated terminal.
    pub fn work_lines(&self) -> Vec<(String, bool)> {
        let Some(s) = &self.snapshot else {
            return vec![("Waiting for engine observation".into(), false)];
        };
        let agents = s
            .endpoints
            .iter()
            .filter(|e| e.role == project_x_client::mk1_protocol::Role::Agent && e.connected)
            .count();
        let mut lines=vec![(format!("WORK | {agents} connected declared agents | {} tasks | {} attempts/2 | {} terminals/2",s.tasks.len(),s.attempts.len(),s.sessions.len()),false),
            ("Up/Down or Home/End select | Enter opens terminal | Ctrl+Shift+X cancels selected attempt".into(),false)];
        let owner = |id: Option<&str>| match id {
            None => "unowned".to_owned(),
            Some(id) => format!(
                "{}{} / {}",
                id,
                if id == self.endpoint { " (you)" } else { "" },
                s.endpoints
                    .iter()
                    .find(|e| e.id == id)
                    .map_or("unknown connection", |e| if e.connected {
                        "connected"
                    } else {
                        "disconnected; effects may continue"
                    })
            ),
        };
        for item in Self::work_items(s) {
            let selected = self.work_selection.as_ref() == Some(&item);
            let line = match &item {
                WorkItem::Attempt(id) => {
                    let a = s.attempts.iter().find(|a| &a.id == id).unwrap();
                    format!(
                        "ATTEMPT {} | task {} | {} | origin {}",
                        a.id,
                        a.task,
                        a.state,
                        a.origin.as_ref().map_or("unknown".into(), |o| format!(
                            "{}#{}",
                            o.endpoint, o.request_id
                        ))
                    )
                }
                WorkItem::Terminal(id) => {
                    let t = s.sessions.iter().find(|t| &t.id == id).unwrap();
                    format!(
                        "TERMINAL {} / {} | owner {} | gen {}",
                        t.id,
                        t.name,
                        owner(t.owner.as_deref()),
                        t.generation
                    )
                }
                WorkItem::Task(id) => {
                    let t = s.tasks.iter().find(|t| &t.task == id).unwrap();
                    format!(
                        "TASK {} | immutable declaration; not acceptance/readiness",
                        t.task
                    )
                }
            };
            lines.push((
                format!("{} {line}", if selected { ">" } else { " " }),
                selected,
            ));
        }
        if Self::work_items(s).is_empty() {
            lines.push((
                "No work: create shell (Ctrl+Shift+N), literal argv (A) or SCRIPTED demo (D)."
                    .into(),
                false,
            ));
        }
        lines.push((String::new(), false));
        if let Some(target) = &self.cancel_confirmation {
            lines.push((
                format!(
                    "CONFIRM CANCEL {} | origin {}#{}",
                    target.attempt, target.origin_endpoint, target.origin_request_id
                ),
                true,
            ));
            lines.push(("Plain Enter once requests one DIRECT child stop; Escape dismisses. Other effects may continue.".into(),true));
            lines.push((format!("Frozen epoch: {}", target.epoch), false));
            return lines;
        }
        match &self.work_selection {
            Some(WorkItem::Attempt(id)) => {
                if let Some(a) = s.attempts.iter().find(|a| &a.id == id) {
                    lines.push((
                        format!("SELECTED {} / task {} | process {}", a.id, a.task, a.state),
                        true,
                    ));
                    lines.push((
                        format!(
                            "Exit: {} | cancellation: {}",
                            a.exit_code.map_or_else(
                                || a.exit_signal
                                    .map_or("not observed/unknown".into(), |sig| format!(
                                        "signal{sig} (observed)"
                                    )),
                                |c| format!("code{c} (observed)")
                            ),
                            a.cancellation
                                .as_ref()
                                .map_or("not requested".into(), |c| format!(
                                    "{}; requested by {}#{}",
                                    cancellation_label(&c.status),
                                    c.requested_by,
                                    c.request_id
                                ))
                        ),
                        false,
                    ));
                    if self.details {
                        lines.push((
                            format!(
                                "Task revision: {}",
                                a.task_revision.as_deref().unwrap_or("ad hoc/unknown")
                            ),
                            false,
                        ));
                    } else {
                        lines.push(("Ctrl+Shift+E for exact revision. Model turns unobserved; managed pipes are not a PTY.".into(),false));
                    }
                    for (name, text) in [("stdout", &a.stdout), ("stderr", &a.stderr)] {
                        lines.push((
                            format!(
                                "{name} preview: {}{}",
                                text.chars()
                                    .take(160)
                                    .collect::<String>()
                                    .replace('\n', " | "),
                                if a.truncated || text.chars().count() > 160 {
                                    " [TRUNCATED]"
                                } else {
                                    ""
                                }
                            ),
                            false,
                        ));
                    }
                } else {
                    lines.push((
                        "Selected attempt unavailable; choose again, no retarget.".into(),
                        true,
                    ));
                }
            }
            Some(WorkItem::Terminal(id)) => {
                if let Some(t) = s.sessions.iter().find(|t| &t.id == id) {
                    lines.push((
                        format!(
                            "SELECTED TERMINAL {} | owner {} | generation {}",
                            t.id,
                            owner(t.owner.as_deref()),
                            t.generation
                        ),
                        true,
                    ));
                    lines.push(("Enter opens canonical terminal PAUSED. Take input explicitly for this exact generation.".into(),false));
                    lines.push((format!("Interpretation {} | child exit {} | no inferred shell-command attribution",if t.screen.input_known{"known"}else{"UNKNOWN"},t.exit_code.map_or("not observed".into(),|code|format!("{code} (observed)"))),false));
                } else {
                    lines.push(("Selected terminal unavailable; choose again.".into(), true));
                }
            }
            Some(WorkItem::Task(id)) => {
                if let Some(t) = s.tasks.iter().find(|t| &t.task == id) {
                    lines.push((format!("SELECTED TASK {} | {}", t.task, t.goal), true));
                    if self.details {
                        lines.push((format!("Exact revision: {}", t.revision), false));
                    }
                    lines.push(("A declaration is not agent progress, artifact correctness or accepted completion.".into(),false));
                }
            }
            None => {}
        }
        if !s.artifacts.is_empty() {
            lines.push((
                format!(
                    "{} observed file revisions; observation is not correctness/producer causality",
                    s.artifacts.len()
                ),
                false,
            ));
        }
        if let Some(form) = &self.form {
            lines.push((
                "RUN ARGV / literal JSON, not an AI prompt / Enter once, Escape dismiss".into(),
                true,
            ));
            lines.push((form.clone(), false));
        }
        lines
    }
}

fn cancellation_label(status: &project_x_client::mk1_protocol::CancellationState) -> &'static str {
    use project_x_client::mk1_protocol::CancellationState;
    match status {
        CancellationState::Requested => "requested, exit not observed",
        CancellationState::ExitObserved => "exit observed",
        CancellationState::Unknown => "UNKNOWN, inspect without retry",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use project_x_client::mk1_protocol as p;
    fn snapshot() -> Snapshot {
        serde_json::from_value(serde_json::json!({"version":p::VERSION,"epoch":"a".repeat(64),"endpoint":"human","capabilities":[p::ATTEMPT_CANCEL],"endpoints":[],"tasks":[{"task":"t","revision":"sha256:fixture","goal":"synthetic","expected_artifacts":[]}],"artifacts":[],"receipts":[],"degraded":null,"attempts":[{"id":"a","task":"t","task_revision":"sha256:fixture","origin":{"endpoint":"agent","request_id":4},"state":"running","exit_code":null,"stdout":"A_ONLY","stderr":"A_ERR","truncated":false},{"id":"b","task":"t","task_revision":"sha256:fixture","origin":{"endpoint":"peer","request_id":2},"state":"running","exit_code":null,"stdout":"B_ONLY","stderr":"B_ERR","truncated":false}],"sessions":[{"id":"s1","name":"first","owner":"agent","generation":2,"process":"running","exit_code":null,"screen":{"cols":2,"rows":2,"lines":["", ""],"revision":1,"mode_revision":1,"cursor_col":0,"cursor_row":0,"cursor_visible":true,"input_known":true,"alternate":false,"diagnostics":[]}},{"id":"s2","name":"second","owner":"peer","generation":9,"process":"running","exit_code":null,"screen":{"cols":2,"rows":2,"lines":["", ""],"revision":1,"mode_revision":1,"cursor_col":0,"cursor_row":0,"cursor_visible":true,"input_known":true,"alternate":false,"diagnostics":[]}}]})).unwrap()
    }
    fn model() -> Model {
        let mut m = Model {
            online: true,
            focused: true,
            endpoint: "human".into(),
            ..Model::default()
        };
        m.update(snapshot());
        m
    }
    #[test]
    fn stable_selection_is_not_terminal_or_input_authority() {
        let mut m = model();
        assert_eq!(m.work_selection, Some(WorkItem::Attempt("a".into())));
        assert!(m.terminal_target().is_err());
        assert!(m.open_work_terminal().is_err());
        m.select_work(1, None);
        assert_eq!(m.work_selection, Some(WorkItem::Attempt("b".into())));
        let mut s = snapshot();
        s.attempts.remove(0);
        m.update(s);
        assert_eq!(m.work_selection, Some(WorkItem::Attempt("b".into())));
        m.select_work(1, None);
        assert_eq!(m.work_selection, Some(WorkItem::Terminal("s1".into())));
        let t = m.terminal_target().unwrap();
        assert_eq!(t.session, "s1");
        assert_eq!(t.generation, 2);
        m.open_work_terminal().unwrap();
        assert_eq!(m.view, View::Terminal);
        assert!(m.paused);
        assert!(m.input(p::Intent::Text { text: "x".into() }).is_err());
    }
    #[test]
    fn confirmation_is_one_shot_and_each_context_change_dismisses() {
        let mut m = model();
        m.open_cancel().unwrap();
        let target = m.confirm_cancel().unwrap();
        assert_eq!(target.attempt, "a");
        assert!(m.confirm_cancel().is_err());
        for reason in ["Focus lost", "Disconnected", "Resize", "Escape"] {
            let mut m = model();
            m.open_cancel().unwrap();
            m.pause(reason);
            assert!(m.confirm_cancel().is_err());
        }
        let mut m = model();
        m.open_cancel().unwrap();
        m.select_work(1, None);
        assert!(m.confirm_cancel().is_err());
        let mut m = model();
        m.open_cancel().unwrap();
        m.switch(View::Terminal);
        assert!(m.confirm_cancel().is_err());
    }
    #[test]
    fn stale_epoch_origin_capability_exit_degradation_or_missing_target_invalidates() {
        for case in 0..7 {
            let mut m = model();
            m.open_cancel().unwrap();
            let mut s = snapshot();
            match case {
                0 => s.epoch = "b".repeat(64),
                1 => s.attempts[0].origin.as_mut().unwrap().request_id = 99,
                2 => s.capabilities.clear(),
                3 => s.attempts[0].state = "exited".into(),
                4 => s.degraded = Some("unknown".into()),
                5 => {
                    s.attempts.remove(0);
                }
                _ => {
                    s.attempts[0].state = "unknown".into();
                }
            }
            m.update(s);
            assert!(m.confirm_cancel().is_err(), "case {case}");
        }
        let mut m = model();
        m.online = false;
        assert!(m.open_cancel().is_err());
        let mut m = model();
        m.focused = false;
        assert!(m.open_cancel().is_err());
    }
    #[test]
    fn selected_detail_does_not_dump_peer_output_and_badges_do_not_grant() {
        let mut m = model();
        let lines = m
            .work_lines()
            .into_iter()
            .map(|x| x.0)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(lines.contains("A_ONLY"));
        assert!(!lines.contains("B_ONLY"));
        assert!(lines.contains("origin peer#2"));
        m.select_work(1, None);
        let lines = m
            .work_lines()
            .into_iter()
            .map(|x| x.0)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(lines.contains("B_ONLY"));
        assert!(!lines.contains("A_ONLY"));
        m.next_session();
        assert!(matches!(m.work_selection, Some(WorkItem::Terminal(_))));
        assert!(m.paused);
    }
}
