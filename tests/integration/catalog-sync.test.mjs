import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { SkillsDirectoryRegistry } from "../../src/discovery/skillsdirectory-registry.mjs";

test("SkillsDirectoryRegistry synchronizes catalog with pagination and resume", async () => {
  const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), "sync-test-"));
  const indexPath = path.join(tmpDir, "catalog", "skillsdirectory-index.json");

  // Mock API returning 2 pages
  const mockFetch = async (urlStr) => {
    const url = new URL(urlStr);
    const offset = Number(url.searchParams.get("offset") || "0");
    const limit = Number(url.searchParams.get("limit") || "2");

    if (offset === 0) {
      return {
        ok: true,
        json: async () => ({
          pagination: { total: 3, hasMore: true },
          skills: [
            { slug: "skill-1", name: "Skill One", repository: "user/skill-1", description: "First" },
            { slug: "skill-2", name: "Skill Two", repository: "user/skill-2", description: "Second" }
          ]
        })
      };
    } else if (offset === 2) {
      return {
        ok: true,
        json: async () => ({
          pagination: { total: 3, hasMore: false },
          skills: [
            { slug: "skill-3", name: "Skill Three", repository: "user/skill-3", description: "Third" }
          ]
        })
      };
    }

    return {
      ok: true,
      json: async () => ({ pagination: { total: 3, hasMore: false }, skills: [] })
    };
  };

  const registry = new SkillsDirectoryRegistry({
    rootDir: tmpDir,
    indexPath,
    fetchFn: mockFetch
  });

  // Run partial sync (maxPages: 1)
  const part1 = await registry.syncCatalog({ pageSize: 2, maxPages: 1 });
  assert.equal(part1.syncedPages, 1);
  assert.equal(part1.totalIndexed, 2);
  assert.equal(part1.nextOffset, 2);
  assert.equal(part1.complete, false);

  // Resume sync
  const part2 = await registry.syncCatalog({ pageSize: 2, maxPages: 5 });
  assert.equal(part2.syncedPages, 1);
  assert.equal(part2.totalIndexed, 3);
  assert.equal(part2.complete, true);

  // Read saved index
  const saved = JSON.parse(await fs.readFile(indexPath, "utf8"));
  assert.equal(saved.skills.length, 3);
  assert.equal(saved.complete, true);

  // Clean up
  await fs.rm(tmpDir, { recursive: true, force: true });
});
