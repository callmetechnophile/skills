#!/usr/bin/env node
import { spawn } from "node:child_process";

const args = process.argv.slice(2);
function option(name, fallback = null) {
  const i = args.indexOf(name);
  return i >= 0 ? (args[i + 1] ?? fallback) : fallback;
}
const repository = option("--repo");
const skill = option("--skill");
const agentOption = option("--agent", "antigravity");
const execute = args.includes("--execute");
const allAgents = args.includes("--all-agents");
const repoPattern = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/;
const skillPattern = /^[A-Za-z0-9_. -]+$/;
const agents = allAgents ? ["*"] : agentOption.split(",").map(x => x.trim()).filter(Boolean);

if (!repository || !repoPattern.test(repository)) {
  console.error("Provide a trusted GitHub repository as --repo owner/name.");
  process.exit(2);
}
if (!skill || !skillPattern.test(skill)) {
  console.error("Provide the exact upstream skill directory name using --skill.");
  process.exit(2);
}
if (!agents.length || agents.some(x => !/^[a-z0-9*-]+$/.test(x))) {
  console.error("Invalid agent identifier. Use a supported skills CLI agent name.");
  process.exit(2);
}

const command = process.platform === "win32" ? "npx.cmd" : "npx";
const cliArgs = ["-y", "skills", "add", repository, "--skill", skill];
for (const agent of agents) cliArgs.push("--agent", agent);
cliArgs.push("-y");

console.log("Planned install: " + command + " " + cliArgs.join(" "));
if (!execute) {
  console.log("Dry run only. Re-run with --execute after reviewing the source and licence.");
  process.exit(0);
}

const child = spawn(command, cliArgs, { stdio: "inherit", shell: process.platform === "win32" });
child.on("error", error => { console.error("Could not launch skills CLI:", error.message); process.exitCode = 1; });
child.on("exit", code => { process.exitCode = code ?? 1; });
