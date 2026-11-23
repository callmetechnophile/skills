#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { IssueSorter } from "./intelligence/issue-sorter.mjs";
import { ActivationPlanner } from "./activation/activation-planner.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const TRIAGE_PATH = path.join(ROOT, "skills/core/sort-the-issue/SKILL.md");

const defaultSorter = new IssueSorter();
const defaultPlanner = new ActivationPlanner({ rootDir: ROOT });

export function sortIssue(task, options = {}) {
  if (!task || typeof task !== "string" || !task.trim()) {
    throw new Error("Provide a non-empty task.");
  }
  const sorter = options.sorter || defaultSorter;
  const forcedWorkflow = options.workflow || (typeof options === "string" ? options : null);
  const syncResult = sorter.sortSync(task, forcedWorkflow);

  // Return dual object/promise
  const promise = Promise.resolve(syncResult);
  Object.assign(promise, syncResult);
  return promise;
}

export function routeTask(task, options = "auto") {
  if (!task || typeof task !== "string" || !task.trim()) {
    throw new Error("Provide a non-empty task.");
  }

  const optObj = typeof options === "string" ? { workflow: options } : (options || {});
  const sorter = optObj.sorter || defaultSorter;
  const planner = optObj.planner || defaultPlanner;

  const triage = sorter.sortSync(task, optObj.workflow);
  const workflow = optObj.workflow === "single"
    ? "single-agent"
    : optObj.workflow === "multi"
      ? "multi-agent"
      : triage.workflow;

  triage.workflow = workflow;

  const syncRouting = {
    workflow,
    selection_policy: workflow === "single-agent"
      ? "Discover and load one specialist skill at a time, sequentially."
      : "Discover per work package; load specialist skills in parallel only when actual sub-agent execution is supported.",
    fallback: "If sub-agents are unavailable, run specialist work packages sequentially; never claim parallelism.",
    next_skill_queries: triage.next_skill_queries
  };

  const syncResult = {
    mandatory_first_skill: "sort-the-issue",
    triage,
    routing: syncRouting
  };

  // Build async activation planning promise
  const asyncPlan = (async () => {
    const activationReport = await planner.planActivation(triage, {
      agent: optObj.agent || "antigravity",
      noNetwork: optObj.noNetwork || false,
      localOnly: optObj.localOnly || false
    });

    const fullResult = {
      ...syncResult,
      activation: activationReport.toJSON(),
      execution_plan: activationReport.executionPlan
    };
    Object.assign(asyncPlan, fullResult);
    return fullResult;
  })();

  Object.assign(asyncPlan, syncResult);
  return asyncPlan;
}

// CLI handler
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const rawArgs = process.argv.slice(2);
  const command = rawArgs[0] && !rawArgs[0].startsWith("-") ? rawArgs.shift() : "route";

  function hasFlag(name) {
    return rawArgs.includes(name);
  }

  function getOption(name, fallback = null) {
    const i = rawArgs.indexOf(name);
    return i >= 0 ? (rawArgs[i + 1] ?? fallback) : fallback;
  }

  const isJson = hasFlag("--json");
  const noNetwork = hasFlag("--no-network");
  const agent = getOption("--agent", "antigravity");
  const workflow = getOption("--workflow", "auto");

  // Filter out flags and their arguments to extract task string
  const taskWords = [];
  for (let i = 0; i < rawArgs.length; i++) {
    const arg = rawArgs[i];
    if (arg === "--workflow" || arg === "--agent" || arg === "--max-skills") {
      i++;
      continue;
    }
    if (arg.startsWith("--")) continue;
    taskWords.push(arg);
  }

  const task = taskWords.join(" ").trim();

  try {
    if (!fs.existsSync(TRIAGE_PATH)) {
      throw new Error("Mandatory sort-the-issue skill is missing from skills/core/sort-the-issue/SKILL.md.");
    }
    if (!task) {
      process.stderr.write("Usage: node src/router.mjs [sort|route] \"<task description>\" [--json] [--workflow auto|single|multi] [--agent <name>]\n");
      process.exit(2);
    }

    if (command === "sort") {
      const result = await sortIssue(task, { workflow });
      if (isJson) {
        process.stdout.write(JSON.stringify(result, null, 2) + "\n");
      } else {
        process.stdout.write(`=== SORT-THE-ISSUE TRIAGE ===\n`);
        process.stdout.write(`Outcome: ${result.outcome}\n`);
        process.stdout.write(`Class: ${result.class.join(", ")}\n`);
        process.stdout.write(`Complexity: ${result.complexity.band} (Score: ${result.complexity.total_score}/10)\n`);
        process.stdout.write(`Recommended Workflow: ${result.workflow}\n`);
        process.stdout.write(`Work Packages (${result.work_packages.length}):\n`);
        for (const wp of result.work_packages) {
          process.stdout.write(`  - [${wp.id}] ${wp.title} (Deps: ${wp.dependencies.length ? wp.dependencies.join(", ") : "none"})\n`);
        }
      }
    } else {
      const result = await routeTask(task, { workflow, agent, noNetwork });
      if (isJson) {
        process.stdout.write(JSON.stringify(result, null, 2) + "\n");
      } else {
        process.stdout.write(`=== AUTOMATIC SKILL ROUTER V0.2 ===\n`);
        process.stdout.write(`Task: ${result.triage.outcome}\n`);
        process.stdout.write(`Mandatory First Skill: ${result.mandatory_first_skill}\n`);
        process.stdout.write(`Complexity: ${result.triage.complexity.band} (Score: ${result.triage.complexity.total_score}/10)\n`);
        process.stdout.write(`Workflow: ${result.routing.workflow}\n`);
        process.stdout.write(`Execution Mode: ${result.execution_plan?.selectedMode || "sequential"}\n`);
        if (result.execution_plan?.downgradeNotice) {
          process.stdout.write(`Notice: ${result.execution_plan.downgradeNotice}\n`);
        }
        process.stdout.write(`\nActivations:\n`);
        for (const act of (result.activation?.activations || [])) {
          process.stdout.write(`  - [${act.status}] ${act.skillName} (Package: ${act.workPackageId}, Trust: ${act.trustStatus})\n`);
        }
        if (result.activation?.failures?.length) {
          process.stdout.write(`\nBlocked / Unresolved:\n`);
          for (const f of result.activation.failures) {
            process.stdout.write(`  - [${f.status}] ${f.skillName}: ${f.message}\n`);
          }
        }
      }
    }
  } catch (error) {
    process.stderr.write(`Error: ${error.message}\n`);
    process.exit(1);
  }
}
