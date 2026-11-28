use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use skill_registry::{
    compute_sha256, RegistryError, RegistryProvider, SkillContentReference, SkillMetadata,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DiscoveryError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Directory resolution error ({code}): {message}")]
    Resolution { code: String, message: String },
    #[error("Registry error: {0}")]
    Registry(#[from] RegistryError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitTreeItem {
    pub path: String,
    #[serde(rename = "type")]
    pub item_type: String,
    #[serde(default)]
    pub sha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitTreeResponse {
    pub sha: Option<String>,
    #[serde(default)]
    pub tree: Vec<GitTreeItem>,
    pub truncated: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredSkillDirectory {
    pub skill_md_path: String,
    pub directory: String,
    pub basename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryResolutionResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact_directory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_command: Option<String>,
    #[serde(default)]
    pub matches: Vec<DiscoveredSkillDirectory>,
}

pub struct DirectoryResolver {
    #[allow(dead_code)]
    client: reqwest::Client,
    tree_cache: std::sync::RwLock<HashMap<String, Vec<GitTreeItem>>>,
}

impl Default for DirectoryResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectoryResolver {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("automatic-skill-router/0.3.0")
            .build()
            .unwrap_or_default();
        Self {
            client,
            tree_cache: std::sync::RwLock::new(HashMap::new()),
        }
    }

    pub fn discover_skill_directories_from_tree(
        &self,
        tree: &[GitTreeItem],
    ) -> Vec<DiscoveredSkillDirectory> {
        let mut skill_dirs = Vec::new();
        for item in tree {
            let path_lower = item.path.to_lowercase();
            if item.item_type == "blob"
                && (path_lower == "skill.md" || path_lower.ends_with("/skill.md"))
            {
                let dir_path = if path_lower == "skill.md" {
                    ".".to_string()
                } else {
                    let end_idx = item.path.len() - "/skill.md".len();
                    item.path[..end_idx].to_string()
                };

                let basename = if dir_path == "." {
                    "root".to_string()
                } else {
                    dir_path
                        .split('/')
                        .next_back()
                        .unwrap_or("root")
                        .to_string()
                };

                skill_dirs.push(DiscoveredSkillDirectory {
                    skill_md_path: item.path.clone(),
                    directory: dir_path,
                    basename,
                });
            }
        }
        skill_dirs
    }

    pub async fn fetch_repository_tree(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
        token: Option<&str>,
    ) -> Result<Vec<GitTreeItem>, DiscoveryError> {
        let cache_key = format!("{}/{}@{}", owner, repo, branch);
        if let Ok(cache) = self.tree_cache.read() {
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        let url = format!(
            "https://api.github.com/repos/{}/{}/git/trees/{}?recursive=1",
            owner, repo, branch
        );
        let mut req = self
            .client
            .get(&url)
            .header("Accept", "application/vnd.github.v3+json");
        let env_token = std::env::var("GITHUB_TOKEN").ok();
        let effective_token = token.or(env_token.as_deref());
        if let Some(t) = effective_token {
            req = req.header("Authorization", format!("Bearer {}", t));
        }

        let res = req.send().await?;
        if !res.status().is_success() {
            if branch == "main" && (res.status().as_u16() == 404 || res.status().as_u16() == 422) {
                return Box::pin(self.fetch_repository_tree(owner, repo, "master", token)).await;
            }
            return Err(DiscoveryError::Resolution {
                code: "TREE_FETCH_ERROR".to_string(),
                message: format!(
                    "Repository tree lookup failed for {}/{}: HTTP {}",
                    owner,
                    repo,
                    res.status()
                ),
            });
        }

        let body: GitTreeResponse = res.json().await?;
        if let Ok(mut cache) = self.tree_cache.write() {
            cache.insert(cache_key, body.tree.clone());
        }
        Ok(body.tree)
    }

    pub async fn resolve_skill_directory(
        &self,
        repository: &str,
        target_skill: &str,
        tree_fixture: Option<&[GitTreeItem]>,
        token: Option<&str>,
    ) -> DirectoryResolutionResult {
        let parts: Vec<&str> = repository.split('/').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return DirectoryResolutionResult {
                success: false,
                error: Some("INVALID_REPOSITORY".to_string()),
                message: Some(format!(
                    "Invalid repository format '{}'. Expected owner/name.",
                    repository
                )),
                exact_directory: None,
                install_command: None,
                matches: vec![],
            };
        }

        let (owner, repo) = (parts[0], parts[1]);
        let tree = match tree_fixture {
            Some(t) => t.to_vec(),
            None => match self.fetch_repository_tree(owner, repo, "main", token).await {
                Ok(t) => t,
                Err(e) => {
                    return DirectoryResolutionResult {
                        success: false,
                        error: Some("TREE_FETCH_ERROR".to_string()),
                        message: Some(e.to_string()),
                        exact_directory: None,
                        install_command: None,
                        matches: vec![],
                    };
                }
            },
        };

        let skill_dirs = self.discover_skill_directories_from_tree(&tree);
        let norm_target = target_skill.to_lowercase().trim().to_string();

        let matched: Vec<DiscoveredSkillDirectory> = skill_dirs
            .into_iter()
            .filter(|d| {
                d.directory.to_lowercase() == norm_target
                    || d.basename.to_lowercase() == norm_target
                    || d.directory
                        .to_lowercase()
                        .ends_with(&format!("/{}", norm_target))
            })
            .collect();

        if matched.is_empty() {
            return DirectoryResolutionResult {
                success: false,
                error: Some("SKILL_NOT_FOUND".to_string()),
                message: Some(format!(
                    "No directory containing SKILL.md matching '{}' found in {}/{}",
                    target_skill, owner, repo
                )),
                exact_directory: None,
                install_command: None,
                matches: vec![],
            };
        }

        if matched.len() > 1 {
            return DirectoryResolutionResult {
                success: false,
                error: Some("AMBIGUOUS_MATCH".to_string()),
                message: Some(format!(
                    "Multiple directories matching '{}' found in {}/{}. Specify exact directory.",
                    target_skill, owner, repo
                )),
                exact_directory: None,
                install_command: None,
                matches: matched,
            };
        }

        let exact = &matched[0].directory;
        DirectoryResolutionResult {
            success: true,
            error: None,
            message: None,
            exact_directory: Some(exact.clone()),
            install_command: Some(format!(
                "npm run install:skill -- --repo {}/{} --skill {}",
                owner, repo, exact
            )),
            matches: matched,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncCatalogResult {
    #[serde(rename = "totalIndexed")]
    pub total_indexed: usize,
    #[serde(rename = "syncedPages")]
    pub synced_pages: usize,
    pub complete: bool,
    #[serde(rename = "nextOffset")]
    pub next_offset: usize,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsDirectoryIndex {
    pub source: String,
    pub fetched_at: Option<String>,
    pub total: Option<usize>,
    pub next_offset: usize,
    pub complete: bool,
    #[serde(default)]
    pub skills: Vec<SkillMetadata>,
}

pub struct SkillsDirectoryRegistry {
    pub root_dir: PathBuf,
    pub index_path: PathBuf,
    pub endpoint: String,
    pub no_network: bool,
    #[allow(dead_code)]
    client: reqwest::Client,
}

impl SkillsDirectoryRegistry {
    pub fn new<P: AsRef<Path>>(root_dir: P, no_network: bool) -> Self {
        let root = root_dir.as_ref().to_path_buf();
        let index_path = root.join("catalog").join("skillsdirectory-index.json");
        Self {
            root_dir: root,
            index_path,
            endpoint: "https://www.skillsdirectory.com/api/registry".to_string(),
            no_network,
            client: reqwest::Client::builder()
                .user_agent("automatic-skill-router/0.3.0")
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn load_index(&self) -> SkillsDirectoryIndex {
        if let Ok(content) = std::fs::read_to_string(&self.index_path) {
            if let Ok(idx) = serde_json::from_str::<SkillsDirectoryIndex>(&content) {
                return idx;
            }
        }

        // Fallback: load catalog/manifest.json if present
        let manifest_path = self.root_dir.join("catalog").join("manifest.json");
        if let Ok(content) = std::fs::read_to_string(&manifest_path) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(items) = val.get("skills").and_then(|s| s.as_array()) {
                    let mut skills = Vec::new();
                    for item in items {
                        let name = item
                            .get("name")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string();
                        let path = item
                            .get("path")
                            .and_then(|s| s.as_str())
                            .map(|s| s.to_string());
                        let source = item
                            .get("source")
                            .and_then(|s| s.as_str())
                            .map(|s| s.to_string());
                        let license = item
                            .get("license")
                            .and_then(|s| s.as_str())
                            .unwrap_or("Apache-2.0")
                            .to_string();
                        let sec = item
                            .get("security_status")
                            .and_then(|s| s.as_str())
                            .unwrap_or("approved-local")
                            .to_string();

                        skills.push(SkillMetadata {
                            id: name.clone(),
                            name: name.clone(),
                            slug: name.to_lowercase(),
                            description: String::new(),
                            path,
                            source,
                            license,
                            security_status: sec,
                            registry: "catalog-manifest".to_string(),
                            ..Default::default()
                        });
                    }

                    return SkillsDirectoryIndex {
                        source: self.endpoint.clone(),
                        fetched_at: None,
                        total: Some(skills.len()),
                        next_offset: 0,
                        complete: false,
                        skills,
                    };
                }
            }
        }

        SkillsDirectoryIndex {
            source: self.endpoint.clone(),
            fetched_at: None,
            total: None,
            next_offset: 0,
            complete: false,
            skills: vec![],
        }
    }

    pub async fn sync_catalog(
        &self,
        limit: usize,
        reset: bool,
    ) -> Result<SyncCatalogResult, DiscoveryError> {
        let mut index = if reset {
            SkillsDirectoryIndex {
                source: self.endpoint.clone(),
                fetched_at: None,
                total: None,
                next_offset: 0,
                complete: false,
                skills: vec![],
            }
        } else {
            self.load_index()
        };

        let page_size = std::cmp::min(limit, 100);
        let mut pages = 0;
        let mut offset = index.next_offset;
        let max_pages = limit.div_ceil(page_size);

        if !self.no_network && max_pages > 0 {
            while pages < max_pages {
                let url = format!("{}?offset={}&limit={}", self.endpoint, offset, page_size);
                match self
                    .client
                    .get(&url)
                    .header("Accept", "application/json")
                    .send()
                    .await
                {
                    Ok(res) if res.status().is_success() => {
                        if let Ok(payload) = res.json::<serde_json::Value>().await {
                            if let Some(items) = payload.get("skills").and_then(|s| s.as_array()) {
                                if items.is_empty() {
                                    index.complete = true;
                                    break;
                                }
                                for item in items {
                                    let slug = item
                                        .get("slug")
                                        .and_then(|s| s.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    if !slug.is_empty()
                                        && !index.skills.iter().any(|s| s.slug == slug)
                                    {
                                        let name = item
                                            .get("name")
                                            .and_then(|s| s.as_str())
                                            .unwrap_or(&slug)
                                            .to_string();
                                        let description = item
                                            .get("description")
                                            .and_then(|s| s.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        let repo = item
                                            .get("repository")
                                            .and_then(|s| s.as_str())
                                            .map(|s| s.to_string());
                                        let skill_dir = item
                                            .get("skill_directory")
                                            .and_then(|s| s.as_str())
                                            .map(|s| s.to_string());
                                        index.skills.push(SkillMetadata {
                                            id: format!("sd:{}", slug),
                                            name,
                                            slug,
                                            description,
                                            repository: repo,
                                            skill_directory: skill_dir,
                                            registry: "skillsdirectory".to_string(),
                                            ..Default::default()
                                        });
                                    }
                                }
                                offset += items.len();
                                pages += 1;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    _ => break,
                }
            }
        }

        index.next_offset = offset;
        index.fetched_at = Some(chrono::Utc::now().to_rfc3339());
        index.total = Some(index.skills.len());

        // Atomic write
        let parent = self.index_path.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;
        let tmp_path = parent.join(format!(".catalog.tmp.{}", uuid::Uuid::new_v4()));
        let json = serde_json::to_string_pretty(&index)? + "\n";
        std::fs::write(&tmp_path, json)?;
        std::fs::rename(&tmp_path, &self.index_path)?;

        Ok(SyncCatalogResult {
            total_indexed: index.skills.len(),
            synced_pages: pages,
            complete: index.complete,
            next_offset: index.next_offset,
            source: self.endpoint.clone(),
        })
    }
}

#[async_trait]
impl RegistryProvider for SkillsDirectoryRegistry {
    fn name(&self) -> &str {
        "skillsdirectory"
    }

    async fn search(
        &self,
        query: &str,
        _limit: usize,
    ) -> Result<Vec<SkillMetadata>, RegistryError> {
        let index = self.load_index();
        if query.trim().is_empty() {
            return Ok(index.skills);
        }
        let q = query.to_lowercase();
        let filtered = index
            .skills
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
        let index = self.load_index();
        Ok(index
            .skills
            .into_iter()
            .find(|s| s.id == id || s.name == id || s.slug == id))
    }

    async fn retrieve_content(&self, id: &str) -> Result<SkillContentReference, RegistryError> {
        Err(RegistryError::Custom(format!(
            "SkillsDirectory skill '{}' is remote metadata-only; must be installed before content retrieval.",
            id
        )))
    }
}

pub struct InstalledRegistry {
    pub root_dir: PathBuf,
    pub ledger_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledLedgerEntry {
    #[serde(rename = "skillId")]
    pub skill_id: String,
    pub name: String,
    pub version: Option<String>,
    pub source: String,
    #[serde(rename = "commitSha")]
    pub commit_sha: Option<String>,
    pub license: Option<String>,
    #[serde(rename = "installationPath")]
    pub installation_path: Option<String>,
    #[serde(rename = "targetAgent")]
    pub target_agent: Option<String>,
    #[serde(rename = "contentHash")]
    pub content_hash: Option<String>,
    #[serde(rename = "trustStatus")]
    pub trust_status: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstalledLedgerFile {
    #[serde(default)]
    pub installed: Vec<InstalledLedgerEntry>,
}

impl InstalledRegistry {
    pub fn new<P: AsRef<Path>>(root_dir: P) -> Self {
        let root = root_dir.as_ref().to_path_buf();
        let ledger_path = root.join(".installed-skills.json");
        Self {
            root_dir: root,
            ledger_path,
        }
    }

    pub fn load_ledger(&self) -> Vec<InstalledLedgerEntry> {
        if let Ok(content) = std::fs::read_to_string(&self.ledger_path) {
            if let Ok(parsed) = serde_json::from_str::<InstalledLedgerFile>(&content) {
                return parsed.installed;
            }
        }
        vec![]
    }
}

#[async_trait]
impl RegistryProvider for InstalledRegistry {
    fn name(&self) -> &str {
        "installed"
    }

    async fn search(
        &self,
        query: &str,
        _limit: usize,
    ) -> Result<Vec<SkillMetadata>, RegistryError> {
        let list = self.load_ledger();
        let q = query.to_lowercase().trim().to_string();
        let mut results = Vec::new();

        for entry in list {
            if q.is_empty()
                || entry.name.to_lowercase().contains(&q)
                || entry.skill_id.to_lowercase().contains(&q)
                || entry
                    .description
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&q)
            {
                results.push(SkillMetadata {
                    id: entry.skill_id.clone(),
                    name: entry.name.clone(),
                    slug: entry.skill_id.clone(),
                    description: entry.description.clone().unwrap_or_default(),
                    path: entry.installation_path.clone(),
                    repository: Some(entry.source.clone()),
                    skill_directory: None,
                    license: entry.license.unwrap_or_else(|| "Unknown".to_string()),
                    security_status: entry.trust_status.unwrap_or_else(|| "APPROVED".to_string()),
                    verified: true,
                    supported_agents: entry.target_agent.map(|a| vec![a]),
                    tags: vec!["installed".to_string()],
                    stars: 0,
                    registry: "installed".to_string(),
                    source: Some(entry.source),
                });
            }
        }
        Ok(results)
    }

    async fn get_metadata(&self, id: &str) -> Result<Option<SkillMetadata>, RegistryError> {
        let all = self.search("", usize::MAX).await?;
        Ok(all.into_iter().find(|s| s.id == id || s.name == id))
    }

    async fn retrieve_content(&self, id: &str) -> Result<SkillContentReference, RegistryError> {
        let meta = self
            .get_metadata(id)
            .await?
            .ok_or_else(|| RegistryError::NotFound(id.to_string()))?;
        let rel_path = meta
            .path
            .ok_or_else(|| RegistryError::NotFound(id.to_string()))?;
        let full_path = self.root_dir.join(&rel_path).join("SKILL.md");
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
    fn test_tree_directory_resolution() {
        let resolver = DirectoryResolver::new();
        let fixture = vec![
            GitTreeItem {
                path: "skills/core/sort-the-issue/SKILL.md".to_string(),
                item_type: "blob".to_string(),
                sha: "123".to_string(),
            },
            GitTreeItem {
                path: "skills/utils/helper/SKILL.md".to_string(),
                item_type: "blob".to_string(),
                sha: "456".to_string(),
            },
        ];

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let res = resolver
                .resolve_skill_directory("owner/repo", "sort-the-issue", Some(&fixture), None)
                .await;
            assert!(res.success);
            assert_eq!(
                res.exact_directory,
                Some("skills/core/sort-the-issue".to_string())
            );
            assert!(res
                .install_command
                .unwrap()
                .contains("--skill skills/core/sort-the-issue"));

            let not_found = resolver
                .resolve_skill_directory("owner/repo", "non-existent", Some(&fixture), None)
                .await;
            assert!(!not_found.success);
            assert_eq!(not_found.error, Some("SKILL_NOT_FOUND".to_string()));
        });
    }
}
