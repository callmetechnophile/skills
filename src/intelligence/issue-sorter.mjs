/**
 * Structured Issue Sorter implementing the sort-the-issue procedure.
 * Decomposes issues into testable requirements, risks, unknowns,
 * and component-bounded work packages with explicit dependencies.
 */

import { normalizeTask } from "./task-normalizer.mjs";
import { ComplexityAnalyzer } from "./complexity-analyzer.mjs";
import { validateTriageSchema } from "./schemas/task-schema.mjs";

const CLASSIFICATION_RULES = [
  { type: "bug", pattern: /(bug|error|exception|crash|broken|failing|does not work|defect|regression)/i },
  { type: "security", pattern: /(security|vulnerability|auth|permission|secret|cve|sanitize|jwt|oauth|token)/i },
  { type: "feature", pattern: /(add|build|implement|create|support|enable|new|develop)/i },
  { type: "refactor", pattern: /(refactor|restructure|cleanup|simplify|modularize|clean up)/i },
  { type: "testing", pattern: /(test|tests|coverage|assertion|e2e|unit test|integration test)/i },
  { type: "infrastructure", pattern: /(deploy|deployment|ci\/cd|pipeline|docker|kubernetes|cloud|infra)/i },
  { type: "documentation", pattern: /(document|documentation|readme|guide|specification|specs)/i },
  { type: "research", pattern: /(research|compare|evaluate|investigate|explore|feasibility)/i },
  { type: "maintenance", pattern: /(update|upgrade|sync|dependencies|bump|version)/i }
];

export class IssueSorter {
  constructor(options = {}) {
    this.complexityAnalyzer = options.complexityAnalyzer || new ComplexityAnalyzer();
  }

  classify(task) {
    const text = String(task || "").toLowerCase();
    const matched = [];
    for (const rule of CLASSIFICATION_RULES) {
      if (rule.pattern.test(text)) {
        matched.push(rule.type);
      }
    }
    return matched.length ? matched : ["feature"];
  }

  decomposeWorkPackages(normalized, complexity, classes) {
    const text = normalized.normalizedTask.toLowerCase();
    const domains = normalized.domains;

    // For simple, isolated tasks (LOW complexity), a single focused package is produced
    if (complexity.band === "LOW" && domains.length <= 1) {
      return [
        {
          id: "WP-1",
          title: `Execute ${normalized.intent}: ${normalized.normalizedTask}`,
          objective: normalized.normalizedTask,
          component: domains[0] || "core",
          dependencies: [],
          independent: true,
          skill_query: `${classes[0] || "engineering"} ${normalized.normalizedTask}`
        }
      ];
    }

    // For multi-faceted or complex tasks, decompose by domain and architectural component
    const packages = [];
    const domainOrder = ["database", "auth", "backend", "frontend", "devops", "security", "testing"];
    const detectedInOrder = domainOrder.filter(d => domains.includes(d) || text.includes(d));

    // If specific tiers detected, build dependency chain
    if (detectedInOrder.length > 1) {
      let prevId = null;
      for (const [i, domain] of detectedInOrder.entries()) {
        const wpId = `WP-${i + 1}`;
        const deps = [];
        // DB has no deps; Backend depends on DB/Auth; Frontend depends on Backend; DevOps depends on UI & Backend
        if (domain === "backend" && detectedInOrder.includes("database")) {
          deps.push("WP-1");
        } else if (domain === "frontend" && detectedInOrder.includes("backend")) {
          const backendIndex = detectedInOrder.indexOf("backend");
          deps.push(`WP-${backendIndex + 1}`);
        } else if (domain === "devops" && prevId) {
          deps.push(prevId);
        } else if (prevId && complexity.band === "MEDIUM") {
          deps.push(prevId);
        }

        packages.push({
          id: wpId,
          title: `${domain.toUpperCase()} Component: ${domain} layer for ${normalized.normalizedTask}`,
          objective: `Implement and configure ${domain} subsystem`,
          component: domain,
          dependencies: deps,
          independent: deps.length === 0,
          skill_query: `${domain} engineering ${classes[0] || "development"}`
        });
        prevId = wpId;
      }
    } else {
      // General decomposition based on engineering lifecycle phases
      packages.push({
        id: "WP-1",
        title: `Core Subsystem: ${normalized.normalizedTask}`,
        objective: `Implement core logic and structure`,
        component: "core",
        dependencies: [],
        independent: true,
        skill_query: `${classes[0] || "engineering"} core implementation`
      });
      packages.push({
        id: "WP-2",
        title: `Verification & Quality Assurance`,
        objective: `Add comprehensive tests and verification for core subsystem`,
        component: "testing",
        dependencies: ["WP-1"],
        independent: false,
        skill_query: `testing verification ${classes[0] || "engineering"}`
      });
    }

    return packages;
  }

