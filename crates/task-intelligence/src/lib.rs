use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum TaskIntelligenceError {
    #[error("Task description must be a non-empty string.")]
    EmptyTask,
    #[error("LLM provider error: {0}")]
    ProviderError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum TaskDomain {
    Frontend,
    Backend,
    Database,
    Auth,
    DevOps,
    Security,
    Testing,
    Core,
    General,
}

impl TaskDomain {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskDomain::Frontend => "frontend",
            TaskDomain::Backend => "backend",
            TaskDomain::Database => "database",
            TaskDomain::Auth => "auth",
            TaskDomain::DevOps => "devops",
            TaskDomain::Security => "security",
            TaskDomain::Testing => "testing",
            TaskDomain::Core => "core",
            TaskDomain::General => "general",
        }
    }

    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "frontend" => Some(TaskDomain::Frontend),
            "backend" => Some(TaskDomain::Backend),
            "database" => Some(TaskDomain::Database),
            "auth" => Some(TaskDomain::Auth),
            "devops" => Some(TaskDomain::DevOps),
            "security" => Some(TaskDomain::Security),
            "testing" => Some(TaskDomain::Testing),
            "core" => Some(TaskDomain::Core),
            "general" => Some(TaskDomain::General),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TaskIntent {
    Create,
    Fix,
    Optimize,
    Audit,
    Sync,
    GeneralEngineering,
}

