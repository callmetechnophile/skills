# Issue Analysis V0.2: Automatic Agent Skill Router Remediation

## 1. Problem Statement
The Automatic Agent Skill Router (V0.1) is an early-stage prototype that contains documentation, policy specifications, and basic scripts, but lacks functional implementation for critical advertised capabilities. Specifically:
- Issue triage relies purely on simplistic regex matching and always produces a single generic work package ("WP-1") regardless of complexity.
- Skill ranking uses an ad-hoc token count and star-count formula instead of the documented hybrid multi-factor ranking architecture.
- Skill discovery is hardcoded to a single remote endpoint or local JSON file without a unified registry abstraction for local skills, installed skills, and multiple remote registries.
- Skill directory resolution naively slugifies display names rather than inspecting the actual upstream repository tree and locating valid `SKILL.md` definitions, leading to failed or fabricated installations.
- Security and trust assessment (static analysis, permission scanning, trust states, content hashing) is entirely missing.
- Skill activation planning (distinguishing discovery, selection, installation, activation, and execution) is absent.
- Agent workflow orchestration (Modes A, B, C; dependency graphs; runtime adapters for Antigravity, Claude Code, Codex) is documentation-only.
- Installed skill lifecycle management (`list:skills`, `verify:skills`, `update:skills`) is not implemented.
- Repository hygiene issues exist: `.gitignore` contains literal `\n` escape sequences, and license definitions are contradictory (Apache-2.0 root vs MIT in `catalog/manifest.json`).

The primary objective is to convert this prototype into a robust, testable, functional V0.2 skill discovery, selection, activation-planning, and workflow orchestration system.

---

## 2. Existing System Behavior vs. Expected System Behavior

| Dimension | Existing System Behavior (V0.1) | Expected System Behavior (V0.2) |
|---|---|---|
| **Git Hygiene & Config** | `.gitignore` has literal `\n` characters on a single line, failing to ignore files. Root `LICENSE` is Apache-2.0, but `catalog/manifest.json` claims MIT for core skills. | Valid multiline `.gitignore` with tests. Consistent licensing and attribution across all catalog entries and core skills. |
| **Task Intelligence & Triage** | `src/router.mjs:sortIssue()` uses 5 regexes to score complexity; outputs a single work package (`WP-1`) even on `HIGH` complexity tasks. No schema validation. | Layered task intelligence: deterministic heuristics + structured LLM provider interface with deterministic fallback; real decomposition into component-based work packages with dependency ordering and JSON schema validation. |
| **Skill Discovery** | `scripts/find-skill.mjs` queries a single index file or makes raw HTTP calls to SkillsDirectory. No local skill discovery, no installed-skill inventory discovery. | Unified `SkillRegistry` interface (`LocalRegistry`, `InstalledRegistry`, `SkillsDirectoryRegistry`, adapter mechanism) with pagination, resume, schema validation, and offline mode (`--no-network`). |
| **Upstream Directory Resolution** | `scripts/find-skill.mjs` derives directory names via regex slugification (`replace(/[^a-z0-9]+/g, "-")`), fabricating folder names. | Exact upstream repository tree inspection (via GitHub API/tree lookup with caching/mocking) finding actual `SKILL.md` paths; rejects ambiguous matches. |
| **Skill Ranking** | `src/registry.mjs:rankSkills()` computes score as token overlap + log10(stars). No breakdown or normalization. | Hybrid ranking engine: Lexical/BM25 + tag/name match + optional semantic/embedding stage + 5-factor reranking (Relevance 40%, Completeness 20%, Provenance 15%, Security 15%, Compatibility 10%) with explainable breakdown. |
| **Security & Trust Layer** | No static inspection or scanning. Security status in manifest is static strings. | Static inspection engine analyzing `SKILL.md`, scripts, commands, network calls, exfiltration patterns, system instruction override attempts. Trust states: `UNSCANNED`, `SCANNING`, `APPROVED`, `RESTRICTED`, `REJECTED`, `QUARANTINED`. Content hashing (SHA-256). |
| **Activation Engine** | No activation planning or verification. Execution claims are untested. | Explicit separation of Discovery -> Selection -> Installation -> Activation -> Execution. Generates verifiable activation reports and verifies loaded instruction integrity. |
| **Orchestration & Workflows** | Hardcoded text strings in `routeTask()`. No dependency graph or scheduling. | Mode A (Single-agent), Mode B (Multi-agent with DAG and parallel groups), Mode C (Sequential fallback). Runtime adapters for Antigravity, Claude Code, and Codex. |
| **Installed Skill Lifecycle** | None. `scripts/install-skill.mjs` runs `skills add` with no local ledger or integrity tracking. | Persistent installed-skill registry (`.installed-skills.json`), supporting `list:skills`, `verify:skills`, `update:skills`, change detection, tamper prevention. |
| **CLI & Testing** | 5 simple assertions; basic npm scripts. | Comprehensive CLI supporting flags (`--json`, `--verbose`, `--dry-run`, `--agent`, `--workflow`, `--max-skills`, `--no-network`), unit tests, integration tests, E2E tests covering 7 mandatory scenarios, and upgraded CI. |

