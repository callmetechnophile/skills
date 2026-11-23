/**
 * Registry tracking locally installed external skills via persistent ledger.
 */
import fs from "node:fs/promises";
import path from "node:path";
import { SkillRegistry } from "./registry-base.mjs";

export class InstalledRegistry extends SkillRegistry {
  constructor(options = {}) {
    super("installed");
    this.rootDir = options.rootDir || path.resolve(".");
    this.ledgerPath = path.resolve(this.rootDir, options.ledgerPath || ".installed-skills.json");
  }

  async _loadLedger() {
    try {
      const data = await fs.readFile(this.ledgerPath, "utf8");
      const parsed = JSON.parse(data);
      return Array.isArray(parsed.installed) ? parsed.installed : [];
    } catch {
      return [];
    }
  }

  async search(query, options = {}) {
    const list = await this._loadLedger();
    if (!query || !query.trim()) return list;
    const q = query.toLowerCase().trim();
    return list.filter(s =>
      (s.name && s.name.toLowerCase().includes(q)) ||
      (s.description && s.description.toLowerCase().includes(q)) ||
      (s.skillId && s.skillId.toLowerCase().includes(q))
    );
  }

  async getMetadata(id) {
    const list = await this._loadLedger();
    return list.find(s => s.skillId === id || s.name === id) || null;
  }

  async getVersions(id) {
    const meta = await this.getMetadata(id);
    return meta ? [{ version: meta.version || "1.0.0", commitSha: meta.commitSha }] : [];
  }

  async resolveSource(id) {
    const meta = await this.getMetadata(id);
    return meta ? { type: "installed", path: meta.installationPath, repository: meta.source } : null;
  }

  async retrieveSkillContent(id) {
    const meta = await this.getMetadata(id);
    if (!meta || !meta.installationPath) throw new Error(`Installed skill '${id}' not found.`);
    const full = path.resolve(this.rootDir, meta.installationPath, "SKILL.md");
    return await fs.readFile(full, "utf8");
  }

  async verifyProvenance(id) {
    const meta = await this.getMetadata(id);
    if (!meta) return { verified: false, reason: "Skill not found in installed registry." };
    return {
      verified: true,
      source: meta.source,
      commitSha: meta.commitSha,
      license: meta.license,
      contentHash: meta.contentHash,
      trustStatus: meta.trustStatus || "APPROVED"
    };
  }
}
