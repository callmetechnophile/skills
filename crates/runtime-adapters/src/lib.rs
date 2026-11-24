use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Adapter error: {0}")]
    Adapter(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentCapability {
    pub supported: bool,
    pub mode: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionPayload {
    #[serde(rename = "workPackageId")]
    pub work_package_id: String,
    #[serde(rename = "skillName")]
    pub skill_name: String,
    pub component: String,
    #[serde(rename = "instructionBody")]
    pub instruction_body: String,
    #[serde(
        rename = "antigravityContract",
        skip_serializing_if = "Option::is_none"
    )]
    pub antigravity_contract: Option<AntigravityContract>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntigravityContract {
    pub mandatory_order: String,
    pub sidecar_integration: String,
    pub file_ownership_boundary: Vec<String>,
}

pub trait AgentRuntime: Send + Sync {
    fn agent_name(&self) -> &str;
    fn check_subagent_support(&self) -> SubagentCapability;
    fn format_instruction_payload(
        &self,
        work_package_id: &str,
        component: &str,
        skill_name: &str,
        skill_content: &str,
    ) -> InstructionPayload;
}

pub struct AntigravityAdapter;

impl AgentRuntime for AntigravityAdapter {
    fn agent_name(&self) -> &str {
        "antigravity"
    }

    fn check_subagent_support(&self) -> SubagentCapability {
        let has_env = std::env::var("ANTIGRAVITY_SUBAGENTS").is_ok()
            || std::env::var("AGY_MULTIAGENT").is_ok();
        SubagentCapability {
            supported: has_env,
            mode: if has_env {
                "multi-agent-parallel".to_string()
            } else {
                "sequential-fallback".to_string()
            },
            reason: if has_env {
                "Antigravity sub-agent runtime environment detected.".to_string()
            } else {
                "Sub-agent spawning API not present in current CLI process; executing via honest sequential fallback.".to_string()
            },
        }
    }

    fn format_instruction_payload(
        &self,
        work_package_id: &str,
        component: &str,
        skill_name: &str,
        skill_content: &str,
    ) -> InstructionPayload {
        InstructionPayload {
            work_package_id: work_package_id.to_string(),
            skill_name: skill_name.to_string(),
            component: component.to_string(),
            instruction_body: skill_content.to_string(),
            antigravity_contract: Some(AntigravityContract {
                mandatory_order: "sort-the-issue -> automatic-skill-router -> bounded execution"
                    .to_string(),
                sidecar_integration: "available".to_string(),
                file_ownership_boundary: vec![component.to_string()],
            }),
        }
    }
}

pub struct ClaudeCodeAdapter;

impl AgentRuntime for ClaudeCodeAdapter {
    fn agent_name(&self) -> &str {
        "claude-code"
    }

    fn check_subagent_support(&self) -> SubagentCapability {
        let has_env = std::env::var("CLAUDE_SUBAGENTS").is_ok();
        SubagentCapability {
            supported: has_env,
            mode: if has_env {
                "multi-agent-parallel".to_string()
            } else {
                "sequential-fallback".to_string()
            },
            reason: if has_env {
                "Claude Code sub-agent support active.".to_string()
            } else {
                "Single session detected; sequential fallback applied.".to_string()
            },
        }
    }

    fn format_instruction_payload(
        &self,
        work_package_id: &str,
        component: &str,
        skill_name: &str,
        skill_content: &str,
    ) -> InstructionPayload {
        InstructionPayload {
            work_package_id: work_package_id.to_string(),
            skill_name: skill_name.to_string(),
            component: component.to_string(),
            instruction_body: skill_content.to_string(),
            antigravity_contract: None,
        }
    }
}

pub struct CodexAdapter;

impl AgentRuntime for CodexAdapter {
    fn agent_name(&self) -> &str {
        "codex"
    }

    fn check_subagent_support(&self) -> SubagentCapability {
        SubagentCapability {
            supported: false,
            mode: "sequential-fallback".to_string(),
            reason: "Codex runtime executes sequentially with persistent session state."
                .to_string(),
        }
    }

    fn format_instruction_payload(
        &self,
        work_package_id: &str,
        component: &str,
        skill_name: &str,
        skill_content: &str,
    ) -> InstructionPayload {
        InstructionPayload {
            work_package_id: work_package_id.to_string(),
            skill_name: skill_name.to_string(),
            component: component.to_string(),
            instruction_body: skill_content.to_string(),
            antigravity_contract: None,
        }
    }
}

pub fn get_runtime_adapter(agent_name: &str) -> Box<dyn AgentRuntime> {
    match agent_name.to_lowercase().trim() {
        "antigravity" | "antigravity-cli" => Box::new(AntigravityAdapter),
        "claude-code" | "claude" => Box::new(ClaudeCodeAdapter),
        "codex" => Box::new(CodexAdapter),
        _ => Box::new(AntigravityAdapter),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_adapter_selection() {
        let adapter = get_runtime_adapter("antigravity");
        assert_eq!(adapter.agent_name(), "antigravity");
        let payload = adapter.format_instruction_payload("WP-1", "core", "test-skill", "content");
        assert!(payload.antigravity_contract.is_some());
    }
}