---

## 3. Functional Requirements
- **FR-01 (Task Decomposition):** Provide structured issue analysis with schema validation, extracting classification, requirements, affected areas, dependencies, risks, unknowns, acceptance criteria, complexity score (scope breadth, coupling, uncertainty, risk, parallelism), and component-bounded work packages.
- **FR-02 (Discovery Engine):** Discover skills from local repository catalog, installed skill ledger, and remote registries through a unified `SkillRegistry` interface.
- **FR-03 (Catalogue Sync):** Resumable pagination and synchronization with deduplication, atomic write (`.tmp` rename), rate-limit/timeout handling, and schema validation.
- **FR-04 (Directory Resolution):** Resolve exact upstream skill directories by inspecting repository structure, locating `SKILL.md`, and rejecting ambiguous/non-existent targets.
- **FR-05 (Hybrid Ranking):** Rank skills using BM25/token retrieval + configurable multi-factor reranking with normalized scores and explainable breakdowns.
- **FR-06 (Security & Trust):** Static scanner detecting credential exfiltration, prompt injection / override, destructive shell commands, privilege escalation, external network calls, obfuscation. Assign trust states and compute SHA-256 hashes.
- **FR-07 (Activation Planning):** Formulate activation plans with explicit prerequisites, compatibility checks, trust gate checks, and output activation verification reports.
- **FR-08 (Workflow Orchestration):** Generate execution plans for Mode A (Single-agent), Mode B (Multi-agent DAG), and Mode C (Sequential fallback) with runtime adapters for Antigravity, Claude Code, and Codex.
- **FR-09 (Lifecycle Management):** Persistently track installed skills, verify integrity via content hashes, detect updates, and prevent silent overwrites.
- **FR-10 (CLI Suite):** Deliver operational CLI commands (`sort`, `route`, `find`, `sync:catalog`, `install:skill`, `list:skills`, `verify:skills`, `update:skills`) with standard flags (`--json`, `--verbose`, `--dry-run`, `--agent`, etc.) and distinct exit codes.

---

## 4. Non-Functional Requirements
- **NFR-01 (Performance & Determinism):** Sub-second response for local discovery, ranking, triage, and activation planning without external network dependencies.
- **NFR-02 (Safety & Least Privilege):** Safe defaults: dry-run by default for installation; zero automatic execution of arbitrary third-party scripts.
- **NFR-03 (Offline Resiliency):** Full test suite and local operations must work completely offline (`--no-network` support; mocked HTTP clients).
- **NFR-04 (Extensibility & Modularity):** Decoupled architecture with clear interfaces for registries, LLM providers, embedding models, and runtime adapters.
- **NFR-05 (Compatibility):** Support Node.js 20+ across Linux, macOS, and Windows.

---

## 5. Affected Modules & Proposed Repository Structure
- `.gitignore`: Fix newline formatting.
- `LICENSE` & `catalog/manifest.json`: Align licensing and attribution.
- `src/intelligence/`:
  - `task-normalizer.mjs`: Task normalization and parameter extraction.
  - `complexity-analyzer.mjs`: Multi-dimensional complexity scoring (deterministic heuristics + model interface fallback).
  - `issue-sorter.mjs`: Structured issue decomposition and work-package partitioning.
  - `schemas/`: JSON schemas for task intelligence and triage output.
