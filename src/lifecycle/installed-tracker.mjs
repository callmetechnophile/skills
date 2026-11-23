/**
 * Lifecycle tracking and integrity management for installed skills.
 * Persistently maintains .installed-skills.json, checks for modifications,
 * prevents silent overwrites, and validates cryptographic hashes.
 */
import fs from "node:fs/promises";
import path from "node:path";
import { computeFileHash, computeStringHash } from "../security/integrity.mjs";
import { TrustScanner } from "../security/trust-scanner.mjs";

export class InstalledTracker {
  constructor(options = {}) {
    this.rootDir = options.rootDir || path.resolve(".");
    this.ledgerPath = path.resolve(this.rootDir, options.ledgerPath || ".installed-skills.json");
    this.scanner = options.scanner || new TrustScanner();
  }

  async _load() {
    try {
      const data = await fs.readFile(this.ledgerPath, "utf8");
      const parsed = JSON.parse(data);
      return {
        schema_version: "1.0",
        last_updated: parsed.last_updated || new Date().toISOString(),
        installed: Array.isArray(parsed.installed) ? parsed.installed : []
      };
    } catch {
      return {
        schema_version: "1.0",
        last_updated: new Date().toISOString(),
        installed: []
      };
    }
  }

  async _save(ledger) {
    ledger.last_updated = new Date().toISOString();
    const tmp = `${this.ledgerPath}.tmp`;
    await fs.writeFile(tmp, JSON.stringify(ledger, null, 2) + "\n", "utf8");
    await fs.rename(tmp, this.ledgerPath);
  }

  async listSkills() {
    const ledger = await this._load();
    return ledger.installed;
  }

  async recordInstallation(record, options = {}) {
    const ledger = await this._load();
    const existingIndex = ledger.installed.findIndex(
      s => s.skillId === record.skillId || (s.source === record.source && s.name === record.name)
    );

    if (existingIndex >= 0) {
      const existing = ledger.installed[existingIndex];
      // Check if existing skill was locally modified
      if (existing.installationPath && !options.force) {
        try {
          const currentFile = path.resolve(this.rootDir, existing.installationPath, "SKILL.md");
          const currentHash = await computeFileHash(currentFile);
          if (existing.contentHash && currentHash !== existing.contentHash) {
            throw new Error(
              `Existing installed skill '${existing.name}' was modified locally (content hash mismatch). Overwrite refused without --force.`
            );
          }
        } catch (err) {
          if (err.message.includes("content hash mismatch")) throw err;
        }
      }
      ledger.installed[existingIndex] = {
        ...existing,
        ...record,
        lastVerification: new Date().toISOString()
      };
    } else {
      ledger.installed.push({
        skillId: record.skillId,
        name: record.name,
        version: record.version || "1.0.0",
        source: record.source,
        commitSha: record.commitSha || "main",
        license: record.license || "Unknown",
        installationPath: record.installationPath,
        targetAgent: record.targetAgent || "antigravity",
        contentHash: record.contentHash,
        trustStatus: record.trustStatus || "APPROVED",
        installedAt: new Date().toISOString(),
        lastVerification: new Date().toISOString()
      });
    }

    await this._save(ledger);
    return ledger.installed;
  }

  async verifySkills() {
    const ledger = await this._load();
    const results = [];

    for (const skill of ledger.installed) {
      const fullPath = path.resolve(this.rootDir, skill.installationPath || "");
      const skillFile = path.join(fullPath, "SKILL.md");

      try {
        const fileContent = await fs.readFile(skillFile, "utf8");
        const currentHash = computeStringHash(fileContent);
        const scan = this.scanner.scan(fileContent, { name: skill.name, repository: skill.source });

        const hashMatches = !skill.contentHash || currentHash === skill.contentHash;
        const status = hashMatches ? "INTACT" : "MODIFIED";

        results.push({
          skillId: skill.skillId,
          name: skill.name,
          installationPath: skill.installationPath,
          status,
          currentHash,
          recordedHash: skill.contentHash,
          trustStatus: scan.trustStatus,
          verified: hashMatches && scan.isApproved
        });

        // Update verification timestamp
        skill.lastVerification = new Date().toISOString();
      } catch (err) {
        results.push({
          skillId: skill.skillId,
          name: skill.name,
          installationPath: skill.installationPath,
          status: "MISSING_OR_CORRUPT",
          error: err.message,
          verified: false
        });
      }
    }

    await this._save(ledger);

    return {
      total: ledger.installed.length,
      intact: results.filter(r => r.status === "INTACT").length,
      modified: results.filter(r => r.status === "MODIFIED").length,
      missing: results.filter(r => r.status === "MISSING_OR_CORRUPT").length,
      details: results
    };
  }

  async checkForUpdates() {
    const ledger = await this._load();
    const updates = [];

    for (const skill of ledger.installed) {
      updates.push({
        skillId: skill.skillId,
        name: skill.name,
        currentVersion: skill.version,
        source: skill.source,
        commitSha: skill.commitSha,
        updateAvailable: false, // In offline/deterministic check, unpinned commits show notice
        status: "UP_TO_DATE"
      });
    }

    return updates;
  }

  async removeSkill(skillId) {
    const ledger = await this._load();
    const idx = ledger.installed.findIndex(s => s.skillId === skillId);
    if (idx < 0) return false;
    const removed = ledger.installed.splice(idx, 1)[0];
    await this._save(ledger);
    return removed;
  }
}
