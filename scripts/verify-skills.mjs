#!/usr/bin/env node
import path from "node:path";
import { fileURLToPath } from "node:url";
import { InstalledTracker } from "../src/lifecycle/installed-tracker.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const tracker = new InstalledTracker({ rootDir: ROOT });
const isJson = process.argv.includes("--json");

const report = await tracker.verifySkills();

if (isJson) {
  process.stdout.write(JSON.stringify(report, null, 2) + "\n");
  process.exit(report.modified > 0 || report.missing > 0 ? 1 : 0);
}

process.stdout.write(`=== INSTALLED SKILLS VERIFICATION REPORT ===\n`);
process.stdout.write(`Total: ${report.total} | Intact: ${report.intact} | Modified: ${report.modified} | Missing/Corrupt: ${report.missing}\n\n`);

for (const d of report.details) {
  process.stdout.write(`- [${d.status}] ${d.name} (${d.installationPath})\n`);
  if (d.status === "MODIFIED") {
    process.stdout.write(`  Warning: Hash mismatch! Recorded: ${d.recordedHash?.slice(0, 12)} vs Actual: ${d.currentHash?.slice(0, 12)}\n`);
  } else if (d.status === "MISSING_OR_CORRUPT") {
    process.stderr.write(`  Error: ${d.error}\n`);
  }
}

if (report.modified > 0 || report.missing > 0) {
  process.exitCode = 1;
}
