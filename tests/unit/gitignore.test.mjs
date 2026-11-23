import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";

test(".gitignore is properly formatted with actual newlines and required rules", async () => {
  const content = await fs.readFile(path.resolve(".gitignore"), "utf8");
  assert.equal(content.includes("\\n"), false, ".gitignore must not contain literal '\\n' sequences");
  const lines = content.split(/\r?\n/).map(l => l.trim()).filter(Boolean);
  assert.ok(lines.includes("node_modules/"), "must ignore node_modules/");
  assert.ok(lines.includes(".env"), "must ignore .env");
  assert.ok(lines.includes(".env.*"), "must ignore .env.*");
  assert.ok(lines.includes("!.env.example"), "must retain !.env.example");
  assert.ok(lines.includes("coverage/"), "must ignore coverage/");
  assert.ok(lines.includes(".DS_Store"), "must ignore .DS_Store");
  assert.ok(lines.length >= 6, "should have multiple distinct lines");
});
