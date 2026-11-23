import test from "node:test";
import assert from "node:assert/strict";
import { normalizeTask } from "../../src/intelligence/task-normalizer.mjs";
import { ComplexityAnalyzer } from "../../src/intelligence/complexity-analyzer.mjs";
import { IssueSorter } from "../../src/intelligence/issue-sorter.mjs";
import { validateTriageSchema } from "../../src/intelligence/schemas/task-schema.mjs";

test("TaskNormalizer extracts intent, domains, and normalizes text", () => {
  const norm = normalizeTask("  Fix the failing   authentication tests in Postgres  ");
  assert.equal(norm.normalizedTask, "Fix the failing authentication tests in Postgres");
  assert.equal(norm.intent, "fix");
  assert.ok(norm.domains.includes("auth"));
  assert.ok(norm.domains.includes("database"));
  assert.ok(norm.domains.includes("testing"));
});

test("ComplexityAnalyzer correctly scores low vs high complexity tasks", () => {
  const analyzer = new ComplexityAnalyzer();

  const low = analyzer.scoreHeuristics("Fix a typo in README.md", ["documentation"]);
  assert.equal(low.band, "LOW");
  assert.ok(low.total_score <= 3);

  const high = analyzer.scoreHeuristics(
    "Build a full-stack platform with frontend, backend API, database migration, oauth auth, and production CI/CD deployment",
    ["frontend", "backend", "database", "auth", "devops"]
  );
  assert.equal(high.band, "HIGH");
  assert.ok(high.total_score >= 7);
  assert.equal(high.scope_breadth, 2);
  assert.equal(high.technical_coupling, 2);
  assert.equal(high.parallelism, 2);
});

test("IssueSorter decomposes high-complexity full-stack tasks into component-bounded work packages", () => {
  const sorter = new IssueSorter();
  const triage = sorter.sortSync("Build an application with frontend, backend, database, authentication and deployment");

  assert.equal(triage.complexity.band, "HIGH");
  assert.equal(triage.workflow, "multi-agent");
  assert.ok(triage.work_packages.length >= 3, "must decompose into multiple component packages");

  const components = triage.work_packages.map(wp => wp.component);
  assert.ok(components.includes("database"), "database package present");
  assert.ok(components.includes("backend"), "backend package present");
  assert.ok(components.includes("frontend"), "frontend package present");

  // Check dependencies: backend depends on database
  const backendPkg = triage.work_packages.find(wp => wp.component === "backend");
  assert.ok(backendPkg.dependencies.length > 0, "backend package must depend on database");

  // Schema validation
  const validation = validateTriageSchema(triage);
  assert.equal(validation.valid, true, `Schema validation errors: ${validation.errors.join(", ")}`);
});