impl TaskIntent {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskIntent::Create => "create",
            TaskIntent::Fix => "fix",
            TaskIntent::Optimize => "optimize",
            TaskIntent::Audit => "audit",
            TaskIntent::Sync => "sync",
            TaskIntent::GeneralEngineering => "general-engineering",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub raw_prompt: String,
    pub forced_workflow: Option<String>,
    pub target_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedTask {
    #[serde(rename = "taskId")]
    pub task_id: Uuid,
    #[serde(rename = "originalTask")]
    pub original_task: String,
    #[serde(rename = "normalizedTask")]
    pub normalized_task: String,
    pub domains: Vec<String>,
    pub intent: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComplexityBand {
    LOW,
    MEDIUM,
    HIGH,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComplexityAssessment {
    pub scope_breadth: u8,
    pub technical_coupling: u8,
    pub uncertainty: u8,
    pub risk: u8,
    pub parallelism: u8,
    pub total_score: u8,
    pub band: ComplexityBand,
    pub method: String,
}

#[async_trait::async_trait]
pub trait TaskComplexityEvaluator: Send + Sync {
    async fn evaluate(
        &self,
        task: &NormalizedTask,
    ) -> Result<ComplexityAssessment, TaskIntelligenceError>;
}

const DOMAIN_KEYWORDS: &[(&str, &[&str])] = &[
    (
        "frontend",
        &[
            "frontend", "react", "vue", "angular", "css", "html", "ui", "tailwind", "next.js",
            "client",
        ],
    ),
    (
        "backend",
        &[
            "backend",
            "api",
            "rest",
            "graphql",
            "server",
            "express",
            "fastapi",
            "django",
            "node",
            "controller",
        ],
    ),
    (
        "database",
        &[
            "database",
            "postgres",
            "postgresql",
            "mysql",
            "mongodb",
            "sqlite",
            "prisma",
            "migration",
            "sql",
            "redis",
            "schema",
        ],
    ),
    (
        "auth",
        &[
            "auth",
            "authentication",
            "authorization",
            "jwt",
            "oauth",
            "sso",
            "login",
            "password",
            "token",
        ],
    ),
    (
        "devops",
        &[
            "devops",
            "deployment",
            "deploy",
            "docker",
            "kubernetes",
            "ci/cd",
            "pipeline",
            "github actions",
            "cloud",
            "aws",
            "gcp",
        ],
    ),
    (
        "security",
        &[
            "security",
            "vulnerability",
            "encryption",
            "secret",
            "permission",
            "cve",
            "sanitize",
            "xss",
            "csrf",
        ],
    ),
    (
        "testing",
        &[
            "test",
            "tests",
            "unit test",
            "e2e",
            "integration test",
            "jest",
            "coverage",
            "playwright",
        ],
    ),
];

const ACTION_VERBS: &[(&str, &[&str])] = &[
    (
        "create",
        &[
            "build",
            "create",
            "implement",
            "add",
            "develop",
            "generate",
            "scaffold",
        ],
    ),
    (
        "fix",
        &["fix", "resolve", "patch", "repair", "debug", "correct"],
    ),
    (
        "optimize",
        &[
            "optimize", "tune", "speed up", "refactor", "cleanup", "improve",
        ],
    ),
    (
        "audit",
        &[
            "audit", "review", "scan", "analyze", "inspect", "assess", "evaluate",
        ],
    ),
    ("sync", &["sync", "update", "migrate", "upgrade"]),
];

pub fn normalize_task(raw_task: &str) -> Result<NormalizedTask, TaskIntelligenceError> {
    let trimmed = raw_task.trim();
    if trimmed.is_empty() {
        return Err(TaskIntelligenceError::EmptyTask);
    }

    let cleaned: String = trimmed.split_whitespace().collect::<Vec<&str>>().join(" ");
    let lower = cleaned.to_lowercase();

    let mut domains = Vec::new();
    for (domain, keywords) in DOMAIN_KEYWORDS {
        let matched = keywords.iter().any(|kw| contains_word(&lower, kw));
        if matched {
            domains.push(domain.to_string());
        }
    }

    let mut intent = "general-engineering".to_string();
    for (action, verbs) in ACTION_VERBS {
        let matched = verbs.iter().any(|v| contains_word(&lower, v));
        if matched {
            intent = action.to_string();
            break;
        }
    }

    Ok(NormalizedTask {
        task_id: Uuid::new_v4(),
        original_task: raw_task.to_string(),
        normalized_task: cleaned,
        domains: if domains.is_empty() {
            vec!["general".to_string()]
        } else {
            domains
        },
        intent,
    })
}

fn contains_word(text: &str, keyword: &str) -> bool {
    let kw_lower = keyword.to_lowercase();
    let text_bytes = text.as_bytes();
    let kw_bytes = kw_lower.as_bytes();

    if kw_bytes.is_empty() || text_bytes.len() < kw_bytes.len() {
        return false;
    }

    let mut pos = 0;
    while let Some(found_idx) = text[pos..].to_lowercase().find(&kw_lower) {
        let start = pos + found_idx;
        let end = start + kw_bytes.len();

        let left_ok = if start == 0 {
            true
        } else {
            let prev_char = text[..start].chars().last().unwrap();
            !prev_char.is_alphanumeric() && prev_char != '_'
        };

        let right_ok = if end >= text.len() {
            true
        } else {
            let next_char = text[end..].chars().next().unwrap();
            !next_char.is_alphanumeric() && next_char != '_'
        };

        if left_ok && right_ok {
            return true;
        }
        pos = start + 1;
        if pos >= text.len() {
            break;
        }
    }
    false
}

#[derive(Default)]
pub struct ComplexityAnalyzer {
    pub uncertainty_threshold: f32,
}

impl ComplexityAnalyzer {
    pub fn new() -> Self {
        Self {
            uncertainty_threshold: 0.6,
        }
    }

    pub fn score_heuristics(&self, task: &str, domains: &[String]) -> ComplexityAssessment {
        let text = task.to_lowercase();

        // 1. Scope Breadth
        let mut scope_breadth = 0;
        let non_testing_domains: Vec<&String> =
            domains.iter().filter(|d| *d != "testing").collect();
        let broad_regex = [
            "entire",
            "whole",
            "all services",
            "across repo",
            "end-to-end",
            "platform",
            "system-wide",
            "full-stack",
        ];
        let has_broad = broad_regex.iter().any(|k| text.contains(k));

        let module_regex = [
            "component",
            "module",
            "service",
            "package",
            "multiple files",
            "workflow",
        ];
        let has_module = module_regex.iter().any(|k| text.contains(k));

        if has_broad || non_testing_domains.len() >= 3 {
            scope_breadth = 2;
        } else if has_module || non_testing_domains.len() >= 2 {
            scope_breadth = 1;
        }

        // 2. Technical Coupling
        let tier_matches = [
            text.contains("database")
                || text.contains("sql")
                || text.contains("postgres")
                || text.contains("mongo")
                || text.contains("migration")
                || text.contains("schema"),
            text.contains("frontend")
                || text.contains("react")
                || text.contains("ui")
                || text.contains("css")
                || text.contains("client"),
            text.contains("backend")
                || text.contains("api")
                || text.contains("server")
                || text.contains("microservice"),
            text.contains("auth")
                || text.contains("oauth")
                || text.contains("jwt")
                || text.contains("session"),
            text.contains("devops")
                || text.contains("docker")
                || text.contains("kubernetes")
                || text.contains("ci/cd")
                || text.contains("pipeline"),
        ]
        .iter()
        .filter(|&&m| m)
        .count();

        let mut technical_coupling = 0;
        let has_coupling_words = text.contains("integration")
            || text.contains("coupled")
            || text.contains("cross-cutting")
            || text.contains("full-stack");
        if tier_matches >= 3 || has_coupling_words {
            technical_coupling = 2;
        } else if tier_matches >= 2 {
            technical_coupling = 1;
        }

        // 3. Uncertainty
        let mut uncertainty = 0;
        let high_uncertainty = [
            "explore",
            "research",
            "unclear",
            "unknown",
            "spike",
            "evaluate architectural",
            "feasibility",
            "ambiguous",
        ];
        let mod_uncertainty = [
            "investigate",
            "diagnose",
            "audit",
            "debug intermittent",
            "analyze cause",
        ];
        if high_uncertainty.iter().any(|k| text.contains(k)) {
            uncertainty = 2;
        } else if mod_uncertainty.iter().any(|k| text.contains(k)) {
            uncertainty = 1;
        }

        // 4. Risk
        let mut risk = 0;
        let is_test_task = text.contains("test")
            || text.contains("tests")
            || text.contains("unit test")
            || text.contains("spec");
        let high_risk = [
            "security",
            "auth",
            "credential",
            "secret",
            "payment",
            "billing",
            "production deploy",
            "drop table",
            "delete data",
            "cve",
            "vulnerability",
        ];
        let mod_risk = [
            "migration",
            "refactor",
            "api change",
            "database",
            "state management",
            "auth",
            "security",
        ];

        if !is_test_task && high_risk.iter().any(|k| text.contains(k)) {
            risk = 2;
        } else if mod_risk.iter().any(|k| text.contains(k)) {
            risk = 1;
        }

        // 5. Parallelism
        let mut parallelism = 0;
        let parallel_keywords = [
            "independent",
            "parallel",
            "multi-agent",
            "sub-agent",
            "concurrently",
            "separate workstreams",
        ];
        if tier_matches >= 3 || parallel_keywords.iter().any(|k| text.contains(k)) {
            parallelism = 2;
        } else if tier_matches == 2
            || text.contains("and")
            || text.contains("as well as")
            || text.contains("both")
        {
            parallelism = 1;
        }

        let total_score = scope_breadth + technical_coupling + uncertainty + risk + parallelism;
        let band = if total_score <= 3 {
            ComplexityBand::LOW
        } else if total_score <= 6 {
            ComplexityBand::MEDIUM
        } else {
            ComplexityBand::HIGH
        };

        ComplexityAssessment {
            scope_breadth,
            technical_coupling,
            uncertainty,
            risk,
            parallelism,
            total_score,
            band,
            method: "deterministic-heuristic".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl TaskComplexityEvaluator for ComplexityAnalyzer {
    async fn evaluate(
        &self,
        task: &NormalizedTask,
    ) -> Result<ComplexityAssessment, TaskIntelligenceError> {
        Ok(self.score_heuristics(&task.normalized_task, &task.domains))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_simple_task() {
        let task = normalize_task("  Implement OAuth2 login   with JWT tokens  ").unwrap();
        assert_eq!(
            task.normalized_task,
            "Implement OAuth2 login with JWT tokens"
        );
        assert_eq!(task.intent, "create");
        assert!(task.domains.contains(&"auth".to_string()));
    }

    #[test]
    fn test_complexity_scoring_low() {
        let analyzer = ComplexityAnalyzer::new();
        let assessment =
            analyzer.score_heuristics("Fix the typo in README.md", &["general".to_string()]);
        assert_eq!(assessment.band, ComplexityBand::LOW);
        assert_eq!(assessment.total_score, 0);
    }

    #[test]
    fn test_complexity_scoring_high() {
        let analyzer = ComplexityAnalyzer::new();
        let domains = vec![
            "frontend".to_string(),
            "backend".to_string(),
            "database".to_string(),
            "auth".to_string(),
        ];
        let assessment = analyzer.score_heuristics("Full-stack redesign across entire platform with database migration, backend api, and frontend ui", &domains);
        assert_eq!(assessment.band, ComplexityBand::HIGH);
    }
}