- `src/discovery/`:
  - `registry-base.mjs`: Common registry abstraction.
  - `local-registry.mjs`: Discovery from local `skills/` and `catalog/manifest.json`.
  - `installed-registry.mjs`: Discovery from local installed skills ledger.
  - `skillsdirectory-registry.mjs`: Discovery and synchronization adapter for SkillsDirectory.
  - `directory-resolver.mjs`: Exact upstream skill tree inspection and verification.
- `src/ranking/`:
  - `lexical-matcher.mjs`: BM25 and token-level matching.
  - `embedding-matcher.mjs`: Pluggable semantic/embedding interface with lexical fallback.
  - `hybrid-ranker.mjs`: Multi-factor reranking engine with explainable score breakdown.
- `src/security/`:
  - `trust-scanner.mjs`: Static rule-based AST/regex vulnerability & exfiltration scanner.
  - `trust-policy.mjs`: Trust states (`UNSCANNED`, `SCANNING`, `APPROVED`, `RESTRICTED`, `REJECTED`, `QUARANTINED`) and approval policies.
  - `integrity.mjs`: Cryptographic hash calculation (SHA-256) and tamper detection.
- `src/activation/`:
  - `activation-planner.mjs`: Phased pipeline (Discover -> Select -> Install -> Activate -> Execute).
  - `activation-report.mjs`: Verification and loading audit reporter.
- `src/orchestration/`:
  - `workflow-selector.mjs`: Mode A, Mode B, Mode C selection logic.
  - `dependency-graph.mjs`: DAG builder, topological sort, parallel wave calculator.
  - `runtime-adapters/`: Adapters for Antigravity, Claude Code, and Codex.
- `src/lifecycle/`:
  - `installed-tracker.mjs`: Ledger management (`.installed-skills.json`), update checker, verifier.
- `src/cli/`:
  - Unified CLI command entry points and option parsers.
- `scripts/`: Update existing scripts to delegate to the new modular architecture.
- `tests/`:
  - `tests/unit/`: Comprehensive unit tests for all modules.
  - `tests/integration/`: Integration tests for sync, discovery, lifecycle, and resolution.
  - `tests/e2e/`: E2E tests for the 7 required scenarios.
- `.github/workflows/`: Enhanced CI pipeline.
- `docs/`: `ARCHITECTURE_V0.2.md`, `SECURITY.md`, `COMPATIBILITY.md`.

---

## 6. Technical Dependencies
- Node.js standard libraries (`node:fs`, `node:path`, `node:crypto`, `node:child_process`, `node:test`, `node:assert`, `node:url`).
- Zero unnecessary external production dependencies; keep installation lightweight and self-contained using modern Node.js 20+ features.

---

## 7. Security Risks & Mitigations
- **Prompt Injection & Instruction Overrides:** Third-party `SKILL.md` files may contain instructions intended to bypass system rules or exfiltrate environment secrets. *Mitigation:* Static rule scanning flags prompt injection, exfiltration endpoints, and forbids skills from being treated as system-level instructions.
- **Arbitrary Code Execution:** Malicious npm hooks or shell scripts executed during skill installation. *Mitigation:* Dry-run default, explicit `--execute` requirement, static script scanning, and isolation from auto-execution.
- **Credential Leakage:** Router or CLI logging environment variables or API keys. *Mitigation:* Explicit filtering of secrets in logs, reports, and JSON outputs; secret-scanning quality gate in CI.
- **Tampered Local Skills:** User modifications or unauthorized file modifications overwritten. *Mitigation:* SHA-256 hash validation before update/installation; abort on hash mismatch without explicit override.

---

## 8. Unknowns & Assumptions
- **SkillsDirectory API Availability:** Public endpoint `/api/registry` might be rate-limited or unavailable during offline testing. *Assumption:* Tests must use mocked HTTP fixtures and offline mode.
- **Runtime Sub-agent APIs:** Sub-agent capabilities vary across host runtimes (Antigravity, Claude Code, Codex). *Assumption:* Treat sub-agent execution as runtime-dependent; provide execution plans with fallback to Mode C (sequential) when runtime execution is unconfirmed.

---

