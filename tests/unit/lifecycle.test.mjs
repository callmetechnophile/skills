import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { InstalledTracker } from "../../src/lifecycle/installed-tracker.mjs";
import { computeStringHash } from "../../src/security/integrity.mjs";

test("InstalledTracker persists records and prevents silent overwrite on content mismatch", async () => {
  const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), "tracker-test-"));
  const ledgerPath = path.join(tmpDir, ".installed-skills.json");
  const tracker = new InstalledTracker({ rootDir: tmpDir, ledgerPath });

  // 1. Create a dummy installed skill file
  const skillDir = path.join(tmpDir, ".agents", "skills", "test-skill");
  await fs.mkdir(skillDir, { recursive: true });
  const initialContent = "# Test Skill v1.0\nInitial instructions.";
  const initialHash = computeStringHash(initialContent);
  await fs.writeFile(path.join(skillDir, "SKILL.md"), initialContent, "utf8");

  // 2. Record initial installation
  await tracker.recordInstallation({
    skillId: "owner/repo:test-skill",
    name: "test-skill",
    source: "owner/repo",
    installationPath: ".agents/skills/test-skill",
    contentHash: initialHash,
    version: "1.0.0"
  });

  const list = await tracker.listSkills();
  assert.equal(list.length, 1);
  assert.equal(list[0].name, "test-skill");

  // 3. User modifies skill file locally
  await fs.writeFile(path.join(skillDir, "SKILL.md"), "# Test Skill\nUser modified locally!", "utf8");

  // 4. Attempting to record installation without force must throw error
  await assert.rejects(
    () => tracker.recordInstallation({
      skillId: "owner/repo:test-skill",
      name: "test-skill",
      source: "owner/repo",
      installationPath: ".agents/skills/test-skill",
      version: "1.1.0"
    }),
    /content hash mismatch.*Overwrite refused/
  );

  // 5. VerifySkills detects modification
  const verifyReport = await tracker.verifySkills();
  assert.equal(verifyReport.modified, 1);
  assert.equal(verifyReport.details[0].status, "MODIFIED");

  // Clean up
  await fs.rm(tmpDir, { recursive: true, force: true });
});
