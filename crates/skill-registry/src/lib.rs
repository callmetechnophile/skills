use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Skill not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Registry error: {0}")]
    Custom(String),
}

pub fn compute_sha256(content: &str) -> String {
    compute_bytes_sha256(content.as_bytes())
}

pub fn compute_bytes_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Computes a deterministic SHA-256 digest over a set of `(relative_path, content_bytes)` entries.
///
/// Canonical format:
/// 1. Normalize path separators to `/` and strip leading `./`.
/// 2. Sort entries lexicographically by normalized relative path bytes.
/// 3. For each entry, feed `u64::to_le_bytes(path.len())`, `path.as_bytes()`,
///    `u64::to_le_bytes(content.len())`, and `content` into SHA-256.
pub fn compute_entries_sha256(entries: &[(String, Vec<u8>)]) -> String {
    let mut normalized: Vec<(String, &[u8])> = entries
        .iter()
        .map(|(p, bytes)| {
            let norm = p.replace('\\', "/");
            let norm = norm.strip_prefix("./").unwrap_or(&norm).to_string();
            (norm, bytes.as_slice())
        })
        .collect();

    normalized.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));

    let mut hasher = Sha256::new();
    for (rel_path, content) in normalized {
        hasher.update((rel_path.len() as u64).to_le_bytes());
        hasher.update(rel_path.as_bytes());
        hasher.update((content.len() as u64).to_le_bytes());
        hasher.update(content);
    }
    format!("{:x}", hasher.finalize())
}

fn is_temp_or_ignored_filename(name: &str) -> bool {
    name == ".DS_Store" || name.ends_with(".tmp") || name.contains(".tmp.")
}

fn collect_dir_entries(
    base: &Path,
    current: &Path,
    out: &mut Vec<(String, Vec<u8>)>,
) -> Result<(), std::io::Error> {
    let mut dir_entries: Vec<PathBuf> = std::fs::read_dir(current)?
        .filter_map(|e| e.ok().map(|entry| entry.path()))
        .collect();
    dir_entries.sort();

    for path in dir_entries {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if is_temp_or_ignored_filename(file_name) {
            continue;
        }

        let meta = std::fs::symlink_metadata(&path)?;
        if meta.file_type().is_symlink() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Symlink detected in skill directory: {}", path.display()),
            ));
        }

        if meta.is_dir() {
            collect_dir_entries(base, &path, out)?;
        } else if meta.is_file() {
            let rel = path
                .strip_prefix(base)
                .map_err(std::io::Error::other)?
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = std::fs::read(&path)?;
            out.push((rel, bytes));
        }
    }
    Ok(())
}

/// Computes the canonical SHA-256 digest of a complete skill directory on disk.
pub fn compute_directory_sha256(dir: &Path) -> Result<String, std::io::Error> {
    if !dir.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Skill directory does not exist: {}", dir.display()),
        ));
    }
    let mut entries = Vec::new();
    collect_dir_entries(dir, dir, &mut entries)?;
    Ok(compute_entries_sha256(&entries))
}

#[derive(Debug, Clone, Default)]
pub struct ParsedFrontmatter {
    pub name: String,
    pub description: String,
    pub supported_agents: Option<Vec<String>>,
    pub version: Option<String>,
    pub license: Option<String>,
}

pub fn parse_frontmatter_metadata(content: &str) -> ParsedFrontmatter {
    let mut parsed = ParsedFrontmatter::default();
    let normalized = content.replace("\r\n", "\n");
    if let Some(rest) = normalized.strip_prefix("---") {
        if let Some(end_idx) = rest.find("\n---") {
            let fm = &rest[..end_idx];
            for line in fm.lines() {
                let trimmed = line.trim();
                if let Some(n) = trimmed.strip_prefix("name:") {
                    parsed.name = n.trim().trim_matches('"').trim_matches('\'').to_string();
                } else if let Some(d) = trimmed.strip_prefix("description:") {
                    parsed.description = d.trim().trim_matches('"').trim_matches('\'').to_string();
                } else if let Some(v) = trimmed.strip_prefix("version:") {
                    let val = v.trim().trim_matches('"').trim_matches('\'').to_string();
                    if !val.is_empty() {
                        parsed.version = Some(val);
                    }
                } else if let Some(l) = trimmed.strip_prefix("license:") {
                    let val = l.trim().trim_matches('"').trim_matches('\'').to_string();
                    if !val.is_empty() {
                        parsed.license = Some(val);
                    }
                } else if let Some(a) = trimmed.strip_prefix("supported_agents:") {
                    let inner = a.trim().trim_matches('[').trim_matches(']');
                    let agents: Vec<String> = inner
                        .split(',')
                        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    if !agents.is_empty() {
                        parsed.supported_agents = Some(agents);
                    }
                }
            }
        }
    }
    parsed
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillMetadata {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_directory: Option<String>,
    #[serde(default = "default_license")]
    pub license: String,
    #[serde(default = "default_security_status")]
    pub security_status: String,
    #[serde(default)]
    pub verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported_agents: Option<Vec<String>>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub stars: u64,
    pub registry: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl Default for SkillMetadata {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            slug: String::new(),
            description: String::new(),
            path: None,
            repository: None,
            skill_directory: None,
            license: default_license(),
            security_status: default_security_status(),
            verified: false,
            supported_agents: None,
            tags: vec![],
            stars: 0,
            registry: "local".to_string(),
            source: None,
        }
    }
}