## 9. Acceptance Criteria (Mapping to Prompt Section 17)
- **AC-01:** `.gitignore` contains valid newline-separated rules and is verified by a test.
- **AC-02:** Repository `LICENSE` and catalog metadata have consistent licensing and attribution.
- **AC-03:** `sort-the-issue` produces validated structured output with component-based work packages.
- **AC-04:** Router discovers local skills in `skills/` and `catalog/manifest.json`.
- **AC-05:** Router discovers remote registry metadata via `SkillRegistry` abstraction.
- **AC-06:** Catalogue sync supports pagination, resumption from offset, and error recovery.
- **AC-07:** Skill ranking returns explainable scores with 5-factor breakdown.
- **AC-08:** Exact upstream skill directory resolution inspects tree and locates valid `SKILL.md`.
- **AC-09:** Unsafe skills are rejected or quarantined by the security scanner and cannot be activated.
- **AC-10:** Runtime compatibility is checked across Antigravity, Claude Code, and Codex.
- **AC-11:** Activation reports reflect actual loaded verification status.
- **AC-12:** Complex tasks produce dependency-aware work packages with topological scheduling.
- **AC-13:** Unsupported multi-agent execution falls back to sequential execution with explicit disclosure.
- **AC-14:** Installation defaults to safe dry-run requiring `--execute` for action.
- **AC-15:** Installed skill versions, commit SHAs, and SHA-256 hashes are tracked persistently.
- **AC-16:** Failure states are accurately recorded without false success.
- **AC-17:** Automated tests cover unit, integration, and all 7 E2E scenarios.
- **AC-18:** CI passes all tests, syntax checks, and quality gates.
- **AC-19:** README matches actual implemented behavior.
- **AC-20:** No placeholder or stub implementation is presented as complete.

---

## 10. Work Packages

```text
WP-1: Repository Hygiene & Licensing (AC-01, AC-02)
      - Fix .gitignore, add validation test.
      - Align Apache-2.0 / MIT catalog licensing and add attribution docs.

WP-2: Task Intelligence & Structured Issue Decomposition (AC-03, AC-12)
      - Implement TaskNormalizer, ComplexityAnalyzer, IssueSorter, and JSON schema validators.
      - Component-based work package decomposition and dependency detection.

WP-3: Unified Registry & Exact Directory Resolution (AC-04, AC-05, AC-06, AC-08)
      - Implement SkillRegistry abstraction (LocalRegistry, InstalledRegistry, SkillsDirectoryRegistry).
      - Resumable catalogue sync and upstream repository tree directory resolver.

WP-4: Hybrid Ranking & Scoring Engine (AC-07)
      - Implement BM25 lexical matcher, pluggable embedding interface with fallback, and 5-factor reranker.

WP-5: Security, Trust & Static Vulnerability Scanner (AC-09, AC-16)
      - Implement TrustScanner, trust state machine, policy gate, and SHA-256 integrity manager.

WP-6: Activation Engine & Runtime Orchestration (AC-10, AC-11, AC-12, AC-13)
      - Implement ActivationPlanner, ActivationReport, DependencyGraph scheduler, and runtime adapters.

WP-7: Installed Skill Lifecycle Management (AC-14, AC-15, AC-16)
      - Implement InstalledTracker, ledger persistence, list/verify/update CLI operations, tamper protection.

WP-8: CLI Unification, Quality Gates & Automated Test Suite (AC-17, AC-18)
      - CLI flags (--json, --dry-run, --verbose, --no-network), Unit, Integration, and E2E Scenarios 1-7.
      - Upgrade GitHub Actions CI.

WP-9: Documentation & Architectural Specifications (AC-19, AC-20)
      - Update README.md, AGENTS.md, ARCHITECTURE_V0.2.md, SECURITY.md, COMPATIBILITY.md.
```

---

## 11. Execution Strategy
- **Complexity Assessment:**
  - Scope breadth: 2 (cross-cutting: CLI, intelligence, discovery, ranking, security, activation, orchestration, lifecycle)
  - Subsystem coupling: 2 (tight contracts between triage, discovery, ranking, security, activation, and orchestration)
  - Uncertainty: 1 (clear specifications from master prompt; need mockable interfaces for external network/runtimes)
  - Risk / blast radius: 1 (remediating existing repo without breaking contract)
  - Parallelism: 2 (work packages have well-defined boundaries and interfaces)
  - **Total Score: 8 / 10 -> HIGH Complexity**
- **Execution Mode:**
  - Although the task has HIGH complexity with multiple modular work packages, execution in the current environment will follow **sequential specialist workstreams** with rigorous bounded work packages (Mode C sequential strategy) to ensure flawless integration without file contention or race conditions.
  - Every work package is verified with automated tests before advancing to the next.
