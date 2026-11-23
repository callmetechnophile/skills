---
name: automatic-skill-router
description: Discovers, ranks, validates and loads task-specific agent skills from the local catalogue and approved upstream registries. Use after sort-the-issue to select one skill at a time for single-agent workflows or specialist skill bundles for multi-agent workflows, with safe fallback when sub-agents are unavailable.
---

# Automatic Skill Finder and Router

## Invocation contract

This router runs immediately after `sort-the-issue`. It does not replace triage. It decides which skill(s) are required for the next work package and when they should be loaded.

## Inputs

- Triage YAML from `sort-the-issue`.
- User prompt and repository context.
- Available skill inventory and supported install/load mechanisms.
- Agent workflow mode and sub-agent capability.
- Security policy and allowed upstream sources.

## Routing algorithm

1. Convert each work package into a concise capability query: action + domain + artifact + constraints.
2. Search the local installed-skill index first.
3. If no adequate skill exists, search approved remote registries (SkillsDirectory, skills.sh, known Git repositories).
4. Rank candidates by: semantic task fit (40%), verified capability and completeness (20%), provenance/maintenance/version quality (15%), security findings/grade (15%), agent/runtime compatibility (10%).
5. Reject candidates with unknown provenance, suspicious scripts, missing license metadata where redistribution is needed, or disallowed security findings.
6. Return the best fit and explain why. Load/install only when needed; do not install every search result.
7. After loading, confirm skill name, source, version/commit, path and required tools are available.
8. If no candidate passes, use a minimal internal procedure and report that no vetted skill was found.

## Workflow rules

### Single-agent workflow
- Search, select and load one skill for the immediate next step.
- Execute its bounded task and validate its output.
- Repeat discovery after the previous step, because later required skills may depend on actual results.
- Do not preload several unrelated skills.

### Multi-agent workflow
- Create one work package per independent specialist stream.
- Discover relevant skills for all packages; load only the skill(s) each agent requires.
- Launch specialist agents in parallel only when the runtime exposes an actual agent-spawn mechanism.
- Every agent receives: task statement, relevant files/context, selected skill, allowed file ownership, output contract, tests, and stop conditions.
- Prevent overlapping writes. Integrator owns shared interfaces and final merge.
- If agent support is missing, downgrade to sequential single-agent execution with an explicit notice.

### Complex task on a single-agent runtime
- Prefer bounded sub-agents when supported.
- Otherwise schedule specialist work packages sequentially, preserving dependency order and hand-off artifacts.
- Never emulate parallel execution by merely writing multiple role prompts and claiming concurrent work.

## Load policy

- Use existing installed skills before downloading.
- For remote skills, prefer official registry install commands and pin source commit.
- Read SKILL.md before enabling a skill. Inspect scripts, references, dependencies and network calls.
- Store upstream skills under `skills/third-party/<owner>-<repo>/<skill-name>/` only when license permits redistribution.
- Store metadata-only references when redistribution is not permitted.
- Record source URL, owner, repository, commit SHA, license, security grade, scan time and local content hash in `catalog/manifest.json`.
- Do not run arbitrary install hooks or downloaded code.

## Decision output

```yaml
routing:
  workflow: single-agent|multi-agent|single-agent-with-subagents
  selected:
    - work_package: ""
      skill: ""
      source: ""
      rationale: ""
      security: ""
      load_action: local|install|metadata-only|fallback
  execution_order: []
  parallel_groups: []
  constraints: []
  fallback: ""
```

## Stop conditions

Stop and request user authorization before installing a skill that requires elevated permissions, adding a paid dependency, running scripts, accessing credentials, sending network data beyond normal registry requests, or modifying files outside the target workspace.
