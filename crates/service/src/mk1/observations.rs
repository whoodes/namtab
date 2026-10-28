//! Explicit bounded pull projections. Never build a full snapshot to filter it.
use super::*;

pub(super) fn capabilities() -> Vec<String> {
    [
        wire::HUMAN_SHUTDOWN,
        wire::ATTEMPT_CANCEL,
        wire::SHARED_TASKS,
        wire::ATTEMPT_ORIGIN,
        wire::RECEIPT_RESULTS,
        wire::SCOPED_OBSERVE,
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

impl Broker {
    pub(super) fn scoped_observation(
        &self,
        endpoint: &str,
        epoch: &str,
        scope: &wire::ObservationScope,
    ) -> Result<wire::ScopedObservation, String> {
        if epoch != self.epoch {
            return Err("observation_epoch_stale".into());
        }
        if !scope.valid() {
            return Err("observation_target_invalid".into());
        }
        let state = self.state.lock().unwrap();
        let view = match scope {
            wire::ObservationScope::Work {} => wire::ObservationView::Work {
                endpoints: state
                    .endpoints
                    .iter()
                    .map(|(id, e)| wire::EndpointView {
                        id: id.clone(),
                        role: e.role.clone(),
                        connected: e.connected,
                    })
                    .collect(),
                capabilities: capabilities(),
                tasks: state.tasks.values().cloned().collect(),
                sessions: state
                    .sessions
                    .values()
                    .map(|s| wire::SessionSummary::from(&s.view))
                    .collect(),
                attempts: state
                    .attempts
                    .values()
                    .map(|a| wire::AttemptSummary::from(&a.view))
                    .collect(),
            },
            wire::ObservationScope::Session { session } => wire::ObservationView::Session {
                session: state
                    .sessions
                    .get(session)
                    .ok_or("unknown_session")?
                    .view
                    .clone(),
            },
            wire::ObservationScope::Attempt { attempt } => {
                let a = state.attempts.get(attempt).ok_or("unknown_attempt")?;
                let mut view = a.view.clone();
                let (stdout, out_cut) = safe_output(&a.stdout);
                let (stderr, err_cut) = safe_output(&a.stderr);
                view.stdout = stdout;
                view.stderr = stderr;
                view.truncated |= out_cut || err_cut;
                wire::ObservationView::Attempt { attempt: view }
            }
        };
        Ok(wire::ScopedObservation {
            version: wire::VERSION.into(),
            profile: wire::OBSERVATION_PROFILE.into(),
            epoch: self.epoch.clone(),
            endpoint: endpoint.into(),
            degraded: state.degraded.clone(),
            view,
        })
    }
}
