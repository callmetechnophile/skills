/**
 * Registry adapter for SkillsDirectory.com public metadata index and sync.
 */
import fs from "node:fs/promises";
import path from "node:path";
import { SkillRegistry } from "./registry-base.mjs";

const DEFAULT_ENDPOINT = "https://www.skillsdirectory.com/api/registry";

export class SkillsDirectoryRegistry extends SkillRegistry {
  constructor(options = {}) {
    super("skillsdirectory");
    this.rootDir = options.rootDir || path.resolve(".");
    this.indexPath = path.resolve(this.rootDir, options.indexPath || "catalog/skillsdirectory-index.json");
    this.endpoint = options.endpoint || DEFAULT_ENDPOINT;
    this.fetchFn = options.fetchFn || globalThis.fetch;
    this.noNetwork = options.noNetwork || false;
  }

  async _loadIndex() {
    try {
      const data = await fs.readFile(this.indexPath, "utf8");
      const parsed = JSON.parse(data);
      return parsed;
    } catch {
      return {
        source: this.endpoint,
        fetchedAt: null,
        total: null,
        nextOffset: 0,
        complete: false,
        skills: []
      };
    }
  }

  async search(query, options = {}) {
    const index = await this._loadIndex();
    let list = Array.isArray(index.skills) ? index.skills : [];

    // If index is empty and network is allowed, query remote endpoint
    if (list.length === 0 && !this.noNetwork && !options.offlineOnly) {
      try {
        const url = new URL(this.endpoint);
        url.searchParams.set("q", query || "");
        url.searchParams.set("limit", String(options.limit || 50));
        url.searchParams.set("sort", "stars");

        const res = await this.fetchFn(url.toString(), {
          headers: { accept: "application/json" },
          signal: AbortSignal.timeout(options.timeoutMs || 8000)
        });
        if (res.ok) {
          const payload = await res.json();
          if (Array.isArray(payload.skills)) {
            list = payload.skills.map(this._normalizeRemoteSkill);
          }
        }
      } catch (err) {
        // Fall back to empty list on network error
      }
    }

    if (!query || !query.trim()) return list;
    const q = query.toLowerCase().trim();
    return list.filter(s =>
      (s.name && s.name.toLowerCase().includes(q)) ||
      (s.description && s.description.toLowerCase().includes(q)) ||
      (s.slug && s.slug.toLowerCase().includes(q)) ||
      (Array.isArray(s.tags) && s.tags.some(t => t.toLowerCase().includes(q)))
    );
  }

  _normalizeRemoteSkill(raw) {
    const slug = raw.slug || String(raw.name || "").toLowerCase().replace(/[^a-z0-9]+/g, "-");
    const repoParts = String(raw.repository || "").split("/");
    const owner = repoParts.length >= 2 ? repoParts[0] : (raw.author || "unknown");
    const repo = repoParts.length >= 2 ? repoParts[1] : "skills";

    return {
      id: `sd:${slug}`,
      slug,
      name: raw.name || slug,
      owner,
      repository: raw.repository || `${owner}/${repo}`,
      skill_directory: raw.skill_directory || raw.skillDirectory || null,
      description: raw.description || "",
      tags: Array.isArray(raw.tags) ? raw.tags : [],
      category: raw.category || "general",
      version: raw.version || "1.0.0",
      license: raw.license || "Unknown",
      stars: Number(raw.stars || 0),
      verified: Boolean(raw.verified),
      last_updated: raw.last_updated || raw.updatedAt || new Date().toISOString(),
      registry: "skillsdirectory",
      retrieval_status: "metadata-only"
    };
  }

  async getMetadata(id) {
    const index = await this._loadIndex();
    const skills = Array.isArray(index.skills) ? index.skills : [];
    return skills.find(s => s.id === id || s.slug === id || `sd:${s.slug}` === id) || null;
  }

  async getVersions(id) {
    const meta = await this.getMetadata(id);
    return meta ? [{ version: meta.version, current: true }] : [];
  }

  async resolveSource(id) {
    const meta = await this.getMetadata(id);
    return meta ? {
      type: "remote-github",
      repository: meta.repository,
      skill_directory: meta.skill_directory,
      owner: meta.owner
    } : null;
  }