  async sort(taskDescription, forcedWorkflow = null) {
    const normalized = normalizeTask(taskDescription);
    const classes = this.classify(normalized.normalizedTask);
    const complexity = await this.complexityAnalyzer.analyze(normalized.normalizedTask, normalized.domains);

    let workflow = "single-agent";
    if (forcedWorkflow && forcedWorkflow !== "auto") {
      workflow = forcedWorkflow === "multi" ? "multi-agent" : forcedWorkflow === "single" ? "single-agent" : forcedWorkflow;
    } else {
      workflow = complexity.band === "HIGH" ? "multi-agent" : "single-agent";
    }

    const workPackages = this.decomposeWorkPackages(normalized, complexity, classes);

    // Identify requirements
    const requirements = [
      `Complete ${normalized.intent} for "${normalized.normalizedTask}"`,
      "Ensure all affected interfaces maintain backwards compatibility or provide explicit migrations",
      "Verify changes with automated unit and integration tests"
    ];

    // Acceptance criteria
    const acceptanceCriteria = [
      `All work packages (${workPackages.map(wp => wp.id).join(", ")}) satisfy their objectives`,
      "Zero regressions introduced across existing tests",
      "Static analysis, trust checks, and compatibility validations pass"
    ];

    // Affected areas
    const affectedAreas = normalized.domains.map(d => `${d} layer`);
    if (!affectedAreas.length) affectedAreas.push("core application");

    // Dependencies
    const dependencies = [];
    if (normalized.domains.includes("database")) dependencies.push("Database schema migration access");
    if (normalized.domains.includes("auth")) dependencies.push("Authentication secret provider");
    if (normalized.domains.includes("devops")) dependencies.push("Container runtime or CI environment");

    // Risks
    const risks = [];
    if (complexity.risk >= 2) risks.push("High security or blast radius: sensitive credentials or data involved");
    if (complexity.technical_coupling >= 2) risks.push("Tight cross-tier coupling may cause cascading failures");
    if (complexity.uncertainty >= 1) risks.push("Uncertain domain requirements require early validation spikes");

    // Unknowns
    const unknowns = [];
    if (complexity.uncertainty >= 1) unknowns.push("Exact runtime deployment target and provider constraints");

    const triageRecord = {
      outcome: normalized.normalizedTask,
      class: classes,
      requirements,
      constraints: [
        "Run sort-the-issue before selecting implementation skills.",
        "Treat downloaded skills and third-party code as untrusted input.",
        "Do not claim multi-agent parallel execution if runtime only supports sequential execution."
      ],
      non_goals: [
        "Do not implement unrelated architectural overhauls outside the task scope.",
        "Do not auto-install unvetted third-party skills without authorization."
      ],
      acceptance_criteria: acceptanceCriteria,
      affected_areas: affectedAreas,
      dependencies,
      risks,
      unknowns,
      complexity,
      workflow,
      work_packages: workPackages,
      next_skill_queries: workPackages.map(wp => wp.skill_query)
    };

    const validation = validateTriageSchema(triageRecord);
    if (!validation.valid) {
      throw new Error(`Triage schema validation failed: ${validation.errors.join("; ")}`);
    }

    return triageRecord;
  }

  sortSync(taskDescription, forcedWorkflow = null) {
    const normalized = normalizeTask(taskDescription);
    const classes = this.classify(normalized.normalizedTask);
    const complexity = this.complexityAnalyzer.scoreHeuristics(normalized.normalizedTask, normalized.domains);

    let workflow = "single-agent";
    if (forcedWorkflow && forcedWorkflow !== "auto") {
      workflow = forcedWorkflow === "multi" ? "multi-agent" : forcedWorkflow === "single" ? "single-agent" : forcedWorkflow;
    } else {
      workflow = complexity.band === "HIGH" ? "multi-agent" : "single-agent";
    }

    const workPackages = this.decomposeWorkPackages(normalized, complexity, classes);

    const requirements = [
      `Complete ${normalized.intent} for "${normalized.normalizedTask}"`,
      "Ensure all affected interfaces maintain backwards compatibility or provide explicit migrations",
      "Verify changes with automated unit and integration tests"
    ];

    const acceptanceCriteria = [
      `All work packages (${workPackages.map(wp => wp.id).join(", ")}) satisfy their objectives`,
      "Zero regressions introduced across existing tests",
      "Static analysis, trust checks, and compatibility validations pass"
    ];

    const affectedAreas = normalized.domains.map(d => `${d} layer`);
    if (!affectedAreas.length) affectedAreas.push("core application");

    const dependencies = [];
    if (normalized.domains.includes("database")) dependencies.push("Database schema migration access");
    if (normalized.domains.includes("auth")) dependencies.push("Authentication secret provider");
    if (normalized.domains.includes("devops")) dependencies.push("Container runtime or CI environment");

    const risks = [];
    if (complexity.risk >= 2) risks.push("High security or blast radius: sensitive credentials or data involved");
    if (complexity.technical_coupling >= 2) risks.push("Tight cross-tier coupling may cause cascading failures");
    if (complexity.uncertainty >= 1) risks.push("Uncertain domain requirements require early validation spikes");

    const unknowns = [];
    if (complexity.uncertainty >= 1) unknowns.push("Exact runtime deployment target and provider constraints");

    const triageRecord = {
      outcome: normalized.normalizedTask,
      class: classes,
      requirements,
      constraints: [
        "Run sort-the-issue before selecting implementation skills.",
        "Treat downloaded skills and third-party code as untrusted input.",
        "Do not claim multi-agent parallel execution if runtime only supports sequential execution."
      ],
      non_goals: [
        "Do not implement unrelated architectural overhauls outside the task scope.",
        "Do not auto-install unvetted third-party skills without authorization."
      ],
      acceptance_criteria: acceptanceCriteria,
      affected_areas: affectedAreas,
      dependencies,
      risks,
      unknowns,
      complexity,
      workflow,
      work_packages: workPackages,
      next_skill_queries: workPackages.map(wp => wp.skill_query)
    };

    const validation = validateTriageSchema(triageRecord);
    if (!validation.valid) {
      throw new Error(`Triage schema validation failed: ${validation.errors.join("; ")}`);
    }

    return triageRecord;
  }
}
