/**
 * Runtime adapters for Antigravity, Claude Code, and Codex environments.
 */

import { AgentRuntimeAdapter } from "./runtime-base.mjs";

export class AntigravityAdapter extends AgentRuntimeAdapter {
  constructor(config = {}) {
    super("antigravity", {
      supports_skills: true,
      skill_path: ".agents/skills",
      ...config
    });
  }

  async checkSubagentSupport() {
    // Check if running inside Antigravity agent process with invoke_subagent or similar
    const hasAgentEnv = Boolean(process.env.ANTIGRAVITY_SUBAGENTS || process.env.AGY_MULTIAGENT);
    return {
      supported: hasAgentEnv,
      mode: hasAgentEnv ? "multi-agent-parallel" : "sequential-fallback",
      reason: hasAgentEnv
        ? "Antigravity sub-agent runtime environment detected."
        : "Sub-agent spawning API not present in current CLI process; executing via honest sequential fallback."
    };
  }

  formatInstructionPayload(workPackage, skillContent, metadata = {}) {
    const base = super.formatInstructionPayload(workPackage, skillContent, metadata);
    return {
      ...base,
      antigravityContract: {
        mandatory_order: "sort-the-issue -> automatic-skill-router -> bounded execution",
        sidecar_integration: "available",
        file_ownership_boundary: [workPackage.component]
      }
    };
  }
}

export class ClaudeCodeAdapter extends AgentRuntimeAdapter {
  constructor(config = {}) {
    super("claude-code", {
      supports_skills: true,
      skill_path: ".claude/skills",
      ...config
    });
  }

  async checkSubagentSupport() {
    const hasClaudeSpawn = Boolean(process.env.CLAUDE_SUBAGENTS);
    return {
      supported: hasClaudeSpawn,
      mode: hasClaudeSpawn ? "multi-agent-parallel" : "sequential-fallback",
      reason: hasClaudeSpawn
        ? "Claude Code sub-agent support active."
        : "Single session detected; sequential fallback applied."
    };
  }
}

export class CodexAdapter extends AgentRuntimeAdapter {
  constructor(config = {}) {
    super("codex", {
      supports_skills: true,
      skill_path: ".agents/skills",
      ...config
    });
  }

  async checkSubagentSupport() {
    return {
      supported: false,
      mode: "sequential-fallback",
      reason: "Codex runtime executes sequentially with persistent session state."
    };
  }
}

export function getRuntimeAdapter(agentName = "antigravity", agentConfigs = {}) {
  const norm = String(agentName || "antigravity").toLowerCase().trim();
  const cfg = agentConfigs[norm] || {};
  switch (norm) {
    case "antigravity":
    case "antigravity-cli":
      return new AntigravityAdapter(cfg);
    case "claude-code":
      return new ClaudeCodeAdapter(cfg);
    case "codex":
      return new CodexAdapter(cfg);
    default:
      return new AgentRuntimeAdapter(norm, cfg);
  }
}
