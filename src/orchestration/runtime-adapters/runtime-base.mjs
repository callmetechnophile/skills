/**
 * Base adapter class for agent execution environments.
 */

export class AgentRuntimeAdapter {
  constructor(name, config = {}) {
    this.name = name;
    this.config = config;
    this.supportsSkills = config.supports_skills ?? true;
    this.skillPath = config.skill_path || ".agents/skills";
    this.subagentsPolicy = config.subagents || "runtime-dependent";
  }

  /**
   * Determine if the current runtime environment actually exposes
   * a functional sub-agent spawning API in the live session.
   */
  async checkSubagentSupport() {
    // In node/CLI standalone execution, sub-agents are externally managed unless runtime bridge exists
    return {
      supported: false,
      mode: "sequential-fallback",
      reason: "Live multi-agent spawning is externally managed by host runtime."
    };
  }

  getSkillDirectory() {
    return this.skillPath;
  }

  formatInstructionPayload(workPackage, skillContent, metadata = {}) {
    return {
      runtime: this.name,
      workPackageId: workPackage.id,
      title: workPackage.title,
      objective: workPackage.objective,
      component: workPackage.component,
      skillName: metadata.name || "custom-skill",
      instructions: skillContent,
      executionContract: "Execute bounded package steps, run verification tests, and produce structured handoff artifact."
    };
  }
}
