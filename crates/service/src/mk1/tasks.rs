//! Four immutable RAM contracts; declared expectations are not artifact validation.
use super::*;

impl Broker {
    pub(super) fn define_task(
        &self,
        endpoint: &str,
        request_id: u64,
        task: &str,
        goal: &str,
        expected_artifacts: &[String],
    ) -> Result<(String, usize, String), String> {
        wire::validate_task_definition(task, goal, expected_artifacts)?;
        let mut state = self.state.lock().unwrap();
        if let Some(contract) = state.tasks.get(task) {
            if contract.goal != goal || contract.expected_artifacts != expected_artifacts {
                return Err("task_contract_immutable_conflict".into());
            }
        } else {
            if state.tasks.len() >= 4 {
                return Err("task_contract_capacity_four_total".into());
            }
            // Domain-separated canonical UTF-8 data, never persisted to the
            // metadata journal. Array order and exact goal bytes are significant.
            let body = serde_json::to_vec(&(wire::SHARED_TASKS, task, goal, expected_artifacts))
                .map_err(|_| "task_revision")?;
            let revision = format!("sha256:{:x}", Sha256::digest(&body));
            state.tasks.insert(
                task.into(),
                TaskContract {
                    task: task.into(),
                    revision,
                    goal: goal.into(),
                    expected_artifacts: expected_artifacts.to_vec(),
                },
            );
        }
        let revision = state.tasks[task].revision.clone();
        state
            .endpoints
            .get_mut(endpoint)
            .unwrap()
            .decisions
            .get_mut(&request_id)
            .unwrap()
            .receipt
            .result = Some(MutationResult::TaskDefined {
            task: task.into(),
            revision,
        });
        Ok((
            "applied".into(),
            0,
            "immutable_ram_task_declared_not_scheduled_or_validated".into(),
        ))
    }
}
