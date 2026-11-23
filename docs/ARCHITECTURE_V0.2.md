# Architecture Specification — Automatic Agent Skill Router V0.2

## 1. System Overview

The Automatic Agent Skill Router V0.2 provides automated skill discovery, multi-factor ranking, static trust verification, activation planning, and dependency-aware workflow orchestration for AI agent runtimes.

```text
USER TASK
    |
    v
TASK NORMALIZATION (`src/intelligence/task-normalizer.mjs`)
    |
    v
SORT-THE-ISSUE (`src/intelligence/issue-sorter.mjs`)
    |
    v
COMPLEXITY & RISK ANALYSIS (`src/intelligence/complexity-analyzer.mjs`)
    |
    v
SKILL DISCOVERY (`src/discovery/`)
    ├── LocalRegistry (`src/discovery/local-registry.mjs`)
    ├── InstalledRegistry (`src/discovery/installed-registry.mjs`)
    └── SkillsDirectoryRegistry (`src/discovery/skillsdirectory-registry.mjs`)
    |
    v
HYBRID RANKING & SCORING (`src/ranking/hybrid-ranker.mjs`)
    ├── Lexical BM25 (`src/ranking/lexical-matcher.mjs`)
    ├── Pluggable Embedding (`src/ranking/embedding-matcher.mjs`)
    └── 5-Factor Weighted Reranker (Relevance, Completeness, Provenance, Trust, Compatibility)
    |
    v
SECURITY & TRUST FILTER (`src/security/trust-scanner.mjs`, `src/security/trust-policy.mjs`)
    |
    v
EXACT DIRECTORY RESOLUTION (`src/discovery/directory-resolver.mjs`)
    |
    v
WORKFLOW ORCHESTRATION & DAG (`src/orchestration/dependency-graph.mjs`)
    ├── Mode A: Single-agent (isolated task)
    ├── Mode B: Multi-agent parallel waves (when runtime supports subagents)
    └── Mode C: Sequential fallback (honest serial execution when runtime is single-session)
    |
    v
ACTIVATION PLANNING & VERIFICATION (`src/activation/activation-planner.mjs`)
    |
    v
EXECUTION HANDOFF & REPORT (`src/activation/activation-report.mjs`)
```

---

## 2. Core Subsystems

### 2.1 Task Intelligence (`src/intelligence/`)
- **TaskNormalizer:** Normalizes whitespace, extracts intent verbs, and identifies domain flags (`frontend`, `backend`, `database`, `auth`, `devops`, `security`, `testing`).
- **ComplexityAnalyzer:** 5-dimension rubric (scope breadth, technical coupling, uncertainty, risk/blast radius, parallelism) scored 0-2. Yields total score (0-10) and band: `LOW` (0-3), `MEDIUM` (4-6), `HIGH` (7-10). Supports deterministic heuristic analysis and pluggable LLM provider with fallback.
- **IssueSorter:** Produces structured triage adhering to JSON schema (`src/intelligence/schemas/task-schema.mjs`). Decomposes complex tasks into component-bounded work packages (`WP-1`, `WP-2`, etc.) with explicit cross-package dependencies.

### 2.2 Discovery Engine (`src/discovery/`)
- **SkillRegistry Abstract Base:** Standardized interface: `search()`, `getMetadata()`, `getVersions()`, `resolveSource()`, `retrieveSkillContent()`, `verifyProvenance()`.
- **LocalRegistry:** Discovers core repository skills (`skills/core/`, `skills/third-party/`) and catalog metadata.
- **InstalledRegistry:** Discovers skills installed in workspace and tracked in `.installed-skills.json`.
- **SkillsDirectoryRegistry:** Queries remote registry index and coordinates resumable catalog sync.
- **DirectoryResolver:** Inspects upstream GitHub Git tree (`api.github.com/repos/{owner}/{repo}/git/trees/{branch}?recursive=1`) to locate the exact folder containing `SKILL.md`. Rejects ambiguous matches and never fabricates names.

### 2.3 Ranking & Scoring Engine (`src/ranking/`)
- **LexicalMatcher:** BM25 term frequency / inverse document frequency with field weighting (Name: 3x, Tags: 2x, Description: 1x, Category: 1x).
- **EmbeddingMatcher:** Pluggable semantic cosine similarity with zero-cost deterministic local character n-gram fallback.
- **HybridRanker:** 5-factor normalized reranking:
  1. Task Relevance: 40%
  2. Capability Completeness: 20%
  3. Provenance & Maintenance: 15%
  4. Security & Trust: 15%
  5. Runtime Compatibility: 10%
  Exposes explainable score breakdown. Rejects skills with critical security findings.

### 2.4 Security & Trust Layer (`src/security/`)
- **TrustScanner:** Static scanner inspecting `SKILL.md`, shell commands, and scripts. Detects credential exfiltration (`curl -d @.env`), prompt injection / instruction override (`ignore developer instructions`), destructive commands (`rm -rf /`), obfuscated base64 code, and piped downloads.
- **Trust States:** `UNSCANNED`, `SCANNING`, `APPROVED`, `RESTRICTED`, `REJECTED`, `QUARANTINED`.
- **Integrity:** SHA-256 cryptographic hashing of skill instructions and directories.

### 2.5 Orchestration & Workflow Scheduling (`src/orchestration/`)
- **DependencyGraph:** Constructs DAG from work package dependencies, validates against circular dependencies (Kahn's algorithm), and groups independent packages into parallel execution waves.
- **Workflow Modes:**
  - **Mode A (Single-agent):** One step at a time for localized tasks.
  - **Mode B (Multi-agent):** Parallel dispatch to specialist subagents across dependency waves.
  - **Mode C (Sequential Fallback):** When multi-agent workflow is requested or needed, but live subagent spawning is unavailable in the execution environment, honestly linearizes packages without emulating fake concurrency.
- **Runtime Adapters:** Adapters for `Antigravity`, `Claude Code`, and `Codex`.

### 2.6 Activation Engine (`src/activation/`)
- Separates the 5 lifecycle stages: Discovery -> Selection -> Installation -> Activation -> Execution.
- Formulates activation plans, verifies loaded instruction integrity via SHA-256 hashes, records activations, and generates structured Activation Reports.

### 2.7 Lifecycle Management (`src/lifecycle/`)
- **InstalledTracker:** Persistently records installed skills in `.installed-skills.json`.
- Detects local file modifications via hash comparison and blocks silent overwrites unless `--force` is explicitly provided.
- Commands: `npm run list:skills`, `npm run verify:skills`, `npm run update:skills`.
