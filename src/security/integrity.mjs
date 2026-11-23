/**
 * Cryptographic content hash calculation for skills and files.
 */
import crypto from "node:crypto";
import fs from "node:fs/promises";
import path from "node:path";

export function computeStringHash(content) {
  return crypto.createHash("sha256").update(content, "utf8").digest("hex");
}

export async function computeFileHash(filePath) {
  const buffer = await fs.readFile(filePath);
  return crypto.createHash("sha256").update(buffer).digest("hex");
}

export async function computeDirectoryHash(dirPath) {
  const hashes = [];

  async function walk(current) {
    try {
      const entries = await fs.readdir(current, { withFileTypes: true });
      entries.sort((a, b) => a.name.localeCompare(b.name));
      for (const entry of entries) {
        const full = path.join(current, entry.name);
        if (entry.isDirectory()) {
          await walk(full);
        } else if (entry.isFile()) {
          const rel = path.relative(dirPath, full).replace(/\\/g, "/");
          const fileHash = await computeFileHash(full);
          hashes.push(`${rel}:${fileHash}`);
        }
      }
    } catch {}
  }

  await walk(dirPath);
  return crypto.createHash("sha256").update(hashes.join("\n"), "utf8").digest("hex");
}