fn default_license() -> String {
    "Apache-2.0".to_string()
}

fn default_security_status() -> String {
    "approved-local".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillContentReference {
    pub skill_id: String,
    pub raw_content: String,
    pub content_hash: String,
    pub path: PathBuf,
}

#[async_trait]
pub trait RegistryProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SkillMetadata>, RegistryError>;
    async fn get_metadata(&self, id: &str) -> Result<Option<SkillMetadata>, RegistryError>;
    async fn retrieve_content(&self, id: &str) -> Result<SkillContentReference, RegistryError>;
}

#[derive(Debug, Clone, Deserialize)]
struct ManifestFile {
    #[serde(default)]
    pub skills: Vec<ManifestSkill>,
}

#[derive(Debug, Clone, Deserialize)]
struct ManifestSkill {
    pub name: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub security_status: Option<String>,
}

pub struct LocalRegistry {
    pub root_dir: PathBuf,
    pub skills_dir: PathBuf,
    pub manifest_path: PathBuf,
}

impl LocalRegistry {
    pub fn new<P: AsRef<Path>>(root_dir: P) -> Self {
        let root = root_dir.as_ref().to_path_buf();
        let skills_dir = root.join("skills");
        let manifest_path = root.join("catalog").join("manifest.json");
        Self {
            root_dir: root,
            skills_dir,
            manifest_path,
        }
    }

    fn load_manifest(&self) -> HashMap<String, ManifestSkill> {
        let mut map = HashMap::new();
        if let Ok(content) = std::fs::read_to_string(&self.manifest_path) {
            if let Ok(manifest) = serde_json::from_str::<ManifestFile>(&content) {
                for s in manifest.skills {
                    map.insert(s.name.clone(), s);
                }
            }
        }
        map
    }

    fn parse_frontmatter(content: &str) -> (String, String, Option<Vec<String>>) {
        let mut name = String::new();
        let mut description = String::new();
        let mut supported_agents: Option<Vec<String>> = None;

        if let Some(rest) = content.strip_prefix("---") {
            if let Some(end_idx) = rest.find("\n---") {
                let fm = &rest[..end_idx];
                for line in fm.lines() {
                    let trimmed = line.trim();
                    if let Some(n) = trimmed.strip_prefix("name:") {
                        name = n.trim().trim_matches('"').trim_matches('\'').to_string();
                    } else if let Some(d) = trimmed.strip_prefix("description:") {
                        description = d.trim().trim_matches('"').trim_matches('\'').to_string();
                    } else if let Some(a) = trimmed.strip_prefix("supported_agents:") {
                        let inner = a.trim().trim_matches('[').trim_matches(']');
                        let agents: Vec<String> = inner
                            .split(',')
                            .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        if !agents.is_empty() {
                            supported_agents = Some(agents);
                        }
                    }
                }
            }
        }

        (name, description, supported_agents)
    }

    fn scan_dir(&self, dir: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    files.extend(self.scan_dir(&path));
                } else if path.is_file() {
                    if let Some(file_name) = path.file_name() {
                        if file_name.to_string_lossy().eq_ignore_ascii_case("skill.md") {
                            files.push(path);
                        }
                    }
                }
            }
        }
        files
    }

    pub fn get_all_skills(&self) -> Vec<SkillMetadata> {
        let manifest = self.load_manifest();
        let skill_files = self.scan_dir(&self.skills_dir);
        let mut skills = Vec::new();

        for file in skill_files {
            if let Ok(content) = std::fs::read_to_string(&file) {
                let (name, description, supported_agents) = Self::parse_frontmatter(&content);
                let dir_name = file
                    .parent()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                let id = if !name.is_empty() {
                    name.clone()
                } else {
                    dir_name.clone()
                };
                let manifest_entry = manifest.get(&id);

                let rel_path = pathdiff_relative(&file, &self.root_dir)
                    .unwrap_or_else(|| file.to_string_lossy().to_string())
                    .replace('\\', "/");

                skills.push(SkillMetadata {
                    id: id.clone(),
                    name: if !name.is_empty() {
                        name
                    } else {
                        dir_name.clone()
                    },
                    slug: id.clone(),
                    description,
                    path: Some(rel_path),
                    repository: Some("callmetechnophile/skills".to_string()),
                    skill_directory: Some(dir_name.clone()),
                    license: manifest_entry
                        .and_then(|m| m.license.clone())
                        .unwrap_or_else(|| "Apache-2.0".to_string()),
                    security_status: manifest_entry
                        .and_then(|m| m.security_status.clone())
                        .unwrap_or_else(|| "approved-local".to_string()),
                    verified: true,
                    supported_agents,
                    tags: vec!["local".to_string(), "core".to_string(), dir_name],
                    stars: 0,
                    registry: "local".to_string(),
                    source: manifest_entry
                        .and_then(|m| m.source.clone())
                        .or_else(|| Some("local".to_string())),
                });
            }
        }
        skills
    }
}

