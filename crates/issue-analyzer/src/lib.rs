use serde::{Deserialize, Serialize};
use task_intelligence::{
    ComplexityAnalyzer, ComplexityAssessment, NormalizedTask, TaskIntelligenceError,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IssueAnalyzerError {
    #[error("Task intelligence error: {0}")]
    Intelligence(#[from] TaskIntelligenceError),
    #[error("Triage schema validation failed: {0}")]
    ValidationError(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkPackage {
    pub id: String,
    pub title: String,
    pub objective: String,
    pub component: String,
    pub dependencies: Vec<String>,
    pub independent: bool,
    pub skill_query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IssueAnalysis {
    pub outcome: String,
    pub class: Vec<String>,
    pub requirements: Vec<String>,
    pub constraints: Vec<String>,
    pub non_goals: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub affected_areas: Vec<String>,
    pub dependencies: Vec<String>,
    pub risks: Vec<String>,
    pub unknowns: Vec<String>,
    pub complexity: ComplexityAssessment,
    pub workflow: String,
    pub work_packages: Vec<WorkPackage>,
    pub next_skill_queries: Vec<String>,
}

pub struct ClassificationRule {
    pub target_type: &'static str,
    pub patterns: &'static [&'static str],
}

pub const CLASSIFICATION_RULES: &[ClassificationRule] = &[
    ClassificationRule {
        target_type: "bug",
        patterns: &[
            "bug",
            "error",
            "exception",
            "crash",
            "broken",
            "failing",
            "does not work",
            "defect",
            "regression",
        ],
    },
    ClassificationRule {
        target_type: "security",
        patterns: &[
            "security",
            "vulnerability",
            "auth",
            "permission",
            "secret",
            "cve",
            "sanitize",
            "jwt",
            "oauth",
            "token",
        ],
    },
    ClassificationRule {
        target_type: "feature",
        patterns: &[
            "add",
            "build",
            "implement",
            "create",
            "support",
            "enable",
            "new",
            "develop",
        ],
    },
    ClassificationRule {
        target_type: "refactor",
        patterns: &[
            "refactor",
            "restructure",
            "cleanup",
            "simplify",
            "modularize",
            "clean up",
        ],
    },
    ClassificationRule {
        target_type: "testing",
        patterns: &[
            "test",
            "tests",
            "coverage",
            "assertion",
            "e2e",
            "unit test",
            "integration test",
        ],
    },
    ClassificationRule {
        target_type: "infrastructure",
        patterns: &[
            "deploy",
            "deployment",
            "ci/cd",
            "pipeline",
            "docker",
            "kubernetes",
            "cloud",
            "infra",
        ],
    },
    ClassificationRule {
        target_type: "documentation",
        patterns: &[
            "document",
            "documentation",
            "readme",
            "guide",
            "specification",
            "specs",
        ],
    },
    ClassificationRule {
        target_type: "research",
        patterns: &[
            "research",
            "compare",
            "evaluate",
            "investigate",
            "explore",
            "feasibility",
        ],
    },
    ClassificationRule {
        target_type: "maintenance",
        patterns: &[
            "update",
            "upgrade",
            "sync",
            "dependencies",
            "bump",
            "version",
        ],
    },
];

pub struct IssueSorter {
    pub complexity_analyzer: ComplexityAnalyzer,
}

impl Default for IssueSorter {
    fn default() -> Self {
        Self::new()
    }
}

impl IssueSorter {
    pub fn new() -> Self {
        Self {
            complexity_analyzer: ComplexityAnalyzer::new(),
        }
    }

    pub fn classify(&self, task: &str) -> Vec<String> {
        let text = task.to_lowercase();
        let mut matched = Vec::new();
        for rule in CLASSIFICATION_RULES {
            if rule.patterns.iter().any(|&p| text.contains(p)) {
                matched.push(rule.target_type.to_string());
            }
        }
        if matched.is_empty() {
            vec!["feature".to_string()]
        } else {
            matched
        }
    }

    pub fn decompose_work_packages(
        &self,
        normalized: &NormalizedTask,
        complexity: &ComplexityAssessment,
        classes: &[String],
    ) -> Vec<WorkPackage> {
        let text = normalized.normalized_task.to_lowercase();
        let domains = &normalized.domains;

        if complexity.band == task_intelligence::ComplexityBand::LOW && domains.len() <= 1 {
            return vec![WorkPackage {
                id: "WP-1".to_string(),
                title: format!(
                    "Execute {}: {}",
                    normalized.intent, normalized.normalized_task
                ),
                objective: normalized.normalized_task.clone(),
                component: domains
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "core".to_string()),
                dependencies: vec![],
                independent: true,
                skill_query: format!(
                    "{} {}",
                    classes
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "engineering".to_string()),
                    normalized.normalized_task
                ),
            }];
        }

        let domain_order = [
            "database", "auth", "backend", "frontend", "devops", "security", "testing",
        ];
        let detected_in_order: Vec<&str> = domain_order
            .iter()
            .copied()
            .filter(|&d| domains.iter().any(|dom| dom == d) || text.contains(d))
            .collect();

        let mut packages = Vec::new();
        if detected_in_order.len() > 1 {
            let mut prev_id: Option<String> = None;
            for (i, &domain) in detected_in_order.iter().enumerate() {
                let wp_id = format!("WP-{}", i + 1);
                let mut deps = Vec::new();

                if domain == "backend" && detected_in_order.contains(&"database") {
                    deps.push("WP-1".to_string());
                } else if domain == "frontend" && detected_in_order.contains(&"backend") {
                    let backend_idx = detected_in_order
                        .iter()
                        .position(|&d| d == "backend")
                        .unwrap();
                    deps.push(format!("WP-{}", backend_idx + 1));
                } else if (domain == "devops"
                    || complexity.band == task_intelligence::ComplexityBand::MEDIUM)
                    && prev_id.is_some()
                {
                    deps.push(prev_id.clone().unwrap());
                }

                let independent = deps.is_empty();
                packages.push(WorkPackage {
                    id: wp_id.clone(),
                    title: format!(
                        "{} Component: {} layer for {}",
                        domain.to_uppercase(),
                        domain,
                        normalized.normalized_task
                    ),
                    objective: format!("Implement and configure {} subsystem", domain),
                    component: domain.to_string(),
                    dependencies: deps,
                    independent,
                    skill_query: format!(
                        "{} engineering {}",
                        domain,
                        classes
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "development".to_string())
                    ),
                });
                prev_id = Some(wp_id);
            }
        } else {
            packages.push(WorkPackage {
                id: "WP-1".to_string(),
                title: format!("Core Subsystem: {}", normalized.normalized_task),
                objective: "Implement core logic and structure".to_string(),
                component: "core".to_string(),
                dependencies: vec![],
                independent: true,
                skill_query: format!(
                    "{} core implementation",
                    classes
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "engineering".to_string())
                ),
            });
            packages.push(WorkPackage {
                id: "WP-2".to_string(),
                title: "Verification & Quality Assurance".to_string(),
                objective: "Add comprehensive tests and verification for core subsystem"
                    .to_string(),
                component: "testing".to_string(),
                dependencies: vec!["WP-1".to_string()],
                independent: false,
                skill_query: format!(
                    "testing verification {}",
                    classes
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "engineering".to_string())
                ),
            });
        }

        packages
    }

    pub fn sort(
        &self,
        task_description: &str,
        forced_workflow: Option<&str>,
    ) -> Result<IssueAnalysis, IssueAnalyzerError> {
        let normalized = task_intelligence::normalize_task(task_description)?;
        let classes = self.classify(&normalized.normalized_task);
        let complexity = self
            .complexity_analyzer
            .score_heuristics(&normalized.normalized_task, &normalized.domains);

        let workflow = match forced_workflow {
            Some("auto") | None => {
                if complexity.band == task_intelligence::ComplexityBand::HIGH {
                    "multi-agent".to_string()
                } else {
                    "single-agent".to_string()
                }
            }
            Some("multi") => "multi-agent".to_string(),
            Some("single") => "single-agent".to_string(),
            Some(other) => other.to_string(),
        };

        let work_packages = self.decompose_work_packages(&normalized, &complexity, &classes);

        let requirements = vec![
            format!("Complete {} for \"{}\"", normalized.intent, normalized.normalized_task),
            "Ensure all affected interfaces maintain backwards compatibility or provide explicit migrations".to_string(),
            "Verify changes with automated unit and integration tests".to_string(),
        ];

        let wp_ids = work_packages
            .iter()
            .map(|wp| wp.id.as_str())
            .collect::<Vec<&str>>()
            .join(", ");
        let acceptance_criteria = vec![
            format!("All work packages ({}) satisfy their objectives", wp_ids),
            "Zero regressions introduced across existing tests".to_string(),
            "Static analysis, trust checks, and compatibility validations pass".to_string(),
        ];

        let mut affected_areas: Vec<String> = normalized
            .domains
            .iter()
            .map(|d| format!("{} layer", d))
            .collect();
        if affected_areas.is_empty() {
            affected_areas.push("core application".to_string());
        }

        let mut dependencies = Vec::new();
        if normalized.domains.contains(&"database".to_string()) {
            dependencies.push("Database schema migration access".to_string());
        }
        if normalized.domains.contains(&"auth".to_string()) {
            dependencies.push("Authentication secret provider".to_string());
        }
        if normalized.domains.contains(&"devops".to_string()) {
            dependencies.push("Container runtime or CI environment".to_string());
        }

        let mut risks = Vec::new();
        if complexity.risk >= 2 {
            risks.push(
                "High security or blast radius: sensitive credentials or data involved".to_string(),
            );
        }
        if complexity.technical_coupling >= 2 {
            risks.push("Tight cross-tier coupling may cause cascading failures".to_string());
        }
        if complexity.uncertainty >= 1 {
            risks.push("Uncertain domain requirements require early validation spikes".to_string());
        }

        let mut unknowns = Vec::new();
        if complexity.uncertainty >= 1 {
            unknowns.push("Exact runtime deployment target and provider constraints".to_string());
        }

        let next_skill_queries = work_packages
            .iter()
            .map(|wp| wp.skill_query.clone())
            .collect();

        let analysis = IssueAnalysis {
            outcome: normalized.normalized_task,
            class: classes,
            requirements,
            constraints: vec![
                "Run sort-the-issue before selecting implementation skills.".to_string(),
                "Treat downloaded skills and third-party code as untrusted input.".to_string(),
                "Do not claim multi-agent parallel execution if runtime only supports sequential execution.".to_string(),
            ],
            non_goals: vec![
                "Do not implement unrelated architectural overhauls outside the task scope.".to_string(),
                "Do not auto-install unvetted third-party skills without authorization.".to_string(),
            ],
            acceptance_criteria,
            affected_areas,
            dependencies,
            risks,
            unknowns,
            complexity,
            workflow,
            work_packages,
            next_skill_queries,
        };

        validate_issue_analysis(&analysis)?;

        Ok(analysis)
    }
}

