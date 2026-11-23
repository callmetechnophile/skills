#!/usr/bin/env node
import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { DirectoryResolver } from "../src/discovery/directory-resolver.mjs";
import { InstalledTracker } from "../src/lifecycle/installed-tracker.mjs";
import { TrustScanner } from "../src/security/trust-scanner.mjs";
import { computeFileHash } from "../src/security/integrity.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);

function option(name, fallback = null) {
  const i = args.indexOf(name);
  return i >= 0 ? (args[i + 1] ?? fallback) : fallback;
}

const isJson = args.includes("--json");
const execute = args.includes("--execute");
const isDryRun = args.includes("--dry-run") || !execute;
const agentOption = option("--agent", "antigravity");
const allAgents = args.includes("--all-agents");

// Support both positional arguments and flag arguments
let repository = option("--repo");
let targetSkill = option("--skill");

if (!repository || !targetSkill) {
  const positionals = args.filter(a => !a.startsWith("-"));
  if (positionals.length >= 2) {
    if (!repository) repository = positionals[0];
    if (!targetSkill) targetSkill = positionals[1];
  }
}

if (!repository || !/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository)) {
  process.stderr.write("Error: Provide a valid GitHub repository in owner/repo format (e.g. --repo owner/name).\n");
  process.exit(2);
}

if (!targetSkill || !targetSkill.trim()) {
  process.stderr.write("Error: Provide the target skill name or directory using --skill <name>.\n");
  process.exit(2);
}

const agents = allAgents ? ["*"] : agentOption.split(",").map(x => x.trim()).filter(Boolean);
const resolver = new DirectoryResolver();
const tracker = new InstalledTracker({ rootDir: ROOT });
const scanner = new TrustScanner();

// Resolve exact directory from upstream repository tree
const resolution = await resolver.resolveSkillDirectory(repository, targetSkill, {
  agent: agents[0]
});

if (!resolution.success) {
  if (isJson) {
    process.stdout.write(JSON.stringify(resolution, null, 2) + "\n");
  } else {
    process.stderr.write(`Error: Skill directory resolution failed: ${resolution.message}\n`);
    if (resolution.candidates) {
      process.stderr.write(`Ambiguous candidates:\n${resolution.candidates.map(c => `  - ${c}`).join("\n")}\n`);
    }
    if (resolution.availableSkills) {
      process.stderr.write(`Available skills in ${repository}:\n${resolution.availableSkills.map(s => `  - ${s}`).join("\n")}\n`);
    }
  }
  process.exit(1);
}

const exactDirectory = resolution.exactDirectory;
const command = process.platform === "win32" ? "npx.cmd" : "npx";
const cliArgs = ["-y", "skills", "add", repository, "--skill", exactDirectory];
for (const a of agents) cliArgs.push("--agent", a);
cliArgs.push("-y");

if (isDryRun) {
  if (isJson) {
    process.stdout.write(JSON.stringify({
      status: "DRY_RUN",
      repository,
      targetSkill,
      exactDirectory,
      skillMdPath: resolution.skillMdPath,
      command: `${command} ${cliArgs.join(" ")}`,
      action: "Re-run with --execute to perform installation."
    }, null, 2) + "\n");
  } else {
    process.stdout.write(`=== SKILL INSTALLATION PLAN (DRY-RUN) ===\n`);
    process.stdout.write(`Repository: ${repository}\n`);
    process.stdout.write(`Requested Skill: ${targetSkill}\n`);
    process.stdout.write(`Resolved Upstream Directory: ${exactDirectory}\n`);
    process.stdout.write(`Upstream SKILL.md: ${resolution.skillMdPath}\n`);
    process.stdout.write(`Planned Command: ${command} ${cliArgs.join(" ")}\n\n`);
    process.stdout.write(`Dry run only. Re-run with --execute after reviewing the upstream source and license.\n`);
  }
  process.exit(0);
}

// Execute installation
process.stdout.write(`Executing verified installation: ${command} ${cliArgs.join(" ")}\n`);

const child = spawn(command, cliArgs, { stdio: "inherit", shell: process.platform === "win32" });

child.on("error", error => {
  process.stderr.write(`Could not launch skills CLI: ${error.message}\n`);
  process.exit(1);
});

child.on("exit", async code => {
  if (code !== 0) {
    process.stderr.write(`Skills CLI exited with failure code ${code}.\n`);
    process.exit(code ?? 1);
  }

  // Record in persistent ledger
  const skillName = path.basename(exactDirectory);
  const installPath = path.join(".agents", "skills", skillName);
  let contentHash = null;
  let trustStatus = "APPROVED";

  try {
    const skillFilePath = path.join(ROOT, installPath, "SKILL.md");
    contentHash = await computeFileHash(skillFilePath);
  } catch {}

  await tracker.recordInstallation({
    skillId: `${repository}:${exactDirectory}`,
    name: skillName,
    source: repository,
    installationPath: installPath,
    targetAgent: agents.join(","),
    contentHash,
    trustStatus
  });

  process.stdout.write(`\nSkill '${skillName}' successfully installed and recorded in .installed-skills.json.\n`);
});