fn pathdiff_relative(target: &Path, base: &Path) -> Option<String> {
    if let Ok(diff) = target.strip_prefix(base) {
        Some(diff.to_string_lossy().to_string())
    } else {
        Some(target.to_string_lossy().to_string())
    }
}

#[async_trait]
impl RegistryProvider for LocalRegistry {
    fn name(&self) -> &str {
        "local"
    }

    async fn search(
        &self,
        query: &str,
        _limit: usize,
    ) -> Result<Vec<SkillMetadata>, RegistryError> {
        let all = self.get_all_skills();
        if query.trim().is_empty() {
            return Ok(all);
        }
        let q = query.to_lowercase();
        let filtered = all
            .into_iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&q)
                    || s.description.to_lowercase().contains(&q)
                    || s.slug.to_lowercase().contains(&q)
                    || s.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect();
        Ok(filtered)
    }

    async fn get_metadata(&self, id: &str) -> Result<Option<SkillMetadata>, RegistryError> {
        let all = self.get_all_skills();
        Ok(all
            .into_iter()
            .find(|s| s.id == id || s.name == id || s.slug == id))
    }

    async fn retrieve_content(&self, id: &str) -> Result<SkillContentReference, RegistryError> {
        let meta = self
            .get_metadata(id)
            .await?
            .ok_or_else(|| RegistryError::NotFound(id.to_string()))?;
        let rel_path = meta
            .path
            .ok_or_else(|| RegistryError::NotFound(id.to_string()))?;
        let full_path = self.root_dir.join(&rel_path);
        let content = std::fs::read_to_string(&full_path)?;
        let content_hash = compute_sha256(&content);
        Ok(SkillContentReference {
            skill_id: meta.id,
            raw_content: content,
            content_hash,
            path: full_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        let md = r#"---
name: "sort-the-issue"
description: "Decomposes complex issues"
supported_agents: ["antigravity", "claude-code"]
---
# Content here"#;
        let (name, desc, agents) = LocalRegistry::parse_frontmatter(md);
        assert_eq!(name, "sort-the-issue");
        assert_eq!(desc, "Decomposes complex issues");
        assert_eq!(
            agents,
            Some(vec!["antigravity".to_string(), "claude-code".to_string()])
        );
    }

    #[test]
    fn test_parse_frontmatter_metadata_version_and_license() {
        let md =
            "---\nname: my-skill\ndescription: test\nversion: \"2.1.0\"\nlicense: MIT\n---\nBody";
        let parsed = parse_frontmatter_metadata(md);
        assert_eq!(parsed.name, "my-skill");
        assert_eq!(parsed.version.as_deref(), Some("2.1.0"));
        assert_eq!(parsed.license.as_deref(), Some("MIT"));
    }

    #[test]
    fn test_canonical_directory_hashing_deterministic_and_ignores_temp() {
        let temp_dir = std::env::temp_dir().join(format!("test_dir_hash_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(temp_dir.join("scripts")).unwrap();
        std::fs::write(temp_dir.join("SKILL.md"), b"# Skill").unwrap();
        std::fs::write(temp_dir.join("scripts/run.sh"), b"echo hi").unwrap();
        std::fs::write(temp_dir.join("SKILL.md.tmp.123"), b"ignored").unwrap();

        let disk_hash = compute_directory_sha256(&temp_dir).unwrap();
        let entries_hash = compute_entries_sha256(&[
            ("scripts\\run.sh".to_string(), b"echo hi".to_vec()),
            ("SKILL.md".to_string(), b"# Skill".to_vec()),
        ]);
        assert_eq!(disk_hash, entries_hash);

        // Mutating a supporting file changes the hash
        std::fs::write(temp_dir.join("scripts/run.sh"), b"echo tampered").unwrap();
        let tampered_hash = compute_directory_sha256(&temp_dir).unwrap();
        assert_ne!(disk_hash, tampered_hash);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
