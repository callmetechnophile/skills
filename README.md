# Automatic Agent Skill Router V0.2

A functional skill discovery, multi-factor ranking, trust verification, activation planning, and workflow orchestration engine for AI agent runtimes (Antigravity, Claude Code, Codex, Cursor, Gemini CLI, GitHub Copilot).

---

## 1. Architecture Overview

The system replaces keyword heuristics with a complete end-to-end routing pipeline:

```text
USER TASK
    │
    ▼
Task Normalization & Domain Detection
    │
    ▼
Sort-the-Issue Triage & Complexity Analysis (LOW / MEDIUM / HIGH)
    │
    ▼
Component-Bounded Work Package Decomposition & Dependency DAG
    │
    ▼
Multi-Registry Discovery (Local, Installed, SkillsDirectory)
    │
    ▼
Hybrid Ranking (BM25 + Semantic Cosine + 5-Factor Weighted Reranking)
    │
    ▼
Static Security & Trust Scanner (Exfiltration, Overrides, Destructive Commands)
    │
    ▼
Exact Upstream Directory Resolution (GitHub Tree Inspection)
    │
    ▼
Workflow Mode Selection (Mode A Single, Mode B Multi, Mode C Sequential Fallback)
    │
    ▼
Skill Activation Planning & SHA-256 Instruction Integrity Verification
```

---

## 2. Installation & Prerequisites

- **Node.js:** `>= 20.0.0`
- Zero external production dependencies; self-contained using modern Node.js standard libraries.

```bash
git clone https://github.com/callmetechnophile/skills.git
cd skills
npm test
```

---

## 3. CLI Commands & Usage

### 3.1 Task Triage (`sort`)
Decomposes an engineering prompt into testable requirements, risks, unknowns, and component work packages:
```bash
# Human-readable output
npm run sort -- "Fix authentication failures in Postgres"

# Machine-readable JSON output
npm run sort -- "Build an application with frontend, backend, database, and auth" --json
```

### 3.2 End-to-End Task Routing (`route`)
Triages the task, discovers candidate skills, assesses trust, plans activation, and creates execution schedule:
```bash
npm run route -- "Build a full-stack platform with frontend and database" --agent antigravity

# Machine-readable JSON execution plan
npm run route -- "Optimize SQL queries" --json --no-network
```

### 3.3 Skill Discovery (`find`)
Searches across local repository skills, installed workspace skills, and the remote catalogue using hybrid ranking:
```bash
npm run find -- "React performance optimization"
npm run find -- "database migration" --json --max-skills 5
```

### 3.4 Catalogue Synchronization (`sync:catalog`)
Synchronizes public skill metadata from SkillsDirectory with pagination and interrupted-session resumption:
```bash
npm run sync:catalog
npm run sync:catalog -- --page-size 50 --max-pages 2
npm run sync:catalog -- --restart
```

### 3.5 Exact Skill Installation (`install:skill`)
Resolves the exact upstream skill directory from the repository tree before generating installation commands. **Dry-run by default.**
```bash
# Plan installation (dry-run):
npm run install:skill -- --repo owner/repo --skill exact-skill-folder

# Execute installation after reviewing license and source:
npm run install:skill -- --repo owner/repo --skill exact-skill-folder --execute
```

### 3.6 Installed Skill Lifecycle (`list:skills`, `verify:skills`, `update:skills`)
Persistently tracks installed skills in `.installed-skills.json` and guards against silent modification:
```bash
# List all recorded skills:
npm run list:skills

# Verify cryptographic content hashes and scan for modifications:
npm run verify:skills

# Check for updates:
npm run update:skills
```

---

## 4. Key Subsystems

### 4.1 Hybrid Ranking Engine
Rank candidates using a 5-factor normalized score breakdown:
1. **Task Relevance (40%):** BM25 lexical token overlap + semantic vector similarity.
2. **Capability Completeness (20%):** Presence of descriptions, metadata, tags, and instructions.
3. **Provenance & Maintenance (15%):** Author reputation, verified flags, logarithmic star bonus.
4. **Security & Trust (15%):** Static scan trust status.
5. **Runtime Compatibility (10%):** Target agent runtime support.

