use issue_analyzer::WorkPackage;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use runtime_adapters::SubagentCapability;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OrchestrationError {
    #[error("Work package '{0}' references non-existent dependency '{1}'")]
    MissingDependency(String, String),
    #[error("Circular dependency detected in work package graph.")]
    CycleDetected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationStep {
    pub id: String,
    pub objective: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    #[serde(rename = "selectedMode")]
    pub selected_mode: String,
    #[serde(rename = "actualExecution")]
    pub actual_execution: String,
    #[serde(rename = "parallelEnabled")]
    pub parallel_enabled: bool,
    pub waves: Vec<Vec<WorkPackage>>,
    #[serde(rename = "linearOrder")]
    pub linear_order: Vec<WorkPackage>,
    pub explanation: String,
    #[serde(rename = "downgradeNotice", skip_serializing_if = "Option::is_none")]
    pub downgrade_notice: Option<String>,
    #[serde(rename = "integrationStep")]
    pub integration_step: IntegrationStep,
}

pub struct DependencyGraph {
    work_packages: Vec<WorkPackage>,
    package_map: HashMap<String, WorkPackage>,
}

impl DependencyGraph {
    pub fn new(work_packages: &[WorkPackage]) -> Self {
        let mut map = HashMap::new();
        for wp in work_packages {
            map.insert(wp.id.clone(), wp.clone());
        }
        Self {
            work_packages: work_packages.to_vec(),
            package_map: map,
        }
    }

    pub fn schedule(
        &self,
    ) -> Result<(Vec<WorkPackage>, Vec<Vec<WorkPackage>>), OrchestrationError> {
        let mut graph = petgraph::graph::DiGraph::<String, ()>::new();
        let mut node_indices = HashMap::new();

        for wp in &self.work_packages {
            let idx = graph.add_node(wp.id.clone());
            node_indices.insert(wp.id.clone(), idx);
        }

        for wp in &self.work_packages {
            let to_idx = node_indices[&wp.id];
            for dep_id in &wp.dependencies {
                let from_idx = node_indices.get(dep_id).copied().ok_or_else(|| {
                    OrchestrationError::MissingDependency(wp.id.clone(), dep_id.clone())
                })?;
                graph.add_edge(from_idx, to_idx, ());
            }
        }

        // Kahn's algorithm for wave scheduling
        let mut in_degrees: HashMap<NodeIndex, usize> = HashMap::new();
        let mut queue: VecDeque<NodeIndex> = VecDeque::new();

        for node in graph.node_indices() {
            let in_deg = graph.edges_directed(node, Direction::Incoming).count();
            in_degrees.insert(node, in_deg);
            if in_deg == 0 {
                queue.push_back(node);
            }
        }

        let mut waves: Vec<Vec<WorkPackage>> = Vec::new();
        let mut execution_order: Vec<WorkPackage> = Vec::new();
        let mut processed_count = 0;

        while !queue.is_empty() {
            let current_wave_nodes: Vec<NodeIndex> = queue.drain(..).collect();
            let mut current_wave_pkgs = Vec::new();

            for node in &current_wave_nodes {
                let id = &graph[*node];
                let pkg = self.package_map.get(id).unwrap().clone();
                current_wave_pkgs.push(pkg.clone());
                execution_order.push(pkg);
                processed_count += 1;
            }

            for node in current_wave_nodes {
                for edge in graph.edges_directed(node, Direction::Outgoing) {
                    let target = edge.target();
                    let deg = in_degrees.get_mut(&target).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(target);
                    }
                }
            }

            waves.push(current_wave_pkgs);
        }

        if processed_count != self.work_packages.len() {
            return Err(OrchestrationError::CycleDetected);
        }

        Ok((execution_order, waves))
    }

    pub fn create_execution_plan(
        &self,
        mode: &str,
        runtime_capability: &SubagentCapability,
    ) -> Result<ExecutionPlan, OrchestrationError> {
        let (execution_order, parallel_waves) = self.schedule()?;

        let all_ids: Vec<String> = self.work_packages.iter().map(|wp| wp.id.clone()).collect();

        // Mode A: Single isolated package
        if self.work_packages.len() <= 1 || mode == "single-agent" {
            return Ok(ExecutionPlan {
                selected_mode: "Mode A: Single-agent".to_string(),
                actual_execution: "sequential-single-agent".to_string(),
                parallel_enabled: false,
                waves: vec![self.work_packages.clone()],
                linear_order: self.work_packages.clone(),
                explanation: "Task is executed sequentially by a single agent one step at a time."
                    .to_string(),
                downgrade_notice: None,
                integration_step: IntegrationStep {
                    id: "INT-VALIDATION".to_string(),
                    objective: "Run automated unit and integration tests to verify changes"
                        .to_string(),
                    dependencies: all_ids,
                },
            });
        }

        // Mode B: Multi-agent (only if runtime truly supports sub-agents)
        if mode == "multi-agent" && runtime_capability.supported {
            return Ok(ExecutionPlan {
                selected_mode: "Mode B: Multi-agent".to_string(),
                actual_execution: "multi-agent-parallel".to_string(),
                parallel_enabled: true,
                waves: parallel_waves,
                linear_order: execution_order,
                explanation: "Independent work packages are dispatched to parallel specialist agents across dependency waves.".to_string(),
                downgrade_notice: None,
                integration_step: IntegrationStep {
                    id: "INT-MERGE-AND-VALIDATION".to_string(),
                    objective: "Integrator agent reconciles specialist outputs, merges changes, and validates end-to-end suite".to_string(),
                    dependencies: all_ids,
                },
            });
        }

        // Mode C: Sequential fallback
        Ok(ExecutionPlan {
            selected_mode: "Mode C: Sequential fallback".to_string(),
            actual_execution: "sequential-step-by-step".to_string(),
            parallel_enabled: false,
            waves: execution_order.iter().map(|wp| vec![wp.clone()]).collect(),
            linear_order: execution_order,
            explanation: "Task requires multi-step decomposition, but parallel sub-agents are unavailable in this environment. Falling back honestly to sequential specialist work packages.".to_string(),
            downgrade_notice: Some(runtime_capability.reason.clone()),
            integration_step: IntegrationStep {
                id: "INT-VALIDATION".to_string(),
                objective: "Perform final integration verification and regression testing".to_string(),
                dependencies: all_ids,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dag_waves_and_fallback() {
        let pkgs = vec![
            WorkPackage {
                id: "WP-1".to_string(),
                title: "Database".to_string(),
                objective: "DB setup".to_string(),
                component: "database".to_string(),
                dependencies: vec![],
                independent: true,
                skill_query: "db".to_string(),
            },
            WorkPackage {
                id: "WP-2".to_string(),
                title: "Backend".to_string(),
                objective: "API setup".to_string(),
                component: "backend".to_string(),
                dependencies: vec!["WP-1".to_string()],
                independent: false,
                skill_query: "api".to_string(),
            },
        ];

        let graph = DependencyGraph::new(&pkgs);
        let cap = SubagentCapability {
            supported: false,
            mode: "sequential-fallback".to_string(),
            reason: "Subagents disabled in CLI".to_string(),
        };

        let plan = graph.create_execution_plan("multi-agent", &cap).unwrap();
        assert_eq!(plan.selected_mode, "Mode C: Sequential fallback");
        assert_eq!(plan.actual_execution, "sequential-step-by-step");
        assert!(!plan.parallel_enabled);
        assert!(plan.downgrade_notice.is_some());
    }

    #[test]
    fn test_cycle_detection() {
        let pkgs = vec![
            WorkPackage {
                id: "WP-1".to_string(),
                title: "A".to_string(),
                objective: "A".to_string(),
                component: "core".to_string(),
                dependencies: vec!["WP-2".to_string()],
                independent: false,
                skill_query: "a".to_string(),
            },
            WorkPackage {
                id: "WP-2".to_string(),
                title: "B".to_string(),
                objective: "B".to_string(),
                component: "core".to_string(),
                dependencies: vec!["WP-1".to_string()],
                independent: false,
                skill_query: "b".to_string(),
            },
        ];

        let graph = DependencyGraph::new(&pkgs);
        let res = graph.schedule();
        assert!(res.is_err());
    }
}
