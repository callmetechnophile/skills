#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const TRIAGE = path.join(ROOT, "skills/core/sort-the-issue/SKILL.md");

function scoreComplexity(task) {
  const t = task.toLowerCase();
  const dimensions = {
    scope_breadth: /(entire|whole|system|platform|end-to-end|multiple|all services)/.test(t) ? 2 : 1,
    coupling: /(database|api|frontend|backend|infra|migration|integration)/.test(t) ? 2 : 0,
    uncertainty: /(unknown|investigate|research|redesign|architecture|ambiguous)/.test(t) ? 2 : 0,
    risk: /(security|auth|payment|production|delete|deploy|credential|medical|safety)/.test(t) ? 2 : 0,
    parallelism: /(multiple modules|independent|parallel|multi-agent|sub-agent)/.test(t) ? 2 : 0
  };
  const total = Object.values(dimensions).reduce((a,b)=>a+b,0);
  return { ...dimensions, total, band: total <= 3 ? "LOW" : total <= 6 ? "MEDIUM" : "HIGH" };
}

function classify(task) {
  const t=task.toLowerCase(), labels=[];
  const rules=[
    ["bug",/bug|error|exception|crash|broken|failing|does not work/],
    ["feature",/add|build|implement|create|support|enable/],
    ["refactor",/refactor|restructure|cleanup|simplify/],
    ["testing",/test|coverage|assertion|regression/],
    ["security",/security|vulnerability|auth|permission|secret/],
    ["research",/research|compare|evaluate|investigate/],
    ["documentation",/document|readme|guide|specification/],
    ["infrastructure",/deploy|ci\\/cd|pipeline|docker|kubernetes|cloud/]
  ];
  for (const [name,re] of rules) if(re.test(t)) labels.push(name);
  return labels.length ? labels : ["engineering"];
}

export function sortIssue(task) {
  if (!task?.trim()) throw new Error("Provide a non-empty task.");
  const complexity=scoreComplexity(task);
  const workflow=complexity.band==="HIGH" ? "multi-agent" : "single-agent";
  const types=classify(task);
  return {
    outcome: task.trim(), class: types, requirements: [],
    constraints: ["Run sort-the-issue before selecting implementation skills."],
    non_goals: [], acceptance_criteria: ["Deliver the requested outcome.", "Run relevant tests or explain why they cannot run."],
    affected_areas: [], dependencies: [], risks: [], unknowns: [], complexity,
    workflow, work_packages: [{ id: "WP-1", objective: task.trim(), independent: complexity.band==="HIGH" }],
    next_skill_queries: types.map(x => `${x} ${task}`)
  };
}

export function routeTask(task, mode="auto") {
  const triage=sortIssue(task);
  const workflow=mode==="single" ? "single-agent" : mode==="multi" ? "multi-agent" : triage.workflow;
  return {
    mandatory_first_skill: "sort-the-issue", triage,
    routing: {
      workflow,
      selection_policy: workflow==="single-agent"
        ? "Discover and load one specialist skill at a time, sequentially."
        : "Discover per work package; load specialist skills in parallel only when actual sub-agent execution is supported.",
      fallback: "If sub-agents are unavailable, run specialist work packages sequentially; never claim parallelism.",
      next_skill_queries: triage.next_skill_queries
    }
  };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args=process.argv.slice(2);
  const command=args.shift();
  const modeIndex=args.indexOf("--workflow");
  const mode=modeIndex>=0 ? (args[modeIndex+1] ?? "auto") : "auto";
  const task=args.filter((x,i)=>x!=="--workflow" && (modeIndex<0 || i!==modeIndex+1) && !x.startsWith("--")).join(" ");
  try {
    if (!fs.existsSync(TRIAGE)) throw new Error("Mandatory sort-the-issue skill is missing.");
    const result=command==="sort" ? sortIssue(task) : routeTask(task,mode);
    process.stdout.write(JSON.stringify(result,null,2)+"\\n");
  } catch(error) { process.stderr.write(error.message+"\\n"); process.exitCode=1; }
}
