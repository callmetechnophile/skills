import test from "node:test";
import assert from "node:assert/strict";
import { sortIssue, routeTask } from "../src/router.mjs";

test("sort-the-issue classifies a bug before skill discovery", () => {
  const result=sortIssue("Fix the failing authentication tests");
  assert.equal(result.class.includes("bug"), true);
  assert.equal(result.complexity.band, "HIGH");
});

test("low complexity uses single-agent sequential selection", () => {
  const result=routeTask("Fix a typo in README");
  assert.equal(result.mandatory_first_skill, "sort-the-issue");
  assert.equal(result.routing.workflow, "single-agent");
  assert.match(result.routing.selection_policy, /one specialist skill at a time/);
});

test("forced multi-agent includes honest sequential fallback", () => {
  const result=routeTask("Build an independent multi-agent platform", "multi");
  assert.equal(result.routing.workflow, "multi-agent");
  assert.match(result.routing.fallback, /sequentially/);
});
