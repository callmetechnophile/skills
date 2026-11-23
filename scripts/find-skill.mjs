#!/usr/bin/env node
import fs from "node:fs/promises";
import path from "node:path";
import { rankSkills } from "../src/registry.mjs";

const query = process.argv.slice(2).join(" ").trim();
if (!query) {
  console.error('Usage: npm run find -- "database migration"');
  process.exit(2);
}
const indexPath = path.resolve("catalog/skillsdirectory-index.json");
let skills = [];
let indexComplete = false;
try {
  const index = JSON.parse(await fs.readFile(indexPath, "utf8"));
  skills = index.skills || [];
  indexComplete = Boolean(index.complete);
} catch {}

if (!indexComplete || skills.length === 0) {
  const url = new URL("https://www.skillsdirectory.com/api/registry");
  url.searchParams.set("q", query);
  url.searchParams.set("limit", "100");
  url.searchParams.set("sort", "stars");
  const response = await fetch(url, { headers: { accept: "application/json" }, signal: AbortSignal.timeout(20000) });
  if (!response.ok) throw new Error("SkillsDirectory search failed: HTTP " + response.status);
  const payload = await response.json();
  skills = payload.skills || [];
}

const matches = rankSkills(skills, query).slice(0, 10);
if (!matches.length) {
  console.log("No matching skill found. Sync the catalogue or try a broader query.");
  process.exit(0);
}
for (const [i, skill] of matches.entries()) {
  const skillName = String(skill.name || skill.slug).toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
  console.log((i + 1) + ". " + skill.name + " [" + skill.relevanceScore + "]");
  console.log("   " + (skill.description || "No description"));
  console.log("   Source: " + skill.repository + " | Registry: " + skill.slug);
  console.log("   Verify skill folder, then install: npx -y skills add " + skill.repository + " --skill \"" + skillName + "\" --agent antigravity -y");
}
