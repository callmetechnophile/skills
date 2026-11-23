/**
 * Local Skill Registry discovering skills bundled within the repository and listed in catalog/manifest.json.
 */
import fs from "node:fs/promises";
import path from "node:path";
import { SkillRegistry } from "./registry-base.mjs";

export class LocalRegistry extends SkillRegistry {
  constructor(options = {}) {
    super("local");
    this.rootDir = options.rootDir || path.resolve(".");
    this.skillsDir = path.resolve(this.rootDir, options.skillsDir || "skills");
    this.manifestPath = path.resolve(this.rootDir, options.manifestPath || "catalog/manifest.json");
  }

  async _loadManifest() {
    try {
      const data = await fs.readFile(this.manifestPath, "utf8");
      return JSON.parse(data);
    } catch {
      return { skills: [] };
    }
  }

  _parseFrontmatter(content) {
    const match = content.match(/^---\r?\n([\s\S]*?)\r?\n---/);
    if (!match) return { name: "", description: "" };
    const raw = match[1];
    const nameMatch = raw.match(/name:\s*([^\r\n]+)/);
    const descMatch = raw.match(/description:\s*([^\r\n]+)/);
    const agentsMatch = raw.match(/supported_agents:\s*\[([\s\S]*?)\]/);
    let supportedAgents = null;
    if (agentsMatch) {
      supportedAgents = agentsMatch[1].split(",").map(a => a.replace(/["'\s]/g, "")).filter(Boolean);
    }
    return {
      name: nameMatch ? nameMatch[1].trim() : "",
      description: descMatch ? descMatch[1].trim() : "",
      supported_agents: supportedAgents
    };
  }

  async _scanDir(dir) {
    const results = [];
    try {
      const entries = await fs.readdir(dir, { withFileTypes: true });
      for (const entry of entries) {
        const full = path.join(dir, entry.name);
        if (entry.isDirectory()) {
          results.push(...await this._scanDir(full));
        } else if (entry.isFile() && entry.name.toLowerCase() === "skill.md") {
          results.push(full);
        }
      }
    } catch {}
    return results;
  }

  async getAllSkills() {
    const manifest = await this._loadManifest();
    const manifestMap = new Map((manifest.skills || []).map(s => [s.name, s]));
    const skillFiles = await this._scanDir(this.skillsDir);
    const skills = [];

    for (const file of skillFiles) {
      try {
        const content = await fs.readFile(file, "utf8");
        const meta = this._parseFrontmatter(content);
        const relPath = path.relative(this.rootDir, file).replace(/\\/g, "/");
        const dirName = path.basename(path.dirname(file));
        const id = meta.name || dirName;
        const manifestEntry = manifestMap.get(id) || {};

        skills.push({
          id,
          name: meta.name || dirName,
          slug: id,
          description: meta.description || "",
          path: relPath,
          source: manifestEntry.source || "local",
          license: manifestEntry.license || "Apache-2.0",
          security_status: manifestEntry.security_status || "approved-local",
          verified: true,
          supported_agents: meta.supported_agents || null,
          tags: ["local", "core", dirName],
          registry: "local",
          repository: "callmetechnophile/skills"
        });
      } catch {}
    }

    return skills;
  }

  async search(query, options = {}) {
    const all = await this.getAllSkills();
    if (!query || !query.trim()) return all;
    const q = query.toLowerCase().trim();
    return all.filter(s =>
      s.name.toLowerCase().includes(q) ||
      s.description.toLowerCase().includes(q) ||
      s.tags.some(t => t.toLowerCase().includes(q))
    );
  }

  async getMetadata(id) {
    const all = await this.getAllSkills();
    return all.find(s => s.id === id || s.name === id || s.slug === id) || null;
  }

  async getVersions(id) {
    const meta = await this.getMetadata(id);
    if (!meta) return [];
    return [{ version: "0.2.0", current: true }];
  }

  async resolveSource(id) {
    const meta = await this.getMetadata(id);
    if (!meta) return null;
    return {
      type: "local",
      path: meta.path,
      repository: meta.repository
    };
  }

  async retrieveSkillContent(id) {
    const meta = await this.getMetadata(id);
    if (!meta) throw new Error(`Local skill '${id}' not found.`);
    const fullPath = path.resolve(this.rootDir, meta.path);
    return await fs.readFile(fullPath, "utf8");
  }

  async verifyProvenance(id) {
    const meta = await this.getMetadata(id);
    if (!meta) return { verified: false, reason: "Skill not found." };
    return {
      verified: true,
      provenance: "locally-authored",
      license: meta.license,
      security_status: meta.security_status
    };
  }
}
