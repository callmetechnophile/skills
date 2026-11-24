use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use skill_registry::compute_sha256;
use skill_security::TrustScanner;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LifecycleError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Overwrite refused: {0}")]
    OverwriteRefused(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledSkill {
    #[serde(rename = "skillId")]
    pub skill_id: String,
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    pub source: String,
    #[serde(rename = "commitSha", default = "default_commit")]
    pub commit_sha: String,
    #[serde(default = "default_license")]
    pub license: String,
    #[serde(rename = "installationPath")]
    pub installation_path: String,
    #[serde(rename = "targetAgent", default = "default_agent")]
    pub target_agent: String,
    #[serde(rename = "contentHash")]
    pub content_hash: String,
    #[serde(rename = "trustStatus", default = "default_trust")]
    pub trust_status: String,
    #[serde(rename = "installedAt")]
    pub installed_at: DateTime<Utc>,
    #[serde(rename = "lastVerification")]
    pub last_verification: DateTime<Utc>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}
fn default_commit() -> String {
    "main".to_string()
}
fn default_license() -> String {
    "Unknown".to_string()
}
fn default_agent() -> String {
    "antigravity".to_string()
}
fn default_trust() -> String {
    "APPROVED".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerFile {
    pub schema_version: String,
    pub last_updated: DateTime<Utc>,
    pub installed: Vec<InstalledSkill>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillIntegrityStatus {
    #[serde(rename = "skillId")]
    pub skill_id: String,
    pub name: String,
    pub status: String,
    pub verified: bool,
    #[serde(rename = "currentHash", skip_serializing_if = "Option::is_none")]
    pub current_hash: Option<String>,
    #[serde(rename = "expectedHash")]
    pub expected_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub total: usize,
    pub intact: usize,
    pub modified: usize,
    pub missing: usize,
    pub details: Vec<SkillIntegrityStatus>,
}

pub struct InstalledTracker {
    pub root_dir: PathBuf,
    pub ledger_path: PathBuf,
    pub scanner: TrustScanner,
}

impl InstalledTracker {
    pub fn new<P: AsRef<Path>>(root_dir: P) -> Self {
        let root = root_dir.as_ref().to_path_buf();
        let ledger_path = root.join(".installed-skills.json");
        Self {
            root_dir: root,
            ledger_path,
            scanner: TrustScanner::new(),
        }
    }

    pub fn load_ledger(&self) -> LedgerFile {
        if let Ok(content) = std::fs::read_to_string(&self.ledger_path) {
            if let Ok(parsed) = serde_json::from_str::<LedgerFile>(&content) {
                return parsed;
            }
        }
        LedgerFile {
            schema_version: "1.0".to_string(),
            last_updated: Utc::now(),
            installed: vec![],
        }
    }

    pub fn save_ledger(&self, mut ledger: LedgerFile) -> Result<(), LifecycleError> {
        ledger.last_updated = Utc::now();
        let tmp = format!("{}.tmp", self.ledger_path.to_string_lossy());
        let json = serde_json::to_string_pretty(&ledger)?;
        std::fs::write(&tmp, json + "\n")?;
        std::fs::rename(&tmp, &self.ledger_path)?;
        Ok(())
    }

    pub fn list_skills(&self) -> Vec<InstalledSkill> {
        self.load_ledger().installed
    }

    pub fn record_installation(
        &self,
        skill: InstalledSkill,
        force: bool,
    ) -> Result<Vec<InstalledSkill>, LifecycleError> {
        let mut ledger = self.load_ledger();
        let existing_idx = ledger.installed.iter().position(|s| {
            s.skill_id == skill.skill_id || (s.source == skill.source && s.name == skill.name)
        });

        if let Some(idx) = existing_idx {
            let existing = &ledger.installed[idx];
            if !force {
                let full_path = self
                    .root_dir
                    .join(&existing.installation_path)
                    .join("SKILL.md");
                if let Ok(content) = std::fs::read_to_string(&full_path) {
                    let current_hash = compute_sha256(&content);
                    if current_hash != existing.content_hash {
                        return Err(LifecycleError::OverwriteRefused(format!(
                            "Existing installed skill '{}' was modified locally (content hash mismatch). Overwrite refused without --force.",
                            existing.name
                        )));
                    }
                }
            }
            ledger.installed[idx] = skill;
        } else {
            ledger.installed.push(skill);
        }

        self.save_ledger(ledger.clone())?;
        Ok(ledger.installed)
    }

    pub fn verify_skills(&self) -> VerificationReport {
        let ledger = self.load_ledger();
        let mut details = Vec::new();
        let mut intact = 0;
        let mut modified = 0;
        let mut missing = 0;

        for skill in &ledger.installed {
            let full_path = self
                .root_dir
                .join(&skill.installation_path)
                .join("SKILL.md");
            match std::fs::read_to_string(&full_path) {
                Ok(content) => {
                    let current_hash = compute_sha256(&content);
                    if current_hash == skill.content_hash {
                        intact += 1;
                        details.push(SkillIntegrityStatus {
                            skill_id: skill.skill_id.clone(),
                            name: skill.name.clone(),
                            status: "INTACT".to_string(),
                            verified: true,
                            current_hash: Some(current_hash),
                            expected_hash: skill.content_hash.clone(),
                            error: None,
                        });
                    } else {
                        modified += 1;
                        details.push(SkillIntegrityStatus {
                            skill_id: skill.skill_id.clone(),
                            name: skill.name.clone(),
                            status: "MODIFIED".to_string(),
                            verified: false,
                            current_hash: Some(current_hash),
                            expected_hash: skill.content_hash.clone(),
                            error: Some(
                                "Content hash does not match installed ledger record.".to_string(),
                            ),
                        });
                    }
                }
                Err(e) => {
                    missing += 1;
                    details.push(SkillIntegrityStatus {
                        skill_id: skill.skill_id.clone(),
                        name: skill.name.clone(),
                        status: "MISSING_OR_CORRUPT".to_string(),
                        verified: false,
                        current_hash: None,
                        expected_hash: skill.content_hash.clone(),
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        VerificationReport {
            total: ledger.installed.len(),
            intact,
            modified,
            missing,
            details,
        }
    }
}
