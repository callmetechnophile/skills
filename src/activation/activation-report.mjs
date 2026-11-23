/**
 * Activation report generator structuring activation status, verification hashes,
 * runtime targets, and explicit failure reasons.
 */

export class ActivationReport {
  constructor(data = {}) {
    this.taskId = data.taskId || "unknown";
    this.generatedAt = new Date().toISOString();
    this.agent = data.agent || "antigravity";
    this.workflowMode = data.workflowMode || "single-agent";
    this.activations = data.activations || [];
    this.failures = data.failures || [];
    this.executionPlan = data.executionPlan || null;
  }

  toJSON() {
    return {
      taskId: this.taskId,
      generatedAt: this.generatedAt,
      agent: this.agent,
      workflowMode: this.workflowMode,
      totalRequested: this.activations.length + this.failures.length,
      activatedCount: this.activations.filter(a => a.status === "ACTIVATED").length,
      failedCount: this.failures.length,
      activations: this.activations,
      failures: this.failures,
      executionPlan: this.executionPlan
    };
  }

  toMarkdown() {
    const lines = [
      `# Skill Activation Report`,
      `**Task ID:** \`${this.taskId}\``,
      `**Timestamp:** ${this.generatedAt}`,
      `**Target Agent:** ${this.agent}`,
      `**Workflow Mode:** ${this.workflowMode}`,
      ``,
      `## Summary`,
      `- Total Skills Processed: ${this.activations.length + this.failures.length}`,
      `- Successfully Activated: ${this.activations.filter(a => a.status === "ACTIVATED").length}`,
      `- Blocked / Failed: ${this.failures.length}`,
      ``,
      `## Activation Records`
    ];

    for (const act of this.activations) {
      lines.push(`### [${act.status}] ${act.skillName}`);
      lines.push(`- **Work Package:** \`${act.workPackageId}\``);
      lines.push(`- **Source:** ${act.sourceType} (\`${act.sourceLocation}\`)`);
      lines.push(`- **Trust Status:** ${act.trustStatus}`);
      lines.push(`- **Content SHA-256:** \`${act.contentHash || "N/A"}\``);
      lines.push(`- **Verification:** ${act.verified ? "Passed" : "Failed"}`);
      lines.push(``);
    }

    if (this.failures.length > 0) {
      lines.push(`## Blocked & Failed Skills`);
      for (const fail of this.failures) {
        lines.push(`### [BLOCKED] ${fail.skillName}`);
        lines.push(`- **Work Package:** \`${fail.workPackageId}\``);
        lines.push(`- **Reason Code:** \`${fail.reason}\``);
        lines.push(`- **Details:** ${fail.message}`);
        lines.push(``);
      }
    }

    return lines.join("\n");
  }
}
