import test from "node:test";
import assert from "node:assert/strict";
import { DependencyGraph } from "../../src/orchestration/dependency-graph.mjs";
import { getRuntimeAdapter } from "../../src/orchestration/runtime-adapters/adapters.mjs";

const samplePackages = [
  { id: "WP-1", title: "Database Layer", dependencies: [] },
  { id: "WP-2", title: "Auth Subsystem", dependencies: ["WP-1"] },
  { id: "WP-3", title: "API Controllers", dependencies: ["WP-1"] },
  { id: "WP-4", title: "Frontend Client", dependencies: ["WP-2", "WP-3"] }
];

test("DependencyGraph computes topological schedule and parallel waves", () => {
  const graph = new DependencyGraph(samplePackages);
  const scheduled = graph.schedule();

  assert.equal(scheduled.parallelWaves.length, 3);
  // Wave 1: WP-1 (0 deps)
  assert.equal(scheduled.parallelWaves[0].length, 1);
  assert.equal(scheduled.parallelWaves[0][0].id, "WP-1");

  // Wave 2: WP-2 and WP-3 can run in parallel
  assert.equal(scheduled.parallelWaves[1].length, 2);
  const wave2Ids = scheduled.parallelWaves[1].map(p => p.id);
  assert.ok(wave2Ids.includes("WP-2"));
  assert.ok(wave2Ids.includes("WP-3"));

  // Wave 3: WP-4
  assert.equal(scheduled.parallelWaves[2].length, 1);
  assert.equal(scheduled.parallelWaves[2][0].id, "WP-4");
});

test("DependencyGraph detects circular dependencies and throws error", () => {
  const cyclic = [
    { id: "WP-A", dependencies: ["WP-B"] },
    { id: "WP-B", dependencies: ["WP-A"] }
  ];
  const graph = new DependencyGraph(cyclic);
  assert.throws(() => graph.schedule(), /Circular dependency detected/);
});

test("DependencyGraph creates honest sequential fallback (Mode C) when subagents are unavailable", () => {
  const graph = new DependencyGraph(samplePackages);
  const plan = graph.createExecutionPlan("multi-agent", { supported: false, reason: "No subagents" });

  assert.equal(plan.selectedMode, "Mode C: Sequential fallback");
  assert.equal(plan.parallelEnabled, false);
  assert.equal(plan.linearOrder.length, 4);
  assert.ok(plan.downgradeNotice.includes("No subagents"));
});

test("Runtime adapters provide agent-specific configurations", async () => {
  const agy = getRuntimeAdapter("antigravity");
  assert.equal(agy.name, "antigravity");
  assert.equal(agy.getSkillDirectory(), ".agents/skills");

  const claude = getRuntimeAdapter("claude-code");
  assert.equal(claude.getSkillDirectory(), ".claude/skills");

  const codex = getRuntimeAdapter("codex");
  const codexSupport = await codex.checkSubagentSupport();
  assert.equal(codexSupport.supported, false);
  assert.equal(codexSupport.mode, "sequential-fallback");
});
