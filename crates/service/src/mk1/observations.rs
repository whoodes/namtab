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
        wire::SCOPED_WORK_EXPORT,
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
        if matches!(scope, wire::ObservationScope::WorkExport {}) {
            self.refresh_artifacts();
        }
        let state = self.state.lock().unwrap();
        let view = match scope {
            wire::ObservationScope::Work {} | wire::ObservationScope::WorkExport {} => {
                let capabilities = capabilities();
                let tasks = state.tasks.values().cloned().collect();
                let sessions = state
                    .sessions
                    .values()
                    .map(|s| wire::SessionSummary::from(&s.view))
                    .collect();
                let attempts = state
                    .attempts
                    .values()
                    .map(|a| {
                        let mut summary = wire::AttemptSummary::from(&a.view);
                        if matches!(scope, wire::ObservationScope::WorkExport {}) {
                            // Preserve legacy MCP's sanitized-source loss flag without
                            // building/exporting bodies. Plain Work retains raw flags.
                            summary.source_truncated |=
                                output_truncated(&a.stdout) || output_truncated(&a.stderr);
                        }
                        summary
                    })
                    .collect();
                if matches!(scope, wire::ObservationScope::WorkExport {}) {
                    wire::ObservationView::WorkExport {
                        capabilities,
                        tasks,
                        sessions,
                        attempts,
                        artifacts: state.artifacts.values().cloned().collect(),
                    }
                } else {
                    wire::ObservationView::Work {
                        endpoints: state
                            .endpoints
                            .iter()
                            .map(|(id, e)| wire::EndpointView {
                                id: id.clone(),
                                role: e.role.clone(),
                                connected: e.connected,
                            })
                            .collect(),
                        capabilities,
                        tasks,
                        sessions,
                        attempts,
                    }
                }
            }
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
