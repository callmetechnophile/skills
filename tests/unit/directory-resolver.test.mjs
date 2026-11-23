import test from "node:test";
import assert from "node:assert/strict";
import { DirectoryResolver } from "../../src/discovery/directory-resolver.mjs";

const mockTree = [
  { path: "README.md", type: "blob" },
  { path: "skills/core/sort-the-issue/SKILL.md", type: "blob" },
  { path: "skills/core/automatic-skill-router/SKILL.md", type: "blob" },
  { path: "skills/database/postgres-query-tuning/SKILL.md", type: "blob" },
  { path: "skills/v1/duplicate-name/SKILL.md", type: "blob" },
  { path: "skills/v2/duplicate-name/SKILL.md", type: "blob" }
];

test("DirectoryResolver finds exact skill directory from tree", async () => {
  const resolver = new DirectoryResolver();
  const res = await resolver.resolveSkillDirectory("callmetechnophile/skills", "postgres-query-tuning", {
    treeFixture: mockTree,
    agent: "antigravity"
  });

  assert.equal(res.success, true);
  assert.equal(res.exactDirectory, "skills/database/postgres-query-tuning");
  assert.equal(res.skillMdPath, "skills/database/postgres-query-tuning/SKILL.md");
  assert.equal(
    res.installCommand,
    'npx -y skills add callmetechnophile/skills --skill skills/database/postgres-query-tuning --agent antigravity -y'
  );
});

test("DirectoryResolver rejects ambiguous matches", async () => {
  const resolver = new DirectoryResolver();
  const res = await resolver.resolveSkillDirectory("callmetechnophile/skills", "duplicate-name", {
    treeFixture: mockTree
  });

  assert.equal(res.success, false);
  assert.equal(res.error, "AMBIGUOUS_MATCH");
  assert.equal(res.candidates.length, 2);
  assert.ok(res.candidates.includes("skills/v1/duplicate-name"));
  assert.ok(res.candidates.includes("skills/v2/duplicate-name"));
});

test("DirectoryResolver rejects missing skills and lists available directories", async () => {
  const resolver = new DirectoryResolver();
  const res = await resolver.resolveSkillDirectory("callmetechnophile/skills", "non-existent-skill", {
    treeFixture: mockTree
  });

  assert.equal(res.success, false);
  assert.equal(res.error, "SKILL_NOT_FOUND");
  assert.ok(res.availableSkills.length > 0);
});
