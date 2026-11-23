#!/usr/bin/env node
import path from "node:path";
import { fileURLToPath } from "node:url";
import { LocalRegistry } from "../src/discovery/local-registry.mjs";
import { InstalledRegistry } from "../src/discovery/installed-registry.mjs";
import { SkillsDirectoryRegistry } from "../src/discovery/skillsdirectory-registry.mjs";
import { HybridRanker } from "../src/ranking/hybrid-ranker.mjs";
import { DirectoryResolver } from "../src/discovery/directory-resolver.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const rawArgs = process.argv.slice(2);
function hasFlag(name) { return rawArgs.includes(name); }
function getOption(name, fallback = null) {
  const i = rawArgs.indexOf(name);
  return i >= 0 ? (rawArgs[i + 1] ?? fallback) : fallback;
}

const isJson = hasFlag("--json");
const noNetwork = hasFlag("--no-network");
const maxSkills = Number(getOption("--max-skills", "10"));
const targetAgent = getOption("--agent", "antigravity");

// Extract query words
const queryWords = [];
for (let i = 0; i < rawArgs.length; i++) {
  const a = rawArgs[i];
  if (a === "--max-skills" || a === "--agent" || a === "--workflow") {
    i++;
    continue;
  }
  if (a.startsWith("--")) continue;
  queryWords.push(a);
}

const query = queryWords.join(" ").trim();
if (!query) {
  process.stderr.write("Usage: npm run find -- \"<search query>\" [--json] [--no-network] [--agent <name>] [--max-skills <n>]\n");
  process.exit(2);
}

const localRegistry = new LocalRegistry({ rootDir: ROOT });
const installedRegistry = new InstalledRegistry({ rootDir: ROOT });
const remoteRegistry = new SkillsDirectoryRegistry({ rootDir: ROOT, noNetwork });
const ranker = new HybridRanker({ targetAgent });
const resolver = new DirectoryResolver();

// Gather from all registries
const [localSkills, installedSkills, remoteSkills] = await Promise.all([
  localRegistry.search(query),
  installedRegistry.search(query),
  noNetwork ? [] : remoteRegistry.search(query, { limit: 50 })
]);

const combined = [...localSkills, ...installedSkills, ...remoteSkills];
const rankResult = await ranker.rank(combined, query, { targetAgent });
const candidates = rankResult.ranked.slice(0, maxSkills);

if (isJson) {
  process.stdout.write(JSON.stringify({
    query,
    count: candidates.length,
    matches: candidates.map(c => ({
      name: c.skill.name,
      relevanceScore: c.relevanceScore,
      scoreBreakdown: c.scoreBreakdown,
      source: c.skill.source || c.skill.registry,
      repository: c.skill.repository,
      path: c.skill.path || c.skill.skill_directory,
      securityResult: c.securityResult,
      compatibilityResult: c.compatibilityResult,
      description: c.skill.description
    }))
  }, null, 2) + "\n");
  process.exit(0);
}

if (!candidates.length) {
  process.stdout.write("No matching skills found. Try syncing the catalog (`npm run sync:catalog`) or broadening the query.\n");
  process.exit(0);
}

process.stdout.write(`Found ${candidates.length} matching skills for "${query}":\n\n`);

for (const [i, c] of candidates.entries()) {
  const s = c.skill;
  const isLocal = s.source === "local" || s.registry === "local";
  const isInstalled = s.registry === "installed";

  process.stdout.write(`${i + 1}. ${s.name} [Score: ${c.relevanceScore}]\n`);
  process.stdout.write(`   Description: ${s.description || "No description provided."}\n`);
  process.stdout.write(`   Provenance: ${s.repository || s.source || "unknown"} (Registry: ${s.registry || "local"})\n`);
  process.stdout.write(`   Security: ${c.securityResult} | Compatibility: ${c.compatibilityResult}\n`);

  if (isLocal) {
    process.stdout.write(`   Action: Local core skill ready for immediate activation at ${s.path}\n`);
  } else if (isInstalled) {
    process.stdout.write(`   Action: Already installed in workspace at ${s.installationPath}\n`);
  } else {
    // Upstream remote skill
    const targetFolder = s.skill_directory || s.slug;
    process.stdout.write(`   Verify upstream tree, then install: npm run install:skill -- --repo "${s.repository}" --skill "${targetFolder}" --agent ${targetAgent}\n`);
  }
  process.stdout.write("\n");
}
