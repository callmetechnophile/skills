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
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
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
}
