use runtime_adapters::InstructionPayload;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionHandoff {
    pub package_id: String,
    pub title: String,
    pub payload: InstructionPayload,
}

pub struct ExecutionEngine;

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self
    }
}

impl ExecutionEngine {
    pub fn format_handoff(
        package_id: &str,
        title: &str,
        payload: InstructionPayload,
    ) -> ExecutionHandoff {
        ExecutionHandoff {
            package_id: package_id.to_string(),
            title: title.to_string(),
            payload,
        }
    }
}
