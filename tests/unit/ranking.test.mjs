import test from "node:test";
import assert from "node:assert/strict";
import { HybridRanker } from "../../src/ranking/hybrid-ranker.mjs";

const candidates = [
  {
    name: "Postgres Migration Specialist",
    description: "Production database schema migrations and indexing",
    tags: ["database", "postgres", "sql"],
    category: "database",
    stars: 50,
    verified: true,
    security_status: "APPROVED"
  },
  {
    name: "Random Popular Project",
    description: "Unrelated utility with many stars",
    tags: ["utility"],
    category: "tools",
    stars: 20000,
    verified: false,
    security_status: "APPROVED"
  },
  {
    name: "Insecure Postgres Tool",
    description: "Postgres tool with dangerous exfiltration status",
    tags: ["database", "postgres"],
    category: "database",
    stars: 500,
    verified: false,
    security_status: "REJECTED"
  }
];

test("HybridRanker prioritizes relevant capability over high star count", async () => {
  const ranker = new HybridRanker();
  const res = await ranker.rank(candidates, "postgres database migration");

  assert.ok(res.selected, "must select top candidate");
  assert.equal(res.selected.skill.name, "Postgres Migration Specialist");
  assert.ok(
    res.selected.relevanceScore > (res.alternatives[0]?.relevanceScore ?? 0),
    "relevant skill must outrank random popular project"
  );
});

test("HybridRanker blocks insecure rejected skills and provides rejection reason", async () => {
  const ranker = new HybridRanker();
  const res = await ranker.rank(candidates, "postgres");

  const rejectedNames = res.rejections.map(r => r.skill.name);
  assert.ok(rejectedNames.includes("Insecure Postgres Tool"));
  const rej = res.rejections.find(r => r.skill.name === "Insecure Postgres Tool");
  assert.equal(rej.reason, "SECURITY_REJECTION");
});

test("HybridRanker returns normalized scores with explainable 5-factor breakdown", async () => {
  const ranker = new HybridRanker();
  const res = await ranker.rank(candidates, "database migration");

  const bd = res.selected.scoreBreakdown;
  assert.ok(bd.task_relevance !== undefined);
  assert.ok(bd.capability_completeness !== undefined);
  assert.ok(bd.provenance_maintenance !== undefined);
  assert.ok(bd.security_trust !== undefined);
  assert.ok(bd.runtime_compatibility !== undefined);
  assert.ok(res.selected.relevanceScore <= 1.0 && res.selected.relevanceScore > 0);
});
