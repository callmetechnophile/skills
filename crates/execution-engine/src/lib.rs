use agent_orchestrator::ExecutionPlan;
use runtime_adapters::InstructionPayload;
use serde::{Deserialize, Serialize};
use skill_activation::SkillActivationRecord;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Missing payload for work package: {0}")]
    MissingPayload(String),
    #[error("Execution error: {0}")]
    ExecutionFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionHandoff {
    pub package_id: String,
    pub title: String,
    pub payload: Option<InstructionPayload>,
    pub execution_mode: String,
    pub wave_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedExecutionBatch {
    pub workflow_mode: String,
    pub total_waves: usize,
    pub total_packages: usize,
    pub handoffs: Vec<ExecutionHandoff>,
}

pub struct ExecutionEngine;

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self
    }
}

impl ExecutionEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn format_handoff(
        package_id: &str,
        title: &str,
        payload: InstructionPayload,
    ) -> ExecutionHandoff {
        ExecutionHandoff {
            package_id: package_id.to_string(),
            title: title.to_string(),
            payload: Some(payload),
            execution_mode: "sequential".to_string(),
            wave_index: 0,
        }
    }

    /// Prepares execution handoffs for each work package according to scheduled plan waves
    pub fn prepare_execution(
        plan: &ExecutionPlan,
        activations: &[SkillActivationRecord],
    ) -> PreparedExecutionBatch {
        let mut payload_map: HashMap<String, InstructionPayload> = HashMap::new();
        for act in activations {
            if let Some(ref p) = act.instruction_payload {
                payload_map.insert(act.work_package_id.clone(), p.clone());
            }
        }

        let mut handoffs = Vec::new();
        for (wave_idx, wave) in plan.waves.iter().enumerate() {
            for wp in wave {
                let payload = payload_map.get(&wp.id).cloned();
                handoffs.push(ExecutionHandoff {
                    package_id: wp.id.clone(),
                    title: wp.component.clone(),
                    payload,
                    execution_mode: plan.actual_execution.clone(),
                    wave_index: wave_idx,
                });
            }
        }

        PreparedExecutionBatch {
            workflow_mode: plan.selected_mode.clone(),
            total_waves: plan.waves.len(),
            total_packages: handoffs.len(),
            handoffs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_orchestrator::IntegrationStep;
    use issue_analyzer::WorkPackage;

    fn create_dummy_wp(id: &str, component: &str) -> WorkPackage {
        WorkPackage {
            id: id.to_string(),
            component: component.to_string(),
            title: format!("Task for {}", component),
            objective: format!("Objective {}", component),
            skill_query: format!("query {}", component),
            dependencies: vec![],
            independent: true,
        }
    }

    #[test]
    fn test_format_handoff() {
        let payload = InstructionPayload {
            work_package_id: "WP-01".to_string(),
            skill_name: "test-skill".to_string(),
            component: "auth".to_string(),
            instruction_body: "Run auth tests".to_string(),
            antigravity_contract: None,
        };

        let handoff = ExecutionEngine::format_handoff("WP-01", "auth", payload.clone());
        assert_eq!(handoff.package_id, "WP-01");
        assert_eq!(handoff.title, "auth");
        assert_eq!(handoff.execution_mode, "sequential");
        assert_eq!(handoff.wave_index, 0);
        assert_eq!(handoff.payload.unwrap().skill_name, "test-skill");
    }

    #[test]
    fn test_prepare_execution_multi_wave() {
        let wp1 = create_dummy_wp("WP-01", "frontend");
        let wp2 = create_dummy_wp("WP-02", "backend");
        let wp3 = create_dummy_wp("WP-03", "database");

        let plan = ExecutionPlan {
            selected_mode: "multi-agent".to_string(),
            actual_execution: "parallel-subagents".to_string(),
            parallel_enabled: true,
            waves: vec![vec![wp1.clone()], vec![wp2.clone(), wp3.clone()]],
            linear_order: vec![wp1.clone(), wp2.clone(), wp3.clone()],
            explanation: "Parallel execution".to_string(),
            downgrade_notice: None,
            integration_step: IntegrationStep {
                id: "INT-01".to_string(),
                objective: "Integrate".to_string(),
                dependencies: vec![
                    "WP-01".to_string(),
                    "WP-02".to_string(),
                    "WP-03".to_string(),
                ],
            },
        };

        let payload1 = InstructionPayload {
            work_package_id: "WP-01".to_string(),
            skill_name: "react-skill".to_string(),
            component: "frontend".to_string(),
            instruction_body: "Build UI".to_string(),
            antigravity_contract: None,
        };

        let activation1 = SkillActivationRecord {
            work_package_id: "WP-01".to_string(),
            skill_name: "react-skill".to_string(),
            status: "ACTIVATED".to_string(),
            source_type: "local".to_string(),
            source_location: "skills/react".to_string(),
            exact_directory: None,
            install_command: None,
            trust_status: "APPROVED".to_string(),
            content_hash: Some("hash1".to_string()),
            verified: true,
            instruction_payload: Some(payload1),
            reason: None,
            message: None,
            note: None,
        };

        let batch = ExecutionEngine::prepare_execution(&plan, &[activation1]);
        assert_eq!(batch.workflow_mode, "multi-agent");
        assert_eq!(batch.total_waves, 2);
        assert_eq!(batch.total_packages, 3);
        assert_eq!(batch.handoffs.len(), 3);

        // Wave 0
        assert_eq!(batch.handoffs[0].package_id, "WP-01");
        assert_eq!(batch.handoffs[0].wave_index, 0);
        assert!(batch.handoffs[0].payload.is_some());

        // Wave 1
        assert_eq!(batch.handoffs[1].package_id, "WP-02");
        assert_eq!(batch.handoffs[1].wave_index, 1);
        assert!(batch.handoffs[1].payload.is_none()); // No activation payload provided for WP-02

        assert_eq!(batch.handoffs[2].package_id, "WP-03");
        assert_eq!(batch.handoffs[2].wave_index, 1);
    }
}
