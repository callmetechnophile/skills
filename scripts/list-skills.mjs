#!/usr/bin/env node
import path from "node:path";
import { fileURLToPath } from "node:url";
import { InstalledTracker } from "../src/lifecycle/installed-tracker.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const tracker = new InstalledTracker({ rootDir: ROOT });
const isJson = process.argv.includes("--json");

const list = await tracker.listSkills();

if (isJson) {
  process.stdout.write(JSON.stringify({ total: list.length, skills: list }, null, 2) + "\n");
  process.exit(0);
}

if (!list.length) {
  process.stdout.write("No skills currently recorded in .installed-skills.json.\n");
  process.exit(0);
}

process.stdout.write(`=== INSTALLED SKILLS (${list.length}) ===\n`);
for (const [i, s] of list.entries()) {
  process.stdout.write(`${i + 1}. ${s.name} [Version: ${s.version || "1.0.0"}]\n`);
  process.stdout.write(`   Source: ${s.source} (Commit: ${s.commitSha || "main"})\n`);
  process.stdout.write(`   Path: ${s.installationPath}\n`);
  process.stdout.write(`   Trust: ${s.trustStatus} | Hash: ${s.contentHash ? s.contentHash.slice(0, 12) + "..." : "none"}\n\n`);
}
