import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { TrustScanner, TRUST_STATES } from "../src/security/trust-scanner.mjs";
import { computeStringHash } from "../src/security/integrity.mjs";
import { InstalledTracker } from "../src/lifecycle/installed-tracker.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const STAGING = "C:/Users/worka/.gemini/antigravity/scratch/skill-sources";
const COMMUNITY_DIR = path.join(ROOT, "skills/community");
const CORE_DIR = path.join(ROOT, "skills/core");

const scanner = new TrustScanner();
const tracker = new InstalledTracker({ rootDir: ROOT, scanner });

async function findSkillFiles(dir) {
  const list = [];
  try {
    const entries = await fs.readdir(dir, { withFileTypes: true });
    for (const e of entries) {
      if (e.name === ".git" || e.name === "node_modules") continue;
      const full = path.join(dir, e.name);
      if (e.isDirectory()) {
        list.push(...await findSkillFiles(full));
      } else if (e.isFile() && e.name.toLowerCase() === "skill.md") {
        list.push(full);
      }
    }
  } catch {}
  return list;
}

function parseFrontmatter(content, folderName) {
  const match = content.match(/^---\r?\n([\s\S]*?)\r?\n---/);
  let name = "";
  let description = "";
  if (match) {
    const nameMatch = match[1].match(/name:\s*([^\r\n]+)/);
    const descMatch = match[1].match(/description:\s*([^\r\n]+)/);
    if (nameMatch) name = nameMatch[1].trim();
    if (descMatch) description = descMatch[1].trim();
  }
  if (!name) {
    name = folderName.toLowerCase().replace(/[^a-z0-9_-]/g, "-").replace(/-+/g, "-");
  }
  if (!description) {
    // derive from first paragraph or header
    const lines = content.replace(/^---\r?\n[\s\S]*?\r?\n---/, "").trim().split("\n");
    const firstLine = lines.find(l => l.trim().length > 0) || "";
    description = firstLine.replace(/^#+\s*/, "").slice(0, 150).trim() || `Skill for ${name}`;
  }
  return { name, description, hasFrontmatter: Boolean(match) };
}

async function main() {
  await fs.mkdir(COMMUNITY_DIR, { recursive: true });

  // 1. Record existing core skills into tracker
  const coreDirs = await fs.readdir(CORE_DIR, { withFileTypes: true });
  for (const c of coreDirs) {
    if (c.isDirectory()) {
      const skillPath = path.join(CORE_DIR, c.name, "SKILL.md");
      try {
        const text = await fs.readFile(skillPath, "utf8");
        const hash = computeStringHash(text);
        await tracker.recordInstallation({
          skillId: `core-${c.name}`,
          name: c.name,
          version: "0.2.0",
          source: "callmetechnophile/skills",
          license: "Apache-2.0",
          installationPath: `skills/core/${c.name}`,
          contentHash: hash,
          trustStatus: "APPROVED"
        }, { force: true });
      } catch (e) {
        console.error("Core skill error:", e.message);
      }
    }
  }

  // 2. Discover upstream skills
  console.log("Searching staging directory for skills...");
  const skillFiles = await findSkillFiles(STAGING);
  console.log(`Found ${skillFiles.length} candidate skill files in staging.`);

  const stats = {
    approved: 0,
    rejected: 0,
    quarantined: 0,
    duplicates: 0
  };

  const existingSkills = new Set(["sort-the-issue", "automatic-skill-router"]);
  const quarantineLog = [];

  for (const skillFile of skillFiles) {
    try {
      const skillFolder = path.dirname(skillFile);
      const folderName = path.basename(skillFolder);
      const content = await fs.readFile(skillFile, "utf8");

      const sourceRepo = path.relative(STAGING, skillFile).split(path.sep)[0] || "upstream";
      const { name, description, hasFrontmatter } = parseFrontmatter(content, folderName);

      if (existingSkills.has(name)) {
        stats.duplicates++;
        continue;
      }

      // Security scan
      const scanResult = scanner.scan(content, { name, repository: sourceRepo });

      if (scanResult.trustState === TRUST_STATES.REJECTED || scanResult.trustState === TRUST_STATES.RESTRICTED) {
        stats.rejected++;
        quarantineLog.push({ name, reason: scanResult.rejectionReason, file: skillFile });
        continue;
      }

      if (scanResult.trustState === TRUST_STATES.QUARANTINED) {
        stats.quarantined++;
        quarantineLog.push({ name, reason: scanResult.rejectionReason, file: skillFile });
        continue;
      }

      // Ensure normalized frontmatter
      let finalContent = content;
      if (!hasFrontmatter) {
        finalContent = `---\nname: ${name}\ndescription: ${JSON.stringify(description)}\n---\n\n${content}`;
      }

      const destFolder = path.join(COMMUNITY_DIR, name);
      await fs.mkdir(destFolder, { recursive: true });
      await fs.writeFile(path.join(destFolder, "SKILL.md"), finalContent, "utf8");

      // Copy other helper files/folders if present
      const entries = await fs.readdir(skillFolder, { withFileTypes: true });
      for (const entry of entries) {
        if (entry.name.toLowerCase() === "skill.md" || entry.name === ".git") continue;
        const srcPath = path.join(skillFolder, entry.name);
        const destPath = path.join(destFolder, entry.name);
        try {
          await fs.cp(srcPath, destPath, { recursive: true });
        } catch {}
      }

      const hash = computeStringHash(finalContent);
      await tracker.recordInstallation({
        skillId: `community-${name}`,
        name,
        version: "1.0.0",
        source: sourceRepo,
        license: "MIT",
        installationPath: `skills/community/${name}`,
        contentHash: hash,
        trustStatus: "APPROVED"
      }, { force: true });

      existingSkills.add(name);
      stats.approved++;
    } catch (err) {
      // skip corrupted files
    }
  }

  console.log("=== INGESTION & SECURITY SCAN RESULTS ===");
  console.log(`Approved & Installed: ${stats.approved}`);
  console.log(`Rejected (Security):   ${stats.rejected}`);
  console.log(`Quarantined:          ${stats.quarantined}`);
  console.log(`Duplicates Skipped:   ${stats.duplicates}`);

  if (quarantineLog.length > 0) {
    await fs.writeFile(
      path.join(ROOT, "catalog/quarantine-log.json"),
      JSON.stringify(quarantineLog, null, 2) + "\n",
      "utf8"
    );
    console.log(`Saved quarantine details to catalog/quarantine-log.json`);
  }
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
