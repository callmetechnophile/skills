use agent_orchestrator::ExecutionPlan;
use chrono::{DateTime, Utc};
use runtime_adapters::{AgentRuntime, InstructionPayload};
use serde::{Deserialize, Serialize};
use skill_registry::compute_sha256;
pub use skill_security::TrustStatus;
use skill_security::{SecurityAssessment, TrustPolicy};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ActivationError {
    #[error("Activation error: {0}")]
    Failed(String),
    #[error("Security blocked: {0}")]
    SecurityBlocked(String),
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

pub struct ActivationEngine;

impl ActivationEngine {
    /// Strictly validates security policy, content hash integrity, and runtime adapter compatibility.
    /// Guarantees that:
    /// - REJECTED, QUARANTINED, and UNSCANNED skills are never activated.
    /// - RESTRICTED skills require explicit confirmation (force = true).
    /// - Loaded memory bytes match the cryptographic hash in the SecurityAssessment.
    #[allow(clippy::too_many_arguments)]
    pub fn activate(
        work_package_id: &str,
        component: &str,
        skill_name: &str,
        raw_content: &str,
        source_type: &str,
        source_location: &str,
        assessment: &SecurityAssessment,
        policy: &TrustPolicy,
        adapter: &dyn AgentRuntime,
        force: bool,
    ) -> Result<SkillActivationRecord, Box<SkillActivationRecord>> {
        let (can_activate, reason, message) = policy.can_activate(assessment, force);

        if !can_activate {
            return Err(Box::new(SkillActivationRecord {
                work_package_id: work_package_id.to_string(),
                skill_name: skill_name.to_string(),
                status: "BLOCKED".to_string(),
                source_type: source_type.to_string(),
                source_location: source_location.to_string(),
                exact_directory: None,
                install_command: None,
                trust_status: assessment.trust_status.as_str().to_string(),
                content_hash: Some(assessment.content_hash.clone()),
                verified: false,
                instruction_payload: None,
                reason,
                message,
                note: None,
            }));
        }

        // Cryptographic integrity check: loaded in-memory bytes must match assessment hash
        let memory_hash = compute_sha256(raw_content);
        if memory_hash != assessment.content_hash {
            return Err(Box::new(SkillActivationRecord {
                work_package_id: work_package_id.to_string(),
                skill_name: skill_name.to_string(),
                status: "BLOCKED".to_string(),
                source_type: source_type.to_string(),
                source_location: source_location.to_string(),
                exact_directory: None,
                install_command: None,
                trust_status: assessment.trust_status.as_str().to_string(),
                content_hash: Some(memory_hash),
                verified: false,
                instruction_payload: None,
                reason: Some("HASH_MISMATCH".to_string()),
                message: Some("Cryptographic integrity violation: in-memory instruction bytes do not match scanned hash.".to_string()),
                note: None,
            }));
        }

        let payload =
            adapter.format_instruction_payload(work_package_id, component, skill_name, raw_content);

        Ok(SkillActivationRecord {
            work_package_id: work_package_id.to_string(),
            skill_name: skill_name.to_string(),
            status: "ACTIVATED".to_string(),
            source_type: source_type.to_string(),
            source_location: source_location.to_string(),
            exact_directory: None,
            install_command: None,
            trust_status: assessment.trust_status.as_str().to_string(),
            content_hash: Some(memory_hash),
            verified: true,
            instruction_payload: Some(payload),
            reason: None,
            message: None,
            note: None,
        })
    }

    pub fn create_requires_installation(
        work_package_id: &str,
        skill_name: &str,
        source_location: &str,
        exact_directory: Option<String>,
        install_command: Option<String>,
    ) -> SkillActivationRecord {
        SkillActivationRecord {
            work_package_id: work_package_id.to_string(),
            skill_name: skill_name.to_string(),
            status: "REQUIRES_INSTALLATION".to_string(),
            source_type: "remote-registry".to_string(),
            source_location: source_location.to_string(),
            exact_directory,
            install_command,
            trust_status: "UNSCANNED".to_string(),
            content_hash: None,
            verified: false,
            instruction_payload: None,
            reason: None,
            message: None,
            note: Some("Skill must be installed and verified before it can be activated into the live session.".to_string()),
        }
    }

    pub fn create_failure(
        work_package_id: &str,
        skill_name: &str,
        reason: Option<String>,
        message: Option<String>,
    ) -> SkillActivationRecord {
        SkillActivationRecord {
            work_package_id: work_package_id.to_string(),
            skill_name: skill_name.to_string(),
            status: "FAILED".to_string(),
            source_type: "none".to_string(),
            source_location: String::new(),
            exact_directory: None,
            install_command: None,
            trust_status: "UNSCANNED".to_string(),
            content_hash: None,
            verified: false,
            instruction_payload: None,
            reason,
            message,
            note: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runtime_adapters::get_runtime_adapter;

    fn mock_assessment(status: TrustStatus, content_hash: &str) -> SecurityAssessment {
        SecurityAssessment {
            skill_name: "test-skill".to_string(),
            source_repository: "test/repo".to_string(),
            commit_sha: "abc1234".to_string(),
            content_hash: content_hash.to_string(),
            scanned_at: Utc::now(),
            trust_status: status,
            is_approved: status == TrustStatus::Approved,
            is_blocked: status == TrustStatus::Rejected || status == TrustStatus::Quarantined,
            requires_human_approval: status == TrustStatus::Restricted,
            findings: vec![],
            disclaimer: "test".to_string(),
        }
    }

    #[test]
    fn test_approved_skill_activates_with_matching_hash() {
        let content = "# Test Skill Content";
        let hash = compute_sha256(content);
        let assessment = mock_assessment(TrustStatus::Approved, &hash);
        let policy = TrustPolicy::new();
        let adapter = get_runtime_adapter("antigravity");

        let result = ActivationEngine::activate(
            "WP-01",
            "test-comp",
            "test-skill",
            content,
            "local-repository",
            "skills/test",
            &assessment,
            &policy,
            adapter.as_ref(),
            false,
        );

        assert!(result.is_ok(), "Approved skill should activate");
        let record = result.unwrap();
        assert_eq!(record.status, "ACTIVATED");
        assert_eq!(record.trust_status, "APPROVED");
        assert!(record.verified);
        assert!(record.instruction_payload.is_some());
        assert_eq!(record.content_hash, Some(hash));
    }

    #[test]
    fn test_rejected_and_quarantined_skills_are_blocked() {
        let content = "# Malicious Skill";
        let hash = compute_sha256(content);
        let policy = TrustPolicy::new();
        let adapter = get_runtime_adapter("antigravity");

        // Rejected
        let assessment_rejected = mock_assessment(TrustStatus::Rejected, &hash);
        let res_rejected = ActivationEngine::activate(
            "WP-01",
            "comp",
            "bad-skill",
            content,
            "local",
            "skills/bad",
            &assessment_rejected,
            &policy,
            adapter.as_ref(),
            true, // Even with force=true, REJECTED is blocked
        );
        assert!(res_rejected.is_err());
        let rec = res_rejected.unwrap_err();
        assert_eq!(rec.status, "BLOCKED");
        assert_eq!(rec.trust_status, "REJECTED");
        assert!(!rec.verified);

        // Quarantined
        let assessment_quarantined = mock_assessment(TrustStatus::Quarantined, &hash);
        let res_quarantined = ActivationEngine::activate(
            "WP-01",
            "comp",
            "quarantined-skill",
            content,
            "local",
            "skills/bad",
            &assessment_quarantined,
            &policy,
            adapter.as_ref(),
            true,
        );
        assert!(res_quarantined.is_err());
        let rec_q = res_quarantined.unwrap_err();
        assert_eq!(rec_q.status, "BLOCKED");
        assert_eq!(rec_q.trust_status, "QUARANTINED");
    }

    #[test]
    fn test_restricted_skill_blocked_without_force_allowed_with_force() {
        let content = "# Restricted Skill";
        let hash = compute_sha256(content);
        let assessment = mock_assessment(TrustStatus::Restricted, &hash);
        let policy = TrustPolicy::new();
        let adapter = get_runtime_adapter("antigravity");

        // Without force -> blocked
        let res_blocked = ActivationEngine::activate(
            "WP-01",
            "comp",
            "restricted-skill",
            content,
            "local",
            "skills/restr",
            &assessment,
            &policy,
            adapter.as_ref(),
            false,
        );
        assert!(res_blocked.is_err());
        assert_eq!(res_blocked.unwrap_err().status, "BLOCKED");

        // With force -> allowed
        let res_allowed = ActivationEngine::activate(
            "WP-01",
            "comp",
            "restricted-skill",
            content,
            "local",
            "skills/restr",
            &assessment,
            &policy,
            adapter.as_ref(),
            true,
        );
        assert!(res_allowed.is_ok());
        assert_eq!(res_allowed.unwrap().status, "ACTIVATED");
    }

    #[test]
    fn test_hash_mismatch_blocks_activation() {
        let content = "# Original Content";
        let tampered_content = "# Tampered Content In Memory";
        let assessment_hash = compute_sha256(content);
        let assessment = mock_assessment(TrustStatus::Approved, &assessment_hash);
        let policy = TrustPolicy::new();
        let adapter = get_runtime_adapter("antigravity");

        let res = ActivationEngine::activate(
            "WP-01",
            "comp",
            "tampered-skill",
            tampered_content,
            "local",
            "skills/test",
            &assessment,
            &policy,
            adapter.as_ref(),
            false,
        );

        assert!(res.is_err());
        let err_rec = res.unwrap_err();
        assert_eq!(err_rec.status, "BLOCKED");
        assert_eq!(err_rec.reason, Some("HASH_MISMATCH".to_string()));
        assert!(!err_rec.verified);
    }

    #[test]
    fn test_activation_report_markdown_generation() {
        let act = SkillActivationRecord {
            work_package_id: "WP-01".to_string(),
            skill_name: "test-skill".to_string(),
            status: "ACTIVATED".to_string(),
            source_type: "local-repository".to_string(),
            source_location: "skills/test".to_string(),
            exact_directory: None,
            install_command: None,
            trust_status: "APPROVED".to_string(),
            content_hash: Some("123456".to_string()),
            verified: true,
            instruction_payload: None,
            reason: None,
            message: None,
            note: None,
        };

        let report = ActivationReport::new(
            "TASK-100",
            "antigravity",
            "single-agent",
            vec![act],
            vec![],
            None,
        );

        assert_eq!(report.total_requested, 1);
        assert_eq!(report.activated_count, 1);
        assert_eq!(report.failed_count, 0);

        let md = report.to_markdown();
        assert!(md.contains("# Skill Activation Report"));
        assert!(md.contains("TASK-100"));
        assert!(md.contains("test-skill"));
        assert!(md.contains("Successfully Activated: 1"));
    }
}