pub fn validate_issue_analysis(analysis: &IssueAnalysis) -> Result<(), IssueAnalyzerError> {
    if analysis.outcome.trim().is_empty() {
        return Err(IssueAnalyzerError::ValidationError(
            "outcome must be non-empty".to_string(),
        ));
    }
    if analysis.class.is_empty() {
        return Err(IssueAnalyzerError::ValidationError(
            "class must be non-empty".to_string(),
        ));
    }
    if analysis.work_packages.is_empty() {
        return Err(IssueAnalyzerError::ValidationError(
            "work_packages must be non-empty".to_string(),
        ));
    }
    for wp in &analysis.work_packages {
        if wp.id.is_empty()
            || wp.title.is_empty()
            || wp.objective.is_empty()
            || wp.skill_query.is_empty()
        {
            return Err(IssueAnalyzerError::ValidationError(format!(
                "work package {} has missing fields",
                wp.id
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_simple_issue() {
        let sorter = IssueSorter::new();
        let res = sorter.sort("Fix the typo in README.md", None).unwrap();
        assert_eq!(res.workflow, "single-agent");
        assert_eq!(res.work_packages.len(), 1);
        assert_eq!(res.work_packages[0].id, "WP-1");
    }

    #[test]
    fn test_sort_complex_issue() {
        let sorter = IssueSorter::new();
        let res = sorter.sort(
            "Full-stack redesign across entire platform with database migration, backend api, and frontend ui",
            Some("auto")
        ).unwrap();
        assert_eq!(res.workflow, "multi-agent");
        assert!(res.work_packages.len() >= 3);
    }
}