### 4.2 Security & Trust Model
Treats all third-party skills as untrusted input.
- **Trust States:** `UNSCANNED`, `SCANNING`, `APPROVED`, `RESTRICTED`, `REJECTED`, `QUARANTINED`.
- **Static Rules:** Blocks prompt injection / developer instruction overrides, credential exfiltration, destructive shell operations (`rm -rf /`), base64 obfuscation, and unvetted downloads piped to shells.
- **Cryptographic Hashes:** Calculates SHA-256 digests for all loaded instructions.

### 4.3 Orchestration & Workflow Modes
- **Mode A (Single-agent):** For isolated, low-complexity tasks (`total_score <= 3`).
- **Mode B (Multi-agent):** For complex tasks with independent work packages when host subagent spawning is active.
- **Mode C (Sequential Fallback):** When multi-agent workflow is requested or needed, but the current runtime environment is single-session, the router schedules work packages sequentially in topological dependency order without falsely claiming parallel execution.

---

## 5. Implementation Status Matrix

| Feature | Status | Description |
|---|---|---|
| Issue Triage & Schema Validation | **Implemented** | Deterministic heuristics + structured LLM interface with fallback |
| Work Package Decomposition | **Implemented** | Architectural component splitting with DAG dependencies |
| Multi-Registry Discovery | **Implemented** | `LocalRegistry`, `InstalledRegistry`, `SkillsDirectoryRegistry` |
| Upstream Directory Resolution | **Implemented** | Inspects GitHub tree; rejects ambiguous matches |
| Hybrid 5-Factor Ranking | **Implemented** | BM25 + n-gram semantic + explainable breakdown |
| Security & Trust Scanner | **Implemented** | Static scanner with 6 trust states and SHA-256 integrity |
| Skill Activation Planner | **Implemented** | Verification reports with content hash checks |
| Single-Agent Orchestration | **Implemented** | Mode A sequential progressive loading |
| Sequential Fallback | **Implemented** | Mode C honest linearization when subagents are unavailable |
| Lifecycle Tracking Ledger | **Implemented** | `.installed-skills.json` with modification detection |
| Parallel Subagent Execution | **Externally Managed** | Scheduled by router; dispatched when host agent runtime exposes API |
| Live Embedding Provider | **Experimental** | Pluggable interface; defaults to deterministic local n-gram vectorizer |
| Automatic Arbitrary Script Execution | **Not Supported** | Intentionally blocked for security |

---

## 6. Testing & Validation

Run the complete automated test suite (33 unit, integration, and E2E scenario tests):
```bash
npm test
```

The test suite covers:
- Task normalization, complexity scoring, and issue triage schema validation
- Multi-registry discovery, catalog pagination, and interrupted-resume sync
- Exact directory resolution, ambiguous match rejection, and command formatting
- 5-factor hybrid ranking, star decoupling, and explainable breakdowns
- Static security scanning (prompt injection, exfiltration, destructive commands)
- Installed skill lifecycle, tamper detection, and overwrite protection
- End-to-end scenarios 1 through 7 (Simple task, Full-stack task, Missing skill, Unsafe skill, Unsupported runtime, Interrupted install, and Local modification).

---

## 7. Release Roadmap

- **V0.1 (Legacy Prototype):** Initial proof-of-concept exploring keyword discovery and basic routing.
- **V0.2 (Stable Node.js Baseline — Frozen):** Complete engineering remediation. Functional task intelligence, 5-factor hybrid ranking, static security scanning, exact Git tree directory resolution, DAG wave orchestration with Mode C sequential fallback, lifecycle tracking, and 33 passing automated tests. Tag: `v0.2.0`.
- **V0.3 (Rust Core Engine — Active Specification):** Migration of the core routing and discovery engine into a high-performance, modular Cargo workspace (`router-core`, `task-intelligence`, `skill-discovery`, `skill-ranking`, `skill-security`, `skill-activation`, `agent-orchestrator`, `skill-lifecycle`, `router-cli`). Provides single self-contained native binary distribution, sub-15ms cold start, and Tantivy-powered search.
- **V0.4 (Enterprise & Local Neural Models):** Local ONNX Runtime neural embeddings, SQLite catalogue indexing for 100k+ skills, and interactive terminal UI.

