/**
 * One-time patch: guard against undefined/null repository in resolveSkillDirectory
 * and add pre-flight check in activation-planner before calling it.
 */
import { readFileSync, writeFileSync } from "node:fs";

// --- Fix 1: directory-resolver.mjs ---
const drPath = new URL("../src/discovery/directory-resolver.mjs", import.meta.url).pathname.replace(/^\/([A-Z]:)/, "$1");
let dr = readFileSync(drPath, "utf8");

const drOld = "async resolveSkillDirectory(repository, targetSkill, options = {}) {";
const drNew = `async resolveSkillDirectory(repository, targetSkill, options = {}) {
    if (!repository || typeof repository !== "string") {
      return {
        success: false,
        error: "INVALID_REPOSITORY",
        message: \`Repository is missing or not a string (got \${JSON.stringify(repository)}). Cannot resolve skill directory.\`
      };
    }`;

if (dr.includes(drOld)) {
  // Only patch if the guard doesn't exist yet
  if (!dr.includes("Repository is missing or not a string")) {
    dr = dr.replace(drOld, drNew);
    writeFileSync(drPath, dr);
    console.log("✅ directory-resolver.mjs: added undefined-repository guard");
  } else {
    console.log("⏭  directory-resolver.mjs: guard already present, skipping");
  }
} else {
  console.error("❌ directory-resolver.mjs: anchor not found — manual patch needed");
  process.exit(1);
}

// --- Fix 2: activation-planner.mjs ---
const apPath = new URL("../src/activation/activation-planner.mjs", import.meta.url).pathname.replace(/^\/([A-Z]:)/, "$1");
let ap = readFileSync(apPath, "utf8");

// Find the remote-skill branch and insert guard before resolveSkillDirectory call
const apOld = "        // Stage 3: Remote skill requiring installation\r\n        // Check exact directory resolution before recommending install\r\n        const resolution = await this.directoryResolver.resolveSkillDirectory(";
const apOldLF = "        // Stage 3: Remote skill requiring installation\n        // Check exact directory resolution before recommending install\n        const resolution = await this.directoryResolver.resolveSkillDirectory(";

const apNew = `        // Stage 3: Remote skill requiring installation
        // Guard: skill.repository must be a non-empty string before directory resolution
        if (!skill.repository || typeof skill.repository !== "string") {
          failures.push({
            workPackageId: wp.id,
            skillName: skill.name || skill.id || "unknown",
            status: "FAILED",
            reason: "MISSING_REPOSITORY",
            message: \`Remote skill candidate has no 'repository' field. Cannot resolve install directory.\`
          });
          continue;
        }
        // Check exact directory resolution before recommending install
        const resolution = await this.directoryResolver.resolveSkillDirectory(`;

if (ap.includes("MISSING_REPOSITORY")) {
  console.log("⏭  activation-planner.mjs: guard already present, skipping");
} else if (ap.includes(apOld)) {
  ap = ap.replace(apOld, apNew);
  writeFileSync(apPath, ap);
  console.log("✅ activation-planner.mjs: CRLF version patched");
} else if (ap.includes(apOldLF)) {
  ap = ap.replace(apOldLF, apNew);
  writeFileSync(apPath, ap);
  console.log("✅ activation-planner.mjs: LF version patched");
} else {
  console.error("❌ activation-planner.mjs: anchor not found — manual patch needed");
  // Print a snippet to debug
  const idx = ap.indexOf("Stage 3: Remote");
  if (idx !== -1) {
    console.error("Context around Stage 3:", JSON.stringify(ap.slice(idx - 10, idx + 200)));
  }
  process.exit(1);
}

console.log("\n✅ All patches applied successfully.");
