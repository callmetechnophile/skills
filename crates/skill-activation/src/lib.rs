use agent_orchestrator::ExecutionPlan;
use chrono::{DateTime, Utc};
use runtime_adapters::InstructionPayload;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ActivationError {
    #[error("Activation error: {0}")]
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum ActivationStatus {
    ACTIVATED,
    REQUIRES_INSTALLATION,
    BLOCKED,
    FAILED,
}

impl ActivationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActivationStatus::ACTIVATED => "ACTIVATED",
            ActivationStatus::REQUIRES_INSTALLATION => "REQUIRES_INSTALLATION",
            ActivationStatus::BLOCKED => "BLOCKED",
            ActivationStatus::FAILED => "FAILED",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillActivationRecord {
    #[serde(rename = "workPackageId")]
    pub work_package_id: String,
    #[serde(rename = "skillName")]
    pub skill_name: String,
    pub status: String,
    #[serde(rename = "sourceType")]
    pub source_type: String,
    #[serde(rename = "sourceLocation")]
    pub source_location: String,
    #[serde(rename = "exactDirectory", skip_serializing_if = "Option::is_none")]
    pub exact_directory: Option<String>,
    #[serde(rename = "installCommand", skip_serializing_if = "Option::is_none")]
    pub install_command: Option<String>,
    #[serde(rename = "trustStatus")]
    pub trust_status: String,
    #[serde(rename = "contentHash", skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    pub verified: bool,
    #[serde(rename = "instructionPayload", skip_serializing_if = "Option::is_none")]
    pub instruction_payload: Option<InstructionPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationReport {
    #[serde(rename = "taskId")]
    pub task_id: String,
    #[serde(rename = "generatedAt")]
    pub generated_at: DateTime<Utc>,
    pub agent: String,
    #[serde(rename = "workflowMode")]
    pub workflow_mode: String,
    #[serde(rename = "totalRequested")]
    pub total_requested: usize,
    #[serde(rename = "activatedCount")]
    pub activated_count: usize,
    #[serde(rename = "failedCount")]
    pub failed_count: usize,
    pub activations: Vec<SkillActivationRecord>,
    pub failures: Vec<SkillActivationRecord>,
    #[serde(rename = "executionPlan", skip_serializing_if = "Option::is_none")]
    pub execution_plan: Option<ExecutionPlan>,
}

impl ActivationReport {
    pub fn new(
        task_id: &str,
        agent: &str,
        workflow_mode: &str,
        activations: Vec<SkillActivationRecord>,
        failures: Vec<SkillActivationRecord>,
        execution_plan: Option<ExecutionPlan>,
    ) -> Self {
        let total_requested = activations.len() + failures.len();
        let activated_count = activations
            .iter()
            .filter(|a| a.status == "ACTIVATED")
            .count();
        let failed_count = failures.len();

        Self {
            task_id: task_id.to_string(),
            generated_at: Utc::now(),
            agent: agent.to_string(),
            workflow_mode: workflow_mode.to_string(),
            total_requested,
            activated_count,
            failed_count,
            activations,
            failures,
            execution_plan,
        }
    }

    pub fn to_markdown(&self) -> String {
        let mut lines = vec![
            "# Skill Activation Report".to_string(),
            format!("**Task ID:** `{}`", self.task_id),
            format!("**Timestamp:** {}", self.generated_at),
            format!("**Target Agent:** {}", self.agent),
            format!("**Workflow Mode:** {}", self.workflow_mode),
            "".to_string(),
            "## Summary".to_string(),
            format!("- Total Skills Processed: {}", self.total_requested),
            format!("- Successfully Activated: {}", self.activated_count),
            format!("- Blocked / Failed: {}", self.failed_count),
            "".to_string(),
            "## Activation Records".to_string(),
        ];

        for act in &self.activations {
            lines.push(format!("### [{}] {}", act.status, act.skill_name));
            lines.push(format!("- **Work Package:** `{}`", act.work_package_id));
            lines.push(format!(
                "- **Source:** {} (`{}`)",
                act.source_type, act.source_location
            ));
            lines.push(format!("- **Trust Status:** {}", act.trust_status));
            lines.push(format!(
                "- **Content SHA-256:** `{}`",
                act.content_hash.as_deref().unwrap_or("N/A")
            ));
            lines.push(format!(
                "- **Verification:** {}",
                if act.verified { "Passed" } else { "Failed" }
            ));
            lines.push("".to_string());
        }

        if !self.failures.is_empty() {
            lines.push("## Blocked & Failed Skills".to_string());
            for fail in &self.failures {
                lines.push(format!("### [BLOCKED] {}", fail.skill_name));
                lines.push(format!("- **Work Package:** `{}`", fail.work_package_id));
                lines.push(format!(
                    "- **Reason Code:** `{}`",
                    fail.reason.as_deref().unwrap_or("UNKNOWN")
                ));
                lines.push(format!(
                    "- **Details:** {}",
                    fail.message.as_deref().unwrap_or("No details")
                ));
                lines.push("".to_string());
            }
        }

        lines.join("\n")
    }
}
