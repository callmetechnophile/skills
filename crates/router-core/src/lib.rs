use agent_orchestrator::{DependencyGraph, ExecutionPlan};
use issue_analyzer::{IssueAnalysis, IssueSorter};
use runtime_adapters::get_runtime_adapter;
use serde::{Deserialize, Serialize};
use skill_activation::{ActivationEngine, ActivationReport, SkillActivationRecord};
use skill_discovery::{DirectoryResolver, InstalledRegistry, SkillsDirectoryRegistry};
use skill_lifecycle::InstalledTracker;
use skill_ranking::{HybridRanker, RankedSkill};
use skill_registry::{LocalRegistry, RegistryProvider};
use skill_security::{TrustPolicy, TrustScanner};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RouterError {
    #[error("Provide a non-empty task.")]
    EmptyTask,
    #[error("Mandatory sort-the-issue skill is missing from skills/core/sort-the-issue/SKILL.md.")]
    MissingMandatorySkill,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Analysis error: {0}")]
    Analysis(#[from] issue_analyzer::IssueAnalyzerError),
    #[error("Lifecycle error: {0}")]
    Lifecycle(#[from] skill_lifecycle::LifecycleError),
    #[error("Orchestration error: {0}")]
    Orchestration(#[from] agent_orchestrator::OrchestrationError),
    #[error("Router error: {0}")]
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub workflow: String,
    pub selection_policy: String,
    pub fallback: String,
    pub next_skill_queries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResult {
    pub mandatory_first_skill: String,
    pub triage: IssueAnalysis,
    pub routing: RoutingDecision,
    pub activation: ActivationReport,
    pub execution_plan: Option<ExecutionPlan>,
}

#[derive(Debug, Clone)]
pub struct RouteOptions {
    pub workflow: Option<String>,
    pub agent: String,
    pub no_network: bool,
    pub local_only: bool,
    pub strict_compatibility: bool,
}

impl Default for RouteOptions {
    fn default() -> Self {
        Self {
            workflow: None,
            agent: "antigravity".to_string(),
            no_network: false,
            local_only: false,
            strict_compatibility: true,
        }
    }
}

pub struct SkillRouter {
    pub root_dir: PathBuf,
    pub sorter: IssueSorter,
    pub local_registry: LocalRegistry,
    pub installed_registry: InstalledRegistry,
    pub remote_registry: SkillsDirectoryRegistry,
    pub directory_resolver: DirectoryResolver,
    pub scanner: TrustScanner,
    pub policy: TrustPolicy,
    pub tracker: InstalledTracker,
}

impl SkillRouter {
    pub fn new<P: AsRef<Path>>(root_dir: P, no_network: bool) -> Self {
        let root = root_dir.as_ref().to_path_buf();
        Self {
            root_dir: root.clone(),
            sorter: IssueSorter::new(),
            local_registry: LocalRegistry::new(&root),
            installed_registry: InstalledRegistry::new(&root),
            remote_registry: SkillsDirectoryRegistry::new(&root, no_network),
            directory_resolver: DirectoryResolver::new(),
            scanner: TrustScanner::new(),
            policy: TrustPolicy::new(),
            tracker: InstalledTracker::new(&root),
        }
    }

    pub fn sort(&self, task: &str, workflow: Option<&str>) -> Result<IssueAnalysis, RouterError> {
        let trimmed = task.trim();
        if trimmed.is_empty() {
            return Err(RouterError::EmptyTask);
        }
        let triage_file = self.root_dir.join("skills/core/sort-the-issue/SKILL.md");
        if !triage_file.exists() {
            return Err(RouterError::MissingMandatorySkill);
        }
        Ok(self.sorter.sort(trimmed, workflow)?)
    }

    pub async fn route(
        &self,
        task: &str,
        options: &RouteOptions,
    ) -> Result<RouteResult, RouterError> {
        let mut triage = self.sort(task, options.workflow.as_deref())?;

        let workflow = match options.workflow.as_deref() {
            Some("single") => "single-agent".to_string(),
            Some("multi") => "multi-agent".to_string(),
            Some(w) if w != "auto" => w.to_string(),
            _ => triage.workflow.clone(),
        };
        triage.workflow = workflow.clone();

        let selection_policy = if workflow == "single-agent" {
            "Discover and load one specialist skill at a time, sequentially.".to_string()
        } else {
            "Discover per work package; load specialist skills in parallel only when actual sub-agent execution is supported.".to_string()
        };

        let routing = RoutingDecision {
            workflow: workflow.clone(),
            selection_policy,
            fallback: "If sub-agents are unavailable, run specialist work packages sequentially; never claim parallelism.".to_string(),
            next_skill_queries: triage.next_skill_queries.clone(),
        };

        let adapter = get_runtime_adapter(&options.agent);
        let subagent_cap = adapter.check_subagent_support();

        let local_skills = self.local_registry.get_all_skills();
        let installed_skills = self
            .installed_registry
            .search("", usize::MAX)
            .await
            .unwrap_or_default();

        let mut activations = Vec::new();
        let mut failures = Vec::new();

        let ranker = HybridRanker::new(&options.agent);

        for wp in &triage.work_packages {
            let query = if !wp.skill_query.is_empty() {
                &wp.skill_query
            } else if !wp.objective.is_empty() {
                &wp.objective
            } else {
                &triage.outcome
            };

            let mut candidates = Vec::new();
            candidates.extend(local_skills.clone());
            candidates.extend(installed_skills.clone());

            if !options.local_only {
                if let Ok(remotes) = self.remote_registry.search(query, 10).await {
                    candidates.extend(remotes);
                }
            }

            let ranking = ranker.rank(
                &candidates,
                query,
                Some(&options.agent),
                options.strict_compatibility,
            );
            let top_match = ranking.selected;

            if let Some(ranked) = top_match {
                let skill = ranked.skill;
                let is_local =
                    skill.source.as_deref() == Some("local") || skill.registry == "local";
                let is_installed = skill.registry == "installed";

                if is_local || is_installed {
                    let content_ref = if is_local {
                        self.local_registry.retrieve_content(&skill.id).await
                    } else {
                        self.installed_registry.retrieve_content(&skill.id).await
                    };

                    match content_ref {
                        Ok(cr) => {
                            let assessment = self.scanner.scan(&cr.raw_content, &skill);
                            let source_type = if is_local {
                                "local-repository"
                            } else {
                                "locally-installed"
                            };
                            let source_location = skill.path.as_deref().unwrap_or_default();

                            match ActivationEngine::activate(
                                &wp.id,
                                &wp.component,
                                &skill.name,
                                &cr.raw_content,
                                source_type,
                                source_location,
                                &assessment,
                                &self.policy,
                                adapter.as_ref(),
                                false,
                            ) {
                                Ok(record) => activations.push(record),
                                Err(record) => failures.push(*record),
                            }
                        }
                        Err(e) => {
                            failures.push(SkillActivationRecord {
                                work_package_id: wp.id.clone(),
                                skill_name: skill.name.clone(),
                                status: "FAILED".to_string(),
                                source_type: if is_local {
                                    "local-repository".to_string()
                                } else {
                                    "locally-installed".to_string()
                                },
                                source_location: skill.path.clone().unwrap_or_default(),
                                exact_directory: None,
                                install_command: None,
                                trust_status: "UNSCANNED".to_string(),
                                content_hash: None,
                                verified: false,
                                instruction_payload: None,
                                reason: Some("CONTENT_RETRIEVAL_ERROR".to_string()),
                                message: Some(format!("Could not read skill content: {}", e)),
                                note: None,
                            });
                        }
                    }
                } else {
                    let resolution = self
                        .directory_resolver
                        .resolve_skill_directory(
                            skill.repository.as_deref().unwrap_or("unknown/repo"),
                            skill.skill_directory.as_deref().unwrap_or(&skill.slug),
                            None,
                            None,
                        )
                        .await;

                    if !resolution.success {
                        failures.push(SkillActivationRecord {
                            work_package_id: wp.id.clone(),
                            skill_name: skill.name.clone(),
                            status: "FAILED".to_string(),
                            source_type: "remote-registry".to_string(),
                            source_location: skill.repository.clone().unwrap_or_default(),
                            exact_directory: None,
                            install_command: None,
                            trust_status: "UNSCANNED".to_string(),
                            content_hash: None,
                            verified: false,
                            instruction_payload: None,
                            reason: resolution.error,
                            message: Some(format!(
                                "Cannot activate remote skill: directory resolution failed: {}",
                                resolution.message.unwrap_or_default()
                            )),
                            note: None,
                        });
                    } else {
                        activations.push(SkillActivationRecord {
                            work_package_id: wp.id.clone(),
                            skill_name: skill.name.clone(),
                            status: "REQUIRES_INSTALLATION".to_string(),
                            source_type: "remote-registry".to_string(),
                            source_location: skill.repository.clone().unwrap_or_default(),
                            exact_directory: resolution.exact_directory,
                            install_command: resolution.install_command,
                            trust_status: "UNSCANNED".to_string(),
                            content_hash: None,
                            verified: false,
                            instruction_payload: None,
                            reason: None,
                            message: None,
                            note: Some("Skill must be installed and verified before it can be activated into the live session.".to_string()),
                        });
                    }
                }
            } else {
                failures.push(SkillActivationRecord {
                    work_package_id: wp.id.clone(),
                    skill_name: query.to_string(),
                    status: "FAILED".to_string(),
                    source_type: "none".to_string(),
                    source_location: "".to_string(),
                    exact_directory: None,
                    install_command: None,
                    trust_status: "UNSCANNED".to_string(),
                    content_hash: None,
                    verified: false,
                    instruction_payload: None,
                    reason: Some("NO_SUITABLE_SKILL".to_string()),
                    message: Some(format!(
                        "No compatible or approved skill discovered for query '{}'.",
                        query
                    )),
                    note: None,
                });
            }
        }

        let dep_graph = DependencyGraph::new(&triage.work_packages);
        let execution_plan = dep_graph
            .create_execution_plan(&workflow, &subagent_cap)
            .ok();

        let report = ActivationReport::new(
            &triage.outcome,
            &options.agent,
            &workflow,
            activations,
            failures,
            execution_plan.clone(),
        );

        Ok(RouteResult {
            mandatory_first_skill: "sort-the-issue".to_string(),
            triage,
            routing,
            activation: report,
            execution_plan,
        })
    }

    pub async fn find(&self, query: &str, agent: &str, limit: usize) -> Vec<RankedSkill> {
        let mut candidates = Vec::new();
        candidates.extend(self.local_registry.get_all_skills());
        if let Ok(installed) = self.installed_registry.search("", usize::MAX).await {
            candidates.extend(installed);
        }
        if let Ok(remotes) = self.remote_registry.search(query, limit).await {
            candidates.extend(remotes);
        }

        let ranker = HybridRanker::new(agent);
        let result = ranker.rank(&candidates, query, Some(agent), false);
        result.ranked.into_iter().take(limit).collect()
    }

    pub fn prepare_execution(
        &self,
        result: &RouteResult,
    ) -> Option<execution_engine::PreparedExecutionBatch> {
        result.execution_plan.as_ref().map(|plan| {
            execution_engine::ExecutionEngine::prepare_execution(
                plan,
                &result.activation.activations,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_task_returns_error() {
        let router = SkillRouter::new(".", true);
        let res = router.sort("   ", None);
        assert!(matches!(res, Err(RouterError::EmptyTask)));
    }

    #[test]
    fn test_missing_mandatory_skill_returns_error() {
        let temp_dir = std::env::temp_dir().join(format!("test_no_sort_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let router = SkillRouter::new(&temp_dir, true);
        let res = router.sort("Fix login button bug", None);
        assert!(matches!(res, Err(RouterError::MissingMandatorySkill)));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_route_options_defaults() {
        let opts = RouteOptions::default();
        assert_eq!(opts.agent, "antigravity");
        assert!(!opts.no_network);
        assert!(!opts.local_only);
        assert!(opts.strict_compatibility);
    }
}
