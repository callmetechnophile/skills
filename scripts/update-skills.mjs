#!/usr/bin/env node
import path from "node:path";
import { fileURLToPath } from "node:url";
import { InstalledTracker } from "../src/lifecycle/installed-tracker.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const tracker = new InstalledTracker({ rootDir: ROOT });
const isJson = process.argv.includes("--json");

const updates = await tracker.checkForUpdates();

if (isJson) {
  process.stdout.write(JSON.stringify({ total: updates.length, updates }, null, 2) + "\n");
  process.exit(0);
}

if (!updates.length) {
  process.stdout.write("No installed skills tracked to update.\n");
  process.exit(0);
}

process.stdout.write(`=== INSTALLED SKILLS UPDATE STATUS ===\n`);
for (const u of updates) {
  process.stdout.write(`- ${u.name} (Source: ${u.source}, Version: ${u.currentVersion}) -> ${u.status}\n`);
}
