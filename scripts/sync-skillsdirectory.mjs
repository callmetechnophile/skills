#!/usr/bin/env node
import path from "node:path";
import { fileURLToPath } from "node:url";
import { SkillsDirectoryRegistry } from "../src/discovery/skillsdirectory-registry.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);

function option(name, fallback = null) {
  const i = args.indexOf(name);
  return i >= 0 ? (args[i + 1] ?? fallback) : fallback;
}

const isJson = args.includes("--json");
const restart = args.includes("--restart") || process.env.RESTART === "1";
const outputPath = path.resolve(ROOT, option("--output", process.env.OUTPUT || "catalog/skillsdirectory-index.json"));
const pageSize = Number(option("--page-size", process.env.PAGE_SIZE || "100"));
const maxPages = option("--max-pages", process.env.MAX_PAGES) ? Number(option("--max-pages", process.env.MAX_PAGES)) : undefined;

const registry = new SkillsDirectoryRegistry({
  rootDir: ROOT,
  indexPath: outputPath
});

try {
  const result = await registry.syncCatalog({
    pageSize,
    maxPages,
    restart
  });

  if (isJson) {
    process.stdout.write(JSON.stringify(result, null, 2) + "\n");
  } else {
    process.stdout.write(`Registry sync complete: ${result.totalIndexed} skills indexed across ${result.syncedPages} page(s).\n`);
    if (!result.complete) {
      process.stdout.write(`Partial sync saved at next offset ${result.nextOffset}. Re-run npm run sync:catalog to resume.\n`);
    }
  }
} catch (err) {
  process.stderr.write(`Sync error: ${err.message}\n`);
  process.exit(1);
}
