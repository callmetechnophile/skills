import test from "node:test";
import assert from "node:assert/strict";
import { rankSkills } from "../src/registry.mjs";

const sample = [
  { name: "Postgres Query Tuning", description: "Index and query planning", tags: ["sql", "database"], category: "development", stars: 120, verified: true },
  { name: "React UI Patterns", description: "Reusable frontend components", tags: ["react"], category: "frontend", stars: 500, verified: true }
];

test("registry ranking prioritizes semantic token overlap over stars", () => {
  const ranked = rankSkills(sample, "database query tuning");
  assert.equal(ranked[0].name, "Postgres Query Tuning");
});

test("registry ranking returns no entries for empty query", () => {
  assert.equal(rankSkills(sample, " ").length, 0);
});
