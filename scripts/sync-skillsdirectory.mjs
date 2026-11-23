#!/usr/bin/env node
import fs from "node:fs/promises";
import path from "node:path";

const endpoint = "https://www.skillsdirectory.com/api/registry";
const outputPath = path.resolve(process.env.OUTPUT || "catalog/skillsdirectory-index.json");
const pageSize = Math.min(100, Math.max(1, Number(process.env.PAGE_SIZE || 100)));
const maxPages = process.env.MAX_PAGES ? Math.max(1, Number(process.env.MAX_PAGES)) : Infinity;
const wait = (ms) => new Promise(resolve => setTimeout(resolve, ms));

await fs.mkdir(path.dirname(outputPath), { recursive: true });
let state = { source: endpoint, fetchedAt: null, total: null, nextOffset: 0, complete: false, skills: [] };
if (process.env.RESTART !== "1") {
  try {
    const existing = JSON.parse(await fs.readFile(outputPath, "utf8"));
    // Resume an interrupted sync. A completed index is refreshed from offset 0 on the next run.
    if (Array.isArray(existing.skills) && existing.complete !== true) state = existing;
  } catch {}
}
let offset = Number(process.env.START_OFFSET ?? state.nextOffset ?? 0);
if (!Number.isInteger(offset) || offset < 0) throw new Error("START_OFFSET must be a non-negative integer.");
const bySlug = new Map(state.skills.map(skill => [skill.slug, skill]));
let pages = 0;

async function persist(complete = false) {
  state.fetchedAt = new Date().toISOString();
  state.nextOffset = offset;
  state.complete = complete;
  state.skills = [...bySlug.values()];
  const temp = outputPath + ".tmp";
  await fs.writeFile(temp, JSON.stringify(state, null, 2) + "\n", "utf8");
  await fs.rename(temp, outputPath);
}

while (pages < maxPages) {
  const url = new URL(endpoint);
  url.searchParams.set("limit", String(pageSize));
  url.searchParams.set("offset", String(offset));
  url.searchParams.set("sort", process.env.SORT || "stars");

  let response;
  for (let attempt = 0; attempt < 4; attempt++) {
    try {
      response = await fetch(url, { headers: { accept: "application/json" }, signal: AbortSignal.timeout(20000) });
      if (response.ok) break;
      if (response.status < 500 && response.status !== 429) {
        throw new Error("SkillsDirectory registry returned HTTP " + response.status + ": " + (await response.text()).slice(0, 500));
      }
    } catch (error) {
      if (attempt === 3) throw error;
    }
    if (attempt < 3) await wait(500 * (2 ** attempt));
  }
  if (!response?.ok) throw new Error("SkillsDirectory registry request failed after retries.");

  const payload = await response.json();
  if (!Array.isArray(payload.skills)) throw new Error("Unexpected registry response: expected a skills array.");
  state.total = payload.pagination?.total ?? state.total;
  if (payload.skills.length === 0) {
    await persist(true);
    break;
  }

  for (const skill of payload.skills) {
    if (!skill.slug || !skill.repository) continue;
    bySlug.set(skill.slug, {
      name: skill.name ?? skill.slug,
      slug: skill.slug,
      description: skill.description ?? "",
      repository: skill.repository,
      category: skill.category ?? null,
      author: skill.author ?? null,
      stars: Number(skill.stars ?? 0),
      verified: Boolean(skill.verified),
      tags: Array.isArray(skill.tags) ? skill.tags : []
    });
  }

  offset += payload.skills.length;
  pages += 1;
  const hasMore = Boolean(payload.pagination?.hasMore);
  await persist(!hasMore);
  process.stdout.write("Synced page " + pages + "; " + bySlug.size + " skills indexed; next offset " + offset + ".\n");
  if (!hasMore) break;
  await wait(150);
}

if (pages === maxPages && maxPages !== Infinity) {
  process.stdout.write("Partial sync saved. Re-run npm run sync:catalog to resume.\n");
} else {
  process.stdout.write("Registry sync complete: " + bySlug.size + " skill records in " + outputPath + ".\n");
}
