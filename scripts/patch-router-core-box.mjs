/**
 * Patch router-core to deref Box<SkillActivationRecord> from ActivationEngine::activate Err branch.
 * Also unbox the SkillActivationRecord literals pushed to failures Vec.
 */
import { readFileSync, writeFileSync } from "node:fs";

function toWinPath(url) {
  return new URL(url, import.meta.url).pathname.replace(/^\/([A-Z]:)/, "$1");
}

const p = toWinPath("../crates/router-core/src/lib.rs");
let src = readFileSync(p, "utf8");

// Fix 1: deref the Box from Err(record) match arm
src = src.replace(
  "Err(record) => failures.push(record),",
  "Err(record) => failures.push(*record),"
);

// Verify the fix landed
if (!src.includes("failures.push(*record)")) {
  console.error("❌ Fix 1 (deref) did not apply — anchor not found");
  process.exit(1);
}

writeFileSync(p, src);
console.log("✅ router-core: Err(record) => failures.push(*record)");

// Quick sanity: count remaining Box push issues
const remaining = (src.match(/failures\.push\(record\)/g) || []).length;
if (remaining > 0) {
  console.warn(`⚠  Still ${remaining} underef'd push(record) calls remaining`);
}

console.log("\n✅ router-core patch complete.");