  async retrieveSkillContent(id) {
    throw new Error(
      `Full skill content for remote registry item '${id}' cannot be downloaded automatically without verified directory resolution and authorization.`
    );
  }

  async verifyProvenance(id) {
    const meta = await this.getMetadata(id);
    if (!meta) return { verified: false, reason: "Skill not found in catalogue." };
    return {
      verified: meta.verified,
      registry: "skillsdirectory",
      source_repository: meta.repository,
      license: meta.license,
      retrieval_status: meta.retrieval_status
    };
  }

  /**
   * Resumable catalogue synchronization with pagination, deduplication, and atomic write.
   */
  async syncCatalog(options = {}) {
    const pageSize = Math.min(100, Math.max(1, Number(options.pageSize || 100)));
    const maxPages = options.maxPages ? Math.max(1, Number(options.maxPages)) : Infinity;
    const restart = options.restart === true;
    const wait = (ms) => new Promise(resolve => setTimeout(resolve, ms));

    await fs.mkdir(path.dirname(this.indexPath), { recursive: true });

    let state = {
      source: this.endpoint,
      fetchedAt: null,
      total: null,
      nextOffset: 0,
      complete: false,
      skills: []
    };

    if (!restart) {
      try {
        const existing = await this._loadIndex();
        if (Array.isArray(existing.skills) && existing.complete !== true) {
          state = existing;
        }
      } catch {}
    }

    let offset = options.startOffset !== undefined ? Number(options.startOffset) : (state.nextOffset || 0);
    const bySlug = new Map((state.skills || []).map(s => [s.slug, s]));
    let pages = 0;

    const persist = async (isComplete) => {
      state.fetchedAt = new Date().toISOString();
      state.nextOffset = offset;
      state.complete = isComplete;
      state.skills = [...bySlug.values()];
      const tmpPath = `${this.indexPath}.tmp`;
      await fs.writeFile(tmpPath, JSON.stringify(state, null, 2) + "\n", "utf8");
      await fs.rename(tmpPath, this.indexPath);
    };

    while (pages < maxPages) {
      const url = new URL(this.endpoint);
      url.searchParams.set("limit", String(pageSize));
      url.searchParams.set("offset", String(offset));
      url.searchParams.set("sort", options.sort || "stars");

      let response;
      for (let attempt = 0; attempt < 3; attempt++) {
        try {
          response = await this.fetchFn(url.toString(), {
            headers: { accept: "application/json" },
            signal: AbortSignal.timeout(options.timeoutMs || 15000)
          });
          if (response.ok) break;
          if (response.status < 500 && response.status !== 429) {
            throw new Error(`SkillsDirectory returned HTTP ${response.status}`);
          }
        } catch (err) {
          if (attempt === 2) throw err;
          await wait(300 * Math.pow(2, attempt));
        }
      }

      if (!response || !response.ok) {
        throw new Error(`SkillsDirectory request failed after retries (offset ${offset})`);
      }

      let payload;
      try {
        payload = await response.json();
      } catch {
        throw new Error("SkillsDirectory returned malformed JSON response");
      }

      if (!Array.isArray(payload.skills)) {
        throw new Error("SkillsDirectory response missing 'skills' array");
      }

      state.total = payload.pagination?.total ?? state.total;

      if (payload.skills.length === 0) {
        await persist(true);
        break;
      }

      for (const raw of payload.skills) {
        if (!raw.slug) continue;
        const normalized = this._normalizeRemoteSkill(raw);
        bySlug.set(normalized.slug, normalized);
      }

      offset += payload.skills.length;
      pages += 1;
      const hasMore = Boolean(payload.pagination?.hasMore);
      await persist(!hasMore);

      if (!hasMore) break;
      if (options.throttleMs) await wait(options.throttleMs);
    }

    const finalComplete = pages !== maxPages || state.complete === true;
    return {
      syncedPages: pages,
      totalIndexed: bySlug.size,
      nextOffset: offset,
      complete: finalComplete
    };
  }
}
