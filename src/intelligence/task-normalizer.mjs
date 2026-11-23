/**
 * Normalizes user input task string and extracts core metadata.
 */
import { randomUUID } from "node:crypto";

const DOMAIN_KEYWORDS = {
  frontend: ["frontend", "react", "vue", "angular", "css", "html", "ui", "tailwind", "next.js", "client"],
  backend: ["backend", "api", "rest", "graphql", "server", "express", "fastapi", "django", "node", "controller"],
  database: ["database", "postgres", "postgresql", "mysql", "mongodb", "sqlite", "prisma", "migration", "sql", "redis", "schema"],
  auth: ["auth", "authentication", "authorization", "jwt", "oauth", "sso", "login", "password", "token"],
  devops: ["devops", "deployment", "deploy", "docker", "kubernetes", "ci/cd", "pipeline", "github actions", "cloud", "aws", "gcp"],
  security: ["security", "vulnerability", "encryption", "secret", "permission", "cve", "sanitize", "xss", "csrf"],
  testing: ["test", "tests", "unit test", "e2e", "integration test", "jest", "coverage", "playwright"]
};

const ACTION_VERBS = {
  create: ["build", "create", "implement", "add", "develop", "generate", "scaffold"],
  fix: ["fix", "resolve", "patch", "repair", "debug", "correct"],
  optimize: ["optimize", "tune", "speed up", "refactor", "cleanup", "improve"],
  audit: ["audit", "review", "scan", "analyze", "inspect", "assess", "evaluate"],
  sync: ["sync", "update", "migrate", "upgrade"]
};

export function normalizeTask(rawTask) {
  if (typeof rawTask !== "string" || !rawTask.trim()) {
    throw new Error("Task description must be a non-empty string.");
  }

  const cleaned = rawTask.trim().replace(/\s+/g, " ");
  const lower = cleaned.toLowerCase();

  // Extract detected domains
  const domains = [];
  for (const [domain, keywords] of Object.entries(DOMAIN_KEYWORDS)) {
    if (keywords.some(kw => new RegExp(`\\b${kw.replace(".", "\\.")}\\b`, "i").test(lower))) {
      domains.push(domain);
    }
  }

  // Extract primary intent
  let intent = "general-engineering";
  for (const [action, verbs] of Object.entries(ACTION_VERBS)) {
    if (verbs.some(v => new RegExp(`\\b${v}\\b`, "i").test(lower))) {
      intent = action;
      break;
    }
  }

  return {
    taskId: randomUUID(),
    originalTask: rawTask,
    normalizedTask: cleaned,
    domains: domains.length ? domains : ["general"],
    intent
  };
}
