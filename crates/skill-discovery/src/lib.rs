use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use skill_registry::{
    compute_directory_sha256, compute_sha256, RegistryError, RegistryProvider,
    SkillContentReference, SkillMetadata,
};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const MAX_SYNC_LIMIT: usize = 10_000;

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
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Upstream HTTP error ({status}): {message}")]
    UpstreamHttp { status: u16, message: String },
    #[error("Invalid upstream schema: {0}")]
    InvalidSchema(String),
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
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub mode: Option<String>,
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
    client: reqwest::Client,
    pub github_api_base: String,
    pub no_network: bool,
    tree_cache: std::sync::RwLock<HashMap<String, Vec<GitTreeItem>>>,
}

impl Default for DirectoryResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectoryResolver {
    pub fn new() -> Self {
        Self::with_config("https://api.github.com", 15, false)
    }

    pub fn with_api_base(api_base: impl Into<String>) -> Self {
        Self::with_config(api_base, 15, false)
    }

    pub fn with_config(api_base: impl Into<String>, timeout_secs: u64, no_network: bool) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("automatic-skill-router/0.3.2")
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
            .unwrap_or_default();
        Self {
            client,
            github_api_base: api_base.into().trim_end_matches('/').to_string(),
            no_network,
            tree_cache: std::sync::RwLock::new(HashMap::new()),
        }
    }

    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.tree_cache.write() {
            cache.clear();
        }
    }

    pub fn invalidate_cache(&self, owner: &str, repo: &str, branch: &str) {
        let cache_key = format!("{}/{}@{}", owner, repo, branch);
        if let Ok(mut cache) = self.tree_cache.write() {
            cache.remove(&cache_key);
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

        if self.no_network {
            return Err(DiscoveryError::Resolution {
                code: "NO_NETWORK".to_string(),
                message: format!(
                    "Network requests are disabled (--no-network); cannot fetch tree for {}/{}@{}.",
                    owner, repo, branch
                ),
            });
        }

        let url = format!(
            "{}/repos/{}/{}/git/trees/{}?recursive=1",
            self.github_api_base, owner, repo, branch
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

        let res = req.send().await.map_err(|e| {
            let code = if e.is_timeout() {
                "TIMEOUT"
            } else {
                "TREE_FETCH_ERROR"
            };
            DiscoveryError::Resolution {
                code: code.to_string(),
                message: format!(
                    "Repository tree request failed for {}/{}@{}: {}",
                    owner, repo, branch, e
                ),
            }
        })?;

        let status = res.status().as_u16();
        if !res.status().is_success() {
            if status == 429
                || (status == 403
                    && res
                        .headers()
                        .get("x-ratelimit-remaining")
                        .and_then(|v| v.to_str().ok())
                        == Some("0"))
            {
                return Err(DiscoveryError::Resolution {
                    code: "RATE_LIMITED".to_string(),
                    message: format!(
                        "GitHub API rate limit exceeded when fetching tree for {}/{}@{} (HTTP {}).",
                        owner, repo, branch, status
                    ),
                });
            }
            if status == 401 || status == 403 {
                return Err(DiscoveryError::Resolution {
                    code: "AUTH_ERROR".to_string(),
                    message: format!(
                        "Authentication or authorization failed for {}/{}@{} (HTTP {}).",
                        owner, repo, branch, status
                    ),
                });
            }
            if branch == "main" && (status == 404 || status == 422) {
                return Box::pin(self.fetch_repository_tree(owner, repo, "master", token)).await;
            }
            return Err(DiscoveryError::Resolution {
                code: "TREE_FETCH_ERROR".to_string(),
                message: format!(
                    "Repository tree lookup failed for {}/{}@{}: HTTP {}",
                    owner, repo, branch, status
                ),
            });
        }

        let body: GitTreeResponse = res.json().await.map_err(|e| DiscoveryError::Resolution {
            code: "MALFORMED_RESPONSE".to_string(),
            message: format!(
                "Invalid JSON in repository tree response for {}/{}@{}: {}",
                owner, repo, branch, e
            ),
        })?;

        if body.truncated == Some(true) {
            // Never cache truncated trees
            return Err(DiscoveryError::Resolution {
                code: "TREE_TRUNCATED".to_string(),
                message: format!(
                    "Repository tree for {}/{}@{} was truncated by GitHub API; refusing incomplete discovery.",
                    owner, repo, branch
                ),
            });
        }

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
        let (result, _) = self
            .resolve_skill_directory_with_tree(repository, target_skill, tree_fixture, token)
            .await;
        result
    }

    pub async fn resolve_skill_directory_with_tree(
        &self,
        repository: &str,
        target_skill: &str,
        tree_fixture: Option<&[GitTreeItem]>,
        token: Option<&str>,
    ) -> (DirectoryResolutionResult, Option<Vec<GitTreeItem>>) {
        let parts: Vec<&str> = repository.split('/').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return (
                DirectoryResolutionResult {
                    success: false,
                    error: Some("INVALID_REPOSITORY".to_string()),
                    message: Some(format!(
                        "Invalid repository format '{}'. Expected owner/name.",
                        repository
                    )),
                    exact_directory: None,
                    install_command: None,
                    matches: vec![],
                },
                None,
            );
        }

        let (owner, repo) = (parts[0], parts[1]);
        let tree = match tree_fixture {
            Some(t) => t.to_vec(),
            None => match self.fetch_repository_tree(owner, repo, "main", token).await {
                Ok(t) => t,
                Err(DiscoveryError::Resolution { code, message }) => {
                    return (
                        DirectoryResolutionResult {
                            success: false,
                            error: Some(code),
                            message: Some(message),
                            exact_directory: None,
                            install_command: None,
                            matches: vec![],
                        },
                        None,
                    );
                }
                Err(e) => {
                    return (
                        DirectoryResolutionResult {
                            success: false,
                            error: Some("TREE_FETCH_ERROR".to_string()),
                            message: Some(e.to_string()),
                            exact_directory: None,
                            install_command: None,
                            matches: vec![],
                        },
                        None,
                    );
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
            return (
                DirectoryResolutionResult {
                    success: false,
                    error: Some("SKILL_NOT_FOUND".to_string()),
                    message: Some(format!(
                        "No directory containing SKILL.md matching '{}' found in {}/{}",
                        target_skill, owner, repo
                    )),
                    exact_directory: None,
                    install_command: None,
                    matches: vec![],
                },
                Some(tree),
            );
        }

        if matched.len() > 1 {
            return (
                DirectoryResolutionResult {
                    success: false,
                    error: Some("AMBIGUOUS_MATCH".to_string()),
                    message: Some(format!(
                        "Multiple directories matching '{}' found in {}/{}. Specify exact directory.",
                        target_skill, owner, repo
                    )),
                    exact_directory: None,
                    install_command: None,
                    matches: matched,
                },
                Some(tree),
            );
        }

        let exact = &matched[0].directory;
        (
            DirectoryResolutionResult {
                success: true,
                error: None,
                message: None,
                exact_directory: Some(exact.clone()),
                install_command: Some(format!(
                    "skill-router install-skill --repo {}/{} --skill {} --execute",
                    owner, repo, exact
                )),
                matches: matched,
            },
            Some(tree),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncCatalogResult {
    #[serde(rename = "totalIndexed")]
    pub total_indexed: usize,
    #[serde(rename = "syncedPages")]
    pub synced_pages: usize,
    pub complete: bool,
    pub partial: bool,
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
    client: reqwest::Client,
}

impl SkillsDirectoryRegistry {
    pub fn new<P: AsRef<Path>>(root_dir: P, no_network: bool) -> Self {
        Self::with_endpoint(
            root_dir,
            no_network,
            "https://www.skillsdirectory.com/api/registry",
            15,
        )
    }

    pub fn with_endpoint<P: AsRef<Path>>(
        root_dir: P,
        no_network: bool,
        endpoint: impl Into<String>,
        timeout_secs: u64,
    ) -> Self {
        let root = root_dir.as_ref().to_path_buf();
        let index_path = root.join("catalog").join("skillsdirectory-index.json");
        Self {
            root_dir: root,
            index_path,
            endpoint: endpoint.into(),
            no_network,
            client: reqwest::Client::builder()
                .user_agent("automatic-skill-router/0.3.2")
                .timeout(std::time::Duration::from_secs(timeout_secs))
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
                            .unwrap_or("UNSPECIFIED")
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
        if limit == 0 {
            return Err(DiscoveryError::InvalidInput(
                "limit must be greater than 0".to_string(),
            ));
        }
        if limit > MAX_SYNC_LIMIT {
            return Err(DiscoveryError::InvalidInput(format!(
                "limit {} exceeds maximum allowed {}",
                limit, MAX_SYNC_LIMIT
            )));
        }

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

        if self.no_network {
            if reset {
                self.save_index_atomically(&index)?;
            }
            return Ok(SyncCatalogResult {
                total_indexed: index.skills.len(),
                synced_pages: 0,
                complete: index.complete,
                partial: !index.complete,
                next_offset: index.next_offset,
                source: self.endpoint.clone(),
            });
        }

        let mut seen_slugs: HashSet<String> = index.skills.iter().map(|s| s.slug.clone()).collect();
        let mut pages = 0usize;
        let mut offset = index.next_offset;
        let mut fetched_items_total = 0usize;

        while fetched_items_total < limit {
            let remaining = limit - fetched_items_total;
            let page_size = std::cmp::min(remaining, 100);
            let url = format!("{}?offset={}&limit={}", self.endpoint, offset, page_size);

            let res = self
                .client
                .get(&url)
                .header("Accept", "application/json")
                .send()
                .await?;

            let status = res.status().as_u16();
            if !res.status().is_success() {
                return Err(DiscoveryError::UpstreamHttp {
                    status,
                    message: format!("Catalog endpoint returned HTTP {}", status),
                });
            }

            let payload: serde_json::Value = res.json().await.map_err(|e| {
                DiscoveryError::InvalidSchema(format!("Malformed JSON from catalog: {}", e))
            })?;

            let items = payload
                .get("skills")
                .and_then(|s| s.as_array())
                .ok_or_else(|| {
                    DiscoveryError::InvalidSchema(
                        "Catalog response is missing required 'skills' array".to_string(),
                    )
                })?;

            if items.is_empty() {
                index.complete = true;
                break;
            }

            for item in items {
                let obj = item.as_object().ok_or_else(|| {
                    DiscoveryError::InvalidSchema("Malformed skill entry: expected object".into())
                })?;

                let raw_slug = obj
                    .get("slug")
                    .and_then(|s| s.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());
                let raw_name = obj
                    .get("name")
                    .and_then(|s| s.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());

                let (slug, name) = match (raw_slug, raw_name) {
                    (Some(s), Some(n)) => (s, n),
                    (Some(s), None) => (s.clone(), s),
                    (None, Some(n)) => {
                        let derived: String = n
                            .to_lowercase()
                            .chars()
                            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                            .collect();
                        (derived.trim_matches('-').to_string(), n)
                    }
                    (None, None) => {
                        return Err(DiscoveryError::InvalidSchema(
                            "Malformed skill entry: missing both 'slug' and 'name'".to_string(),
                        ));
                    }
                };

                if slug.is_empty() {
                    return Err(DiscoveryError::InvalidSchema(
                        "Malformed skill entry: empty slug".to_string(),
                    ));
                }

                if seen_slugs.insert(slug.clone()) {
                    let description = obj
                        .get("description")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string();
                    let repo = obj
                        .get("repository")
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string());
                    let skill_dir = obj
                        .get("skill_directory")
                        .or_else(|| obj.get("skillDirectory"))
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string());
                    let license = obj
                        .get("license")
                        .and_then(|s| s.as_str())
                        .unwrap_or("UNSPECIFIED")
                        .to_string();
                    let stars = obj.get("stars").and_then(|s| s.as_u64()).unwrap_or(0);
                    let verified = obj
                        .get("verified")
                        .and_then(|s| s.as_bool())
                        .unwrap_or(false);

                    index.skills.push(SkillMetadata {
                        id: format!("sd:{}", slug),
                        name,
                        slug,
                        description,
                        repository: repo,
                        skill_directory: skill_dir,
                        license,
                        stars,
                        verified,
                        registry: "skillsdirectory".to_string(),
                        ..Default::default()
                    });
                }
            }

            offset = offset.checked_add(items.len()).ok_or_else(|| {
                DiscoveryError::InvalidSchema("Pagination offset overflow".to_string())
            })?;
            fetched_items_total += items.len();
            pages += 1;

            if items.len() < page_size {
                index.complete = true;
                break;
            }
        }

        index.next_offset = offset;
        index.fetched_at = Some(chrono::Utc::now().to_rfc3339());
        index.total = Some(index.skills.len());

        self.save_index_atomically(&index)?;

        Ok(SyncCatalogResult {
            total_indexed: index.skills.len(),
            synced_pages: pages,
            complete: index.complete,
            partial: !index.complete,
            next_offset: index.next_offset,
            source: self.endpoint.clone(),
        })
    }

    fn save_index_atomically(&self, index: &SkillsDirectoryIndex) -> Result<(), DiscoveryError> {
        let parent = self.index_path.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;
        let tmp_path = parent.join(format!(".catalog.tmp.{}", uuid::Uuid::new_v4()));
        let json = serde_json::to_string_pretty(index)? + "\n";

        let write_res = (|| -> std::io::Result<()> {
            let mut file = std::fs::File::create(&tmp_path)?;
            file.write_all(json.as_bytes())?;
            file.flush()?;
            file.sync_all()?;
            Ok(())
        })();

        if let Err(e) = write_res {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(DiscoveryError::Io(e));
        }

        if let Err(e) = std::fs::rename(&tmp_path, &self.index_path) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(DiscoveryError::Io(e));
        }
        Ok(())
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

    pub fn get_ledger_entry(&self, id: &str) -> Option<InstalledLedgerEntry> {
        self.load_ledger()
            .into_iter()
            .find(|e| e.skill_id == id || e.name == id)
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
                    license: entry.license.unwrap_or_else(|| "UNSPECIFIED".to_string()),
                    security_status: entry
                        .trust_status
                        .unwrap_or_else(|| "UNSCANNED".to_string()),
                    verified: entry.content_hash.is_some(),
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
        let entry = self
            .get_ledger_entry(id)
            .ok_or_else(|| RegistryError::NotFound(id.to_string()))?;
        let rel_path = entry
            .installation_path
            .ok_or_else(|| RegistryError::NotFound(id.to_string()))?;
        let skill_dir = self.root_dir.join(&rel_path);
        let full_path = skill_dir.join("SKILL.md");

        // Verify complete directory integrity before loading if ledger has a content_hash
        if let Some(expected_hash) = entry.content_hash {
            let actual_dir_hash = compute_directory_sha256(&skill_dir).map_err(|e| {
                RegistryError::Custom(format!(
                    "Failed to compute installed directory hash for '{}': {}",
                    id, e
                ))
            })?;
            if actual_dir_hash != expected_hash {
                return Err(RegistryError::Custom(format!(
                    "HASH_MISMATCH: Installed skill directory '{}' failed integrity verification (expected {}, got {}).",
                    id, expected_hash, actual_dir_hash
                )));
            }
        }

        let content = std::fs::read_to_string(&full_path)?;
        let content_hash = compute_sha256(&content);
        Ok(SkillContentReference {
            skill_id: entry.skill_id,
            raw_content: content,
            content_hash,
            path: full_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn spawn_mock_server<F>(handler: F) -> String
    where
        F: Fn(&str) -> (u16, String, String) + Send + Sync + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handler = std::sync::Arc::new(handler);
        tokio::spawn(async move {
            loop {
                if let Ok((mut socket, _)) = listener.accept().await {
                    let h = handler.clone();
                    tokio::spawn(async move {
                        let mut buf = [0u8; 4096];
                        let n = socket.read(&mut buf).await.unwrap_or(0);
                        let req_str = String::from_utf8_lossy(&buf[..n]);
                        let first_line = req_str.lines().next().unwrap_or("");
                        let path = first_line.split_whitespace().nth(1).unwrap_or("/");
                        let (status, headers, body) = h(path);
                        let resp = format!(
                            "HTTP/1.1 {} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{}Connection: close\r\n\r\n{}",
                            status,
                            body.len(),
                            headers,
                            body
                        );
                        let _ = socket.write_all(resp.as_bytes()).await;
                    });
                }
            }
        });
        format!("http://{}", addr)
    }

    #[test]
    fn test_tree_directory_resolution() {
        let resolver = DirectoryResolver::new();
        let fixture = vec![
            GitTreeItem {
                path: "skills/core/sort-the-issue/SKILL.md".to_string(),
                item_type: "blob".to_string(),
                sha: "123".to_string(),
                size: None,
                mode: None,
            },
            GitTreeItem {
                path: "skills/utils/helper/SKILL.md".to_string(),
                item_type: "blob".to_string(),
                sha: "456".to_string(),
                size: None,
                mode: None,
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

    #[tokio::test]
    async fn test_tree_resolution_complete_truncated_empty_ambiguous_and_fallbacks() {
        let base = spawn_mock_server(|path| {
            if path.contains("/repos/good/repo/git/trees/main") {
                (
                    200,
                    String::new(),
                    r#"{"sha":"abc","truncated":false,"tree":[{"path":"skills/alpha/SKILL.md","type":"blob","sha":"1"}]}"#.to_string(),
                )
            } else if path.contains("/repos/trunc/repo/git/trees/main") {
                (
                    200,
                    String::new(),
                    r#"{"sha":"abc","truncated":true,"tree":[{"path":"skills/alpha/SKILL.md","type":"blob","sha":"1"}]}"#.to_string(),
                )
            } else if path.contains("/repos/empty/repo/git/trees/main") {
                (200, String::new(), r#"{"sha":"abc","truncated":false,"tree":[]}"#.to_string())
            } else if path.contains("/repos/masteronly/repo/git/trees/main") {
                (404, String::new(), r#"{"message":"Not Found"}"#.to_string())
            } else if path.contains("/repos/masteronly/repo/git/trees/master") {
                (
                    200,
                    String::new(),
                    r#"{"sha":"def","truncated":false,"tree":[{"path":"skills/beta/SKILL.md","type":"blob","sha":"2"}]}"#.to_string(),
                )
            } else if path.contains("/repos/ratelimit/repo/git/trees/main") {
                (
                    429,
                    String::new(),
                    r#"{"message":"rate limit"}"#.to_string(),
                )
            } else {
                (404, String::new(), r#"{"message":"Not Found"}"#.to_string())
            }
        })
        .await;

        let resolver = DirectoryResolver::with_api_base(&base);

        // 1. Complete tree & cache hit
        let ok = resolver
            .resolve_skill_directory("good/repo", "alpha", None, None)
            .await;
        assert!(ok.success);
        assert_eq!(ok.exact_directory.as_deref(), Some("skills/alpha"));

        // Cache hit works even with invalidation check
        let ok_cached = resolver
            .resolve_skill_directory("good/repo", "alpha", None, None)
            .await;
        assert!(ok_cached.success);
        resolver.invalidate_cache("good", "repo", "main");

        // 2. Truncated tree rejected and not cached
        let trunc = resolver
            .resolve_skill_directory("trunc/repo", "alpha", None, None)
            .await;
        assert!(!trunc.success);
        assert_eq!(trunc.error.as_deref(), Some("TREE_TRUNCATED"));

        // 3. Empty tree -> SKILL_NOT_FOUND
        let empty = resolver
            .resolve_skill_directory("empty/repo", "alpha", None, None)
            .await;
        assert!(!empty.success);
        assert_eq!(empty.error.as_deref(), Some("SKILL_NOT_FOUND"));

        // 4. Master fallback
        let master = resolver
            .resolve_skill_directory("masteronly/repo", "beta", None, None)
            .await;
        assert!(master.success);
        assert_eq!(master.exact_directory.as_deref(), Some("skills/beta"));

        // 5. Rate limiting
        let rl = resolver
            .resolve_skill_directory("ratelimit/repo", "alpha", None, None)
            .await;
        assert!(!rl.success);
        assert_eq!(rl.error.as_deref(), Some("RATE_LIMITED"));

        // 6. Invalid repo
        let inv = resolver
            .resolve_skill_directory("badrepo", "alpha", None, None)
            .await;
        assert!(!inv.success);
        assert_eq!(inv.error.as_deref(), Some("INVALID_REPOSITORY"));

        // 7. Ambiguous directory
        let amb_fixture = vec![
            GitTreeItem {
                path: "a/dup/SKILL.md".to_string(),
                item_type: "blob".to_string(),
                sha: "1".to_string(),
                size: None,
                mode: None,
            },
            GitTreeItem {
                path: "b/dup/SKILL.md".to_string(),
                item_type: "blob".to_string(),
                sha: "2".to_string(),
                size: None,
                mode: None,
            },
        ];
        let amb = resolver
            .resolve_skill_directory("good/repo", "dup", Some(&amb_fixture), None)
            .await;
        assert!(!amb.success);
        assert_eq!(amb.error.as_deref(), Some("AMBIGUOUS_MATCH"));
    }

    #[tokio::test]
    async fn test_sync_catalog_validation_pagination_and_error_preservation() {
        let base = spawn_mock_server(|path| {
            if path.starts_with("/ok?offset=0") {
                (
                    200,
                    String::new(),
                    r#"{"skills":[{"slug":"s1","name":"Skill One"},{"slug":"s1","name":"Duplicate"},{"slug":"s2","name":"Skill Two"}]}"#.to_string(),
                )
            } else if path.starts_with("/paged?offset=0&limit=100") {
                // Return 100 items on page 1? Let's test limit=101 -> page 1 has 100, page 2 has 0
                let items: Vec<String> = (0..100)
                    .map(|i| format!(r#"{{"slug":"p{}","name":"P{}"}}"#, i, i))
                    .collect();
                (200, String::new(), format!(r#"{{"skills":[{}]}}"#, items.join(",")))
            } else if path.starts_with("/paged?offset=100") {
                (200, String::new(), r#"{"skills":[]}"#.to_string())
            } else if path.starts_with("/malformed") {
                (200, String::new(), r#"{not valid json"#.to_string())
            } else if path.starts_with("/noskills") {
                (200, String::new(), r#"{"items":[]}"#.to_string())
            } else if path.starts_with("/404") {
                (404, String::new(), r#"{"error":"not found"}"#.to_string())
            } else {
                (429, String::new(), r#"{"error":"rate limited"}"#.to_string())
            }
        })
        .await;

        let temp_dir = std::env::temp_dir().join(format!("test_sync_cat_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Zero limit rejected
        let reg =
            SkillsDirectoryRegistry::with_endpoint(&temp_dir, false, format!("{}/ok", base), 5);
        assert!(matches!(
            reg.sync_catalog(0, false).await,
            Err(DiscoveryError::InvalidInput(_))
        ));
        // Over max limit rejected
        assert!(matches!(
            reg.sync_catalog(MAX_SYNC_LIMIT + 1, false).await,
            Err(DiscoveryError::InvalidInput(_))
        ));

        // Valid sync with dedup + end-of-catalog detection
        let res = reg.sync_catalog(10, false).await.unwrap();
        assert_eq!(res.total_indexed, 2);
        assert!(res.complete);
        assert!(!res.partial);

        // Preserve existing index when a subsequent sync fails (404 / 429 / malformed / missing skills)
        for bad_ep in ["/malformed", "/noskills", "/404", "/429"] {
            let bad_reg = SkillsDirectoryRegistry::with_endpoint(
                &temp_dir,
                false,
                format!("{}{}", base, bad_ep),
                5,
            );
            assert!(bad_reg.sync_catalog(10, false).await.is_err());
            let preserved = bad_reg.load_index();
            assert_eq!(
                preserved.skills.len(),
                2,
                "Index must be preserved after {} failure",
                bad_ep
            );
        }

        // Pagination + empty final page + resume + reset
        let paged_reg =
            SkillsDirectoryRegistry::with_endpoint(&temp_dir, false, format!("{}/paged", base), 5);
        let p_res = paged_reg.sync_catalog(150, true).await.unwrap();
        assert_eq!(p_res.total_indexed, 100);
        assert!(p_res.complete);

        // No-network does not make requests
        let offline_reg = SkillsDirectoryRegistry::with_endpoint(
            &temp_dir,
            true,
            "http://127.0.0.1:1/unreachable",
            1,
        );
        let off_res = offline_reg.sync_catalog(10, false).await.unwrap();
        assert_eq!(off_res.synced_pages, 0);
        assert_eq!(off_res.total_indexed, 100);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
