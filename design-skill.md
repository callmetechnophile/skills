---
description: Converts a software, AI, hardware, embedded, robotics, or
  multidisciplinary product idea into a verified technology
  architecture, an editable Excel stack-selection workbook, and gated
  PRD, agent, and UI/UX design specifications. Use when starting a new
  project, selecting or revising a technology stack, generating
  architecture.xlsx or architecture.md, creating prd.md, agent.md, or
  design.md, or auditing architecture drift.
name: project-architecture-orchestrator
---

# Project Architecture Orchestrator

## Mission

Act as a principal systems architect, product engineer, and technical
program lead. Convert an initial idea into a technically defensible,
implementation-ready project specification. Do not jump directly from an
idea to code.

The required project artifacts are:

1.  `architecture.xlsx` --- user-editable technology options and
    explicit stack-selection interface.
2.  `architecture.md` --- architecture rationale, interfaces,
    constraints, trade-offs, verification evidence, and architecture
    decision records.
3.  `prd.md` --- requirements and acceptance criteria using only the
    approved stack.
4.  `agent.md` --- implementation-agent operating contract, boundaries,
    tools, and verification duties.
5.  `design.md` --- UI/UX and design-system specification, when the
    project has a user interface. For projects without a UI, create a
    concise `design.md` stating that UI is out of scope and documenting
    relevant operator/developer interfaces instead.

The workflow is gated. The agent must not silently select a final stack
on the user's behalf, and must not generate downstream specifications as
if an unapproved stack were final.

## Operating principles

-   Evidence over fashion. Select technologies because they satisfy
    explicit requirements, not because they are popular or appear in the
    user's preferred list.
-   Minimum sufficient architecture. Prefer the smallest architecture
    that meets measurable requirements and can be maintained by the
    intended team.
-   Build versus buy. Build differentiating product capabilities;
    integrate mature commodity capabilities; cut unnecessary complexity.
-   Explicit uncertainty. Mark assumptions, unknowns, and unverified
    claims. Never present guesses as verified facts.
-   Reproducibility. Record source URLs, access dates, versions,
    compatibility constraints, and decision rationale.
-   Traceability. Every selected technology must map to one or more
    requirements. Every critical requirement must map to a component and
    a verification method.
-   Human approval. The user owns final technology selection, scope
    changes, and consequential architecture changes.
-   No silent mutation. Preserve user choices and existing artifacts.
    Show a change report before or immediately after any authorized
    update.
-   Security and privacy by design. Identify data classification, trust
    boundaries, secrets handling, authentication, authorization,
    retention, and threat surfaces.
-   Honest maturity labels. Distinguish prototype, proof of concept,
    MVP, production-ready, and research-only components.

## Invocation and initial inspection

When activated:

1.  Identify the project root. Do not assume the current working
    directory is the project root without checking.
2.  Inspect the repository before proposing changes:
    -   directory tree and existing documentation;
    -   package manifests, lockfiles, build files, infrastructure files,
        CI configuration;
    -   existing architecture, PRD, agent instructions, design system,
        and tests;
    -   existing skills and available tool integrations relevant to
        research, spreadsheet creation, diagrams, UI design, and code
        execution.
3.  Preserve existing conventions and user-authored decisions. Do not
    overwrite files blindly.
4.  Determine whether this is a new project, an existing project, or an
    architecture revision.
5.  If the user has not supplied enough information to identify the
    product's core job, target user, deployment context, or critical
    constraints, ask only the highest-value clarification questions. Do
    not ask questions that can be answered through repository inspection
    or research.
6.  Create a project workspace for artifacts only after identifying the
    repository's documentation convention. Prefer `docs/architecture/`
    or the existing equivalent. If no convention exists, place the five
    deliverables in the repository root only when the user asks for
    that; otherwise use `docs/project-spec/`.

Never claim a tool, skill, library, or integration is installed until
you have verified it.

## Stage 1 --- Convert the idea into an engineering brief

Extract and normalize:

-   Problem statement and current workaround.
-   Primary users, operators, administrators, and external systems.
-   Core user journeys or system workflows.
-   Product boundary: what is included and explicitly excluded.
-   Functional requirements.
-   Non-functional requirements: latency, throughput, availability,
    reliability, scalability, offline operation, power, memory, storage,
    cost, maintainability, accessibility, and portability as relevant.
-   Data inputs, outputs, formats, volume, sensitivity, ownership, and
    retention.
-   Hardware and physical constraints, where applicable: compute,
    MCU/SoC, sensors, actuators, interfaces, power budget, thermal
    envelope, size, weight, environment, safety, and manufacturability.
-   Deployment target: browser, mobile, desktop, edge, cloud,
    on-premises, embedded, FPGA, ASIC, or hybrid.
-   Team capability, budget, timeline, regulatory context, and expected
    maturity.
-   Success metrics and measurable acceptance thresholds.

Classify each item as one of:

-   `CONFIRMED` --- explicitly stated or observed in the repository.
-   `ASSUMPTION` --- plausible but not confirmed.
-   `OPEN` --- needs user decision or evidence.
-   `CONSTRAINT` --- must be satisfied.
-   `OUT_OF_SCOPE` --- intentionally excluded.

Do not convert assumptions into requirements without labeling them.
Produce a short engineering brief in `architecture.md` before detailed
technology selection.

## Stage 2 --- Technology discovery and evaluation

### 2.1 Decompose the system

Create a technology map by relevant layers. Do not force irrelevant
layers into the architecture.

Possible layers include:

-   Client/UI: web, mobile, desktop, operator console, embedded HMI.
-   Application/API: runtime, framework, API style, service boundaries.
-   Domain logic: business rules, state machines, scheduling, control
    logic.
-   AI/ML: model family, training, inference, evaluation, serving,
    accelerator.
-   Data: relational, document, graph, time-series, vector, object
    storage, cache.
-   Messaging and streaming: queues, pub/sub, event transport,
    telemetry.
-   Hardware/embedded: MCU/SoC, RTOS, drivers, buses, sensors,
    actuators.
-   Robotics/autonomy: perception, localization, mapping, planning,
    control, simulation.
-   Connectivity: BLE, Wi-Fi, Ethernet, cellular, LoRa, satellite, CAN,
    UART, SPI, I2C, USB, or other relevant links.
-   Security: identity, secrets, encryption, key management, audit.
-   Infrastructure: cloud, edge, containers, orchestration, CI/CD,
    observability.
-   Testing and simulation: unit, integration, HIL/SIL, performance,
    security, usability.
-   Manufacturing/EDA: PCB, FPGA, RTL, synthesis, physical design,
    DFM/DFT, compliance.
-   Operations: logging, monitoring, backup, recovery, update/rollback.

For each layer, decide whether it is `REQUIRED`, `OPTIONAL`,
`NOT_APPLICABLE`, or `OPEN`.

### 2.2 Research candidates

For every important layer:

1.  Identify at least two credible candidates when meaningful. Do not
    manufacture alternatives where only one practical option exists.
2.  Prefer primary sources:
    -   official product documentation;
    -   vendor compatibility matrices and release notes;
    -   standards bodies and protocol specifications;
    -   official repositories and package registries;
    -   peer-reviewed papers for research claims;
    -   security advisories and CVE databases.
3.  Use community sources only as supplementary evidence for operational
    experience, ecosystem maturity, or known pain points.
4.  Verify current maintenance status, supported platforms, licensing,
    release activity, compatibility, deployment model, hardware support,
    and known limitations.
5.  Record the date checked. If live research is unavailable, mark
    current status `NOT VERIFIED`.
6.  Never infer that a library is production-ready from a demo, landing
    page, or GitHub star count.
7.  For proprietary services, identify lock-in, pricing uncertainty,
    data residency, rate limits, and exit strategy.

### 2.3 Score candidates

Use a weighted score from 1--5, with a short evidence-based explanation
for each score.

Default weights (adjust to the project and explain changes):

  Criterion                         Default weight
  ------------------------------- ----------------
  Functional fit                               25%
  Reliability and maturity                     15%
  Performance / resource fit                   15%
  Integration and compatibility                15%
  Security and privacy                         10%
  Total cost of ownership                      10%
  Maintainability / team fit                    5%
  Portability / lock-in                         5%

A weighted score is a decision aid, not an automatic decision. Hard
constraints override the score. Reject a candidate that fails a critical
constraint even if its total score is high.

For each candidate, record:

-   technology and category;
-   role in the architecture;
-   status: `RECOMMENDED`, `ALTERNATIVE`, `REJECTED`, or
    `RESEARCH_REQUIRED`;
-   version or version range, if verified;
-   license and commercial restrictions;
-   supported platforms and hardware;
-   key strengths;
-   limitations and failure modes;
-   compatibility/dependency notes;
-   estimated cost class (`LOW`, `MEDIUM`, `HIGH`, `UNKNOWN`) with
    assumptions;
-   weighted score and rationale;
-   source links and verification date;
-   migration/exit considerations.

### 2.4 Classify implementation scope

Every major capability must be categorized:

-   `BUILD FOR REAL` --- differentiating logic, safety-critical control,
    core IP, or capability requiring custom behavior.
-   `MOCK / INTEGRATE` --- commodity capability that should use a mature
    library, API, service, or temporary mock.
-   `CUT ENTIRELY` --- unnecessary, unproven, duplicative, or outside
    the product's validated scope.

Explain each classification. Do not recommend custom infrastructure,
custom databases, custom auth, custom orchestration, or custom model
training without a concrete requirement and build-versus-buy
justification.

## Stage 3 --- Generate `architecture.xlsx`

Create a real `.xlsx` workbook, not a CSV renamed to `.xlsx`. Use the
available spreadsheet tool or a reliable workbook library such as
`openpyxl`. If neither is available, report the limitation and provide a
valid script or a clearly labeled CSV fallback; never claim an Excel
workbook was created when it was not.

The workbook must be usable by a non-expert without losing technical
precision.

### Required worksheets

#### 1. `START HERE`

Include: - project name and brief; - workbook version and generated
date; - status (`DRAFT`, `AWAITING_USER_SELECTION`,
`VALIDATION_REQUIRED`, `APPROVED`); - exact user instructions; - color
legend; - selection workflow; - warning that changing a selection does
not automatically approve the architecture.

#### 2. `Stack Options`

One row per candidate technology. Include columns:

`Layer | Requirement ID | Category | Technology | Role | Status | Version | License | Platform/Hardware | Functional Fit (1-5) | Maturity (1-5) | Performance Fit (1-5) | Integration Fit (1-5) | Security Fit (1-5) | Cost Fit (1-5) | Maintainability (1-5) | Portability (1-5) | Weighted Score | Recommendation | Limitations | Compatibility Notes | Source URL | Verified Date | User Selection`

Use Excel data validation for `User Selection`, with values such as: -
`SELECT` - `ALTERNATIVE` - `REJECT` - `NEEDS DISCUSSION`

Prepopulate the sheet with researched candidates and recommended
defaults, but do not preselect a final stack on the user's behalf. Make
user-editable cells visually distinct from calculated or reference
cells. Use formulas for weighted scores where practical. Freeze headers,
enable filters, set sensible widths, wrap long text, and use conditional
formatting for statuses and scores.

#### 3. `Selected Stack`

Summarize the user's selected technology per layer. Include:
`Layer | Requirement IDs | Selected Technology | Version | Purpose | Required Dependencies | Compatibility Status | User Decision | Validation Status`

Use formulas or a reproducible generation step to derive this view from
`Stack Options`. Flag: - missing required layers; - multiple selections
for mutually exclusive roles; - incompatible runtime or language
versions; - unsupported target platforms; - unverified license or
maintenance status; - unresolved `NEEDS DISCUSSION` choices.

Do not silently choose between conflicting selections.

#### 4. `Decision Matrix`

Show candidate comparison, weighted scores, hard-constraint failures,
recommendation, and decision rationale. Make the weight values visible
and editable only when the user intentionally changes the evaluation
model.

#### 5. `Risks & Dependencies`

Include risk ID, description, probability, impact, severity, mitigation,
owner, dependency, trigger, and status.

#### 6. `Change Log`

Record timestamp, artifact, field/decision changed, previous value, new
value, reason, evidence/source, actor (`USER` or `AGENT`), and approval
state.

#### 7. `Sources`

Record source ID, technology, source title, URL, publisher, source type,
date accessed, claim supported, and confidence.

### Workbook quality requirements

-   Use stable IDs for requirements (`FR-001`, `NFR-001`, `HW-001`,
    `SEC-001`) and technology decisions (`ADR-001`).
-   Use formulas for weighted scores and summaries where practical.
    Avoid fragile formulas that depend on row order.
-   Use dropdown validation for selections and statuses.
-   Protect formula cells if the workbook library supports it, but do
    not add a password that blocks the user.
-   Do not use macros unless explicitly requested.
-   Keep source URLs as clickable hyperlinks.
-   Add an `UNVERIFIED` state wherever research evidence is missing.
-   Save a copy or versioned snapshot before changing a user-edited
    workbook.
-   Reopen the generated workbook programmatically and validate sheet
    names, required columns, formulas, data validations, and non-empty
    candidate rows.
-   If formulas cannot be calculated by the workbook engine, state that
    Excel/LibreOffice may calculate them on open. Do not claim formula
    results were validated unless they were.

## Stage 4 --- Generate `architecture.md`

This is the engineering record and must include:

1.  Project overview and scope.
2.  Engineering brief and requirement IDs.
3.  System context and external actors.
4.  Logical architecture with component responsibilities.
5.  Data flow and control flow.
6.  Deployment topology.
7.  Technology candidates and decision rationale.
8.  Build / Mock-Integrate / Cut classification.
9.  Interface contracts: API, event, data schema, hardware bus,
    electrical interface, or protocol as relevant.
10. Security, privacy, safety, and threat boundaries.
11. Performance/resource budgets and assumptions.
12. Reliability, observability, backup, recovery, and update strategy.
13. Compatibility matrix and dependency constraints.
14. Risks, open questions, and unresolved decisions.
15. Architecture Decision Records (ADRs).
16. Source register with URLs and verification dates.
17. Links to `architecture.xlsx` and downstream artifacts.
18. Architecture status and last verification date.

Use Mermaid diagrams where they materially improve understanding.
Diagrams must represent actual proposed components and flows, not
decorative architecture.

The architecture record must distinguish: - candidate stack; -
user-selected stack; - validated stack; - approved/frozen stack.

## Stage 5 --- Validate the user's Excel selections

This is a mandatory approval gate.

1.  Detect whether the workbook has changed since it was generated.
2.  Read the actual workbook values; do not rely on an old in-memory
    copy or on the agent's previous recommendation.
3.  Compare each user selection with requirements, hard constraints,
    dependency compatibility, supported versions, security, licensing,
    platform support, and total cost.
4.  Validate the stack as a system, not as isolated technologies.
5.  Check integration boundaries: language/runtime compatibility, API
    contracts, drivers, SDK versions, data formats, deployment
    constraints, hardware interfaces, and resource budgets.
6.  Flag each issue with severity:
    -   `BLOCKER` --- stack cannot meet a critical requirement or has a
        hard incompatibility;
    -   `HIGH` --- material reliability, security, performance, or
        operational risk;
    -   `MEDIUM` --- manageable trade-off or additional work;
    -   `LOW` --- preference or minor maintainability concern.
7.  If a selection is invalid, explain the exact conflict and propose
    alternatives. Do not silently replace the user's selection.
8.  If selections are valid but have trade-offs, show them and request
    confirmation when they materially affect cost, privacy, lock-in,
    safety, or performance.
9.  If the workbook is incomplete, ask the user to resolve only the
    blocking rows. Preserve all other selections.
10. Record the validation result in `architecture.md` and `Change Log`.

Only after the user confirms the validated selection may the
architecture status become `APPROVED` or `FROZEN`.

## Stage 6 --- Generate `prd.md`

Generate only after the stack-selection gate is passed, unless the user
explicitly asks for a provisional PRD. Label provisional documents
clearly.

Required PRD structure:

1.  Document status, version, date, and linked architecture revision.
2.  Product vision and problem statement.
3.  Goals and measurable success metrics.
4.  Non-goals and exclusions.
5.  Personas, roles, and permissions.
6.  User journeys / operational workflows.
7.  Functional requirements with stable IDs and priorities (`P0`, `P1`,
    `P2`).
8.  Non-functional requirements with measurable thresholds.
9.  Data requirements, lifecycle, retention, and privacy.
10. API, event, hardware, and external integration requirements.
11. Error states, edge cases, recovery, and offline behavior.
12. Security, safety, regulatory, and accessibility requirements.
13. Analytics and observability requirements.
14. Acceptance criteria in testable Given/When/Then or equivalent form.
15. Release scope and milestone boundaries.
16. Risks, assumptions, and open questions.
17. Finalized technology stack table linked to architecture decisions.
18. Traceability matrix: requirement → component → implementation task →
    test.

Rules: - Every P0 requirement must have a measurable acceptance
criterion. - Avoid vague words such as "fast", "secure", "scalable",
"intelligent", or "user-friendly" without a measurable definition. - Do
not invent performance targets. Propose targets as `PROPOSED` and obtain
user approval. - Do not add features simply because a chosen framework
supports them. - Ensure the PRD does not introduce technologies absent
from the approved architecture. - Any stack change requires architecture
revalidation and a PRD revision.

## Stage 7 --- Generate `agent.md`

Create an implementation-agent contract, not a generic motivational
prompt.

Include:

1.  Mission and project context.
2.  Source-of-truth hierarchy:
    -   explicit user decisions;
    -   approved `architecture.xlsx` selections and `architecture.md`;
    -   approved `prd.md`;
    -   `design.md`;
    -   existing repository conventions;
    -   agent proposals.
3.  Final approved stack with versions and prohibited substitutions.
4.  Repository map and module boundaries.
5.  Agent roles or workstreams, only where useful.
6.  Task execution protocol:
    -   inspect before editing;
    -   plan changes;
    -   implement the smallest complete vertical slice;
    -   test;
    -   review;
    -   update documentation;
    -   report evidence.
7.  Tool and skill discovery protocol.
8.  Coding standards and architecture constraints.
9.  Security, privacy, secrets, and dependency rules.
10. Testing strategy and required commands.
11. Definition of done.
12. Git and change-management rules.
13. Failure handling and escalation.
14. Explicit approval boundaries.

Agent execution rules: - Do not install, upgrade, remove, or replace
major dependencies without approval. - Do not change the approved stack
silently. - Do not fabricate test results, benchmarks, screenshots, or
deployment status. - Do not expose secrets in logs, artifacts, commits,
or reports. - Do not perform destructive operations without explicit
approval. - If requirements conflict, stop at the conflict and report
it. - Keep implementation tasks traceable to PRD requirement IDs. -
Update `architecture.md`, `prd.md`, `agent.md`, or `design.md` when
implementation reality changes; record why and what changed. - Do not
modify user-selected cells in `architecture.xlsx` unless explicitly
authorized.

## Stage 8 --- Generate `design.md`

For projects with a UI, specify the experience before implementation. The mandatory order is: product/context analysis → Mobbin and visual-reference research → Visual Concept Decision Engine and user approval → UI library compatibility gate → design-system specification. Never select UI libraries before approving the visual concept.

Include:

1.  Product experience principles and target users.
2.  Information architecture and navigation.
3.  Core screens, page hierarchy, and user flows.
4.  Design tokens: color, typography, spacing, radius, elevation,
    borders, breakpoints, and motion.
5.  Component inventory and component ownership.
6.  Responsive behavior and accessibility requirements.
7.  Loading, empty, error, offline, permission, and success states.
8.  Interaction behavior and micro-interactions.
9.  Content design, tone, localization, and human-readable error
    messages.
10. Asset and image-generation requirements.
11. Performance budgets and reduced-motion behavior.
12. UI testing and visual-regression criteria.
13. Selected UI libraries, versions, licenses, and source links.
14. Rationale for selecting one coherent design system.

### Design Intelligence workflow (integrated from the supplied `uiux-designer` skill)

Use this workflow for every project with a user interface: landing pages, SaaS products, dashboards, e-commerce, portfolios, web apps, desktop apps, and mobile applications. It applies to both new design specifications and UI review/refinement.

The supplied design skill describes a searchable design database containing 50+ styles, 97 color palettes, 57 font pairings, 99 UX guidelines, and 25 chart types across 12 technology stacks. These counts describe that skill's intended database; they are not evidence that the database files or search script are present in this project. Verify the actual supporting files before relying on them.

#### Priority order

Resolve design decisions in this order. Lower-priority visual polish must not override higher-priority usability requirements.

| Priority | Domain | Impact | Required focus |
|---:|---|---|---|
| 1 | Accessibility | CRITICAL | Contrast, keyboard access, focus, labels, semantics, alt text |
| 2 | Touch and interaction | CRITICAL | Target size, click/tap behavior, loading and error feedback |
| 3 | Performance | HIGH | Image/media optimization, reduced motion, layout stability |
| 4 | Layout and responsive behavior | HIGH | Viewport, mobile readability, overflow, layering |
| 5 | Typography and color | MEDIUM | Readability, hierarchy, consistent font pairing and palette |
| 6 | Animation | MEDIUM | Useful timing, performant properties, loading feedback |
| 7 | Style selection | MEDIUM | Match product and industry; maintain consistency |
| 8 | Charts and data visualization | LOW | Match chart to data and provide accessible alternatives |

#### Mandatory design-system-first procedure

Before writing detailed screen specifications or choosing component libraries:

1. Analyze product type, industry, target users, brand direction, style keywords, platform, and approved technology stack.
2. Generate a complete design-system proposal covering layout pattern, visual style, palette, typography, effects, component principles, and anti-patterns.
3. If the actual `uiux-designer` skill package is installed and its search script exists, use its documented design-system-first command. The supplied skill documents this invocation:

```bash
python3 .agent/skills/uiux-designer/scripts/search.py "<product_type> <industry> <style_keywords>" --design-system [-p "Project Name"]
```

4. Confirm the script path and its required data files exist before execution. Do not install Python or dependencies automatically just to run this optional design search. Do not assume the script exists because this SKILL.md was supplied.
5. If the script or its data files are absent, mark the database search `UNAVAILABLE`, then perform a documented manual design-system synthesis using the rules in this orchestrator. Do not fabricate database search results or claim access to the 50+ styles, 97 palettes, 57 font pairings, 99 UX guidelines, or 25 chart types.
6. Save the design-system proposal and reasoning in `design.md`. Treat it as a proposal until it is checked against the product requirements and approved stack.

The supplied skill describes a `--design-system` search that searches product, style, color, landing-page, and typography domains in parallel, applies reasoning rules, and returns design patterns, colors, typography, effects, and anti-patterns. Use that behavior only if the actual implementation is present and verified.

#### Supplementary domain searches

When the design-system result leaves a material gap, run focused searches only for that gap. The supplied skill documents these domains:

| Domain | Purpose | Example query |
|---|---|---|
| `product` | Product-type design recommendations | `healthcare SaaS`, `e-commerce`, `portfolio` |
| `style` | Visual styles, colors, and effects | `minimalism`, `dark mode`, `brutalism` |
| `typography` | Font pairings and type direction | `professional`, `modern`, `elegant` |
| `color` | Product/industry palette direction | `healthcare`, `fintech`, `SaaS` |
| `landing` | Page structure and conversion patterns | `hero`, `testimonial`, `pricing`, `social-proof` |
| `chart` | Chart and visualization selection | `trend`, `comparison`, `timeline`, `funnel` |
| `ux` | Interaction and usability rules | `animation`, `accessibility`, `z-index`, `loading` |
| `react` | React/Next.js implementation performance | `bundle`, `suspense`, `rerender`, `cache` |
| `web` | Web accessibility and interface semantics | `ARIA`, `focus`, `keyboard`, `semantic`, `virtualize` |
| `prompt` | Design-generation prompts and CSS style keywords | Specific selected style name |

Do not search every domain by default. Search only what is needed, record the query and result source, and incorporate only relevant findings.

#### Stack-specific implementation guidance

The supplied design skill documents these supported stack identifiers. Use the identifier matching the approved architecture; do not default to a different framework simply because the design skill has a default.

| Stack identifier | Main implementation concerns |
|---|---|
| `html-tailwind` | Tailwind utilities, responsive behavior, accessibility |
| `react` | State, hooks, rendering performance, component patterns |
| `nextjs` | SSR, routing, image optimization, API routes |
| `vue` | Composition API, Pinia, Vue Router |
| `svelte` | Runes, stores, SvelteKit |
| `swiftui` | Views, state, navigation, animation |
| `react-native` | Components, navigation, virtualized lists |
| `flutter` | Widgets, state, layout, theming |
| `shadcn` | shadcn/ui components, theming, forms, patterns |
| `jetpack-compose` | Composables, modifiers, state hoisting, recomposition |
| `nuxtjs` | Nuxt 3, Nitro, composables, SEO |
| `nuxt-ui` | Nuxt UI components, theming, forms |

The supplied skill uses `html-tailwind` as its fallback when no stack is specified. This orchestrator must instead wait for the approved architecture stack. If the stack is not yet approved, describe design intent without committing to framework-specific implementation details.

#### UX and visual implementation rules

Apply these rules as baseline checks, adjusting only when platform conventions or documented requirements justify it:

**Accessibility — critical**
- Maintain at least 4.5:1 contrast for normal text. Check large text and non-text contrast against the applicable accessibility standard.
- Provide visible keyboard focus states.
- Give meaningful images descriptive alt text; use empty alt text for decorative images.
- Give icon-only controls accessible names.
- Keep keyboard tab order aligned with visual and reading order.
- Associate form controls with persistent labels; do not rely on placeholder text as the only label.
- Do not use color as the only means of communicating status or meaning.

**Touch and interaction — critical**
- Use a minimum 44 × 44 CSS-pixel touch target as the default design target, unless platform guidance or a justified dense-control case requires a documented alternative.
- Primary interactions must work through click/tap, not hover alone.
- Disable or otherwise guard submit controls during asynchronous operations to prevent accidental duplicate actions.
- Display actionable error feedback near the affected control or content.
- Use pointer cursors for clickable web controls and provide clear hover, active, disabled, and focus states.

**Performance — high**
- Prefer appropriately sized modern image formats, responsive sources (`srcset` where applicable), and lazy loading for off-screen non-critical images.
- Reserve dimensions or aspect ratios for asynchronously loaded media to prevent layout shifts.
- Respect `prefers-reduced-motion`.
- Avoid blocking initial rendering with non-critical scripts, large media, or animation.

**Layout and responsive behavior — high**
- Include the correct viewport configuration for web pages: `width=device-width, initial-scale=1`.
- Target at least 16px body text on mobile unless the content and platform provide a justified exception.
- Prevent horizontal overflow at supported viewport widths.
- Define a documented z-index scale and stacking-context policy; a simple scale such as 10, 20, 30, and 50 may be used where suitable.
- Reserve space for fixed or sticky navigation and ensure content is not obscured.
- Use consistent content container widths and spacing tokens.

**Typography and color — medium**
- Use a body line-height around 1.5–1.75 as a starting point, then validate for the selected typeface and content.
- Aim for readable line lengths around 65–75 characters for long-form desktop text; adapt for mobile.
- Pair heading and body typefaces intentionally. Limit font families and weights to what the design needs.
- Define semantic color tokens for text, surfaces, borders, actions, focus, and status. Validate both light and dark themes if both are supported.

**Animation — medium**
- Use approximately 150–300ms for ordinary micro-interactions as a starting range; use shorter or longer durations only with a reason.
- Prefer `transform` and `opacity` for animation rather than layout-triggering properties such as `width` and `height`.
- Define skeleton, progress, or spinner behavior for meaningful loading states.
- Avoid hover transforms that cause layout shifts.
- Never use motion as a substitute for state communication.

**Style consistency — medium**
- Match the visual style to product type, industry, audience, and brand.
- Use one coherent visual system across the product.
- Use SVG/icon-font assets from a consistent icon set rather than emoji glyphs as interface icons.
- Verify official brand marks and logos from authoritative sources; never guess logo paths or substitute a visually similar mark.

**Charts and data visualization — low, but mandatory when charts are present**
- Select chart types based on the analytical question and data structure.
- Use accessible color palettes and do not rely on color alone to distinguish series.
- Provide a table or equivalent textual representation for important chart data.
- Include units, labels, legends, and meaningful empty/error states.

#### Design-system output contract for `design.md`

Record:
- Product type, industry, target users, design keywords, and approved UI stack.
- Search method: design-skill script, other verified tool, or manual synthesis.
- Search queries, domains, and actual sources consulted.
- Chosen layout pattern and rationale.
- Selected style and rationale.
- Color palette with token names and exact values, plus contrast validation status.
- Typography families, weights, scale, and line-height.
- Spacing, container widths, radii, elevation, borders, and z-index tokens.
- Motion principles, durations, easing, reduced-motion behavior, and fallback behavior.
- Component and icon conventions.
- Chart choices and accessible alternatives, if applicable.
- Anti-patterns explicitly prohibited for this project.
- Stack-specific implementation constraints.
- Outstanding decisions and approval status.

#### UI pre-delivery checklist

Before approving any UI implementation, verify:

**Visual quality**
- [ ] No emoji used as interface icons.
- [ ] Icons come from a consistent, verified icon set.
- [ ] Brand logos are correct and sourced from an authoritative location.
- [ ] Hover and focus states do not cause layout shifts.
- [ ] Semantic theme tokens are used consistently.

**Interaction**
- [ ] All interactive elements have an obvious pointer/tap affordance.
- [ ] Hover, active, disabled, loading, success, and error states are defined where relevant.
- [ ] Transitions are purposeful and generally within the 150–300ms micro-interaction range.
- [ ] Keyboard focus is visible and usable.

**Themes**
- [ ] Text contrast meets the applicable contrast target in each supported theme.
- [ ] Transparent/glass surfaces remain legible in light and dark contexts.
- [ ] Borders and focus indicators remain visible in each supported theme.
- [ ] Every supported theme has been tested.

**Layout**
- [ ] Fixed/floating elements do not obscure content.
- [ ] Responsive behavior is checked at 375px, 768px, 1024px, and 1440px, plus actual target-device widths.
- [ ] No unintended horizontal scrolling occurs.
- [ ] Text, controls, and media adapt without clipping or overlap.

**Accessibility**
- [ ] Meaningful images have alt text.
- [ ] Form controls have associated labels.
- [ ] Color is not the only status indicator.
- [ ] Keyboard navigation and focus order are correct.
- [ ] Reduced-motion preferences are respected.

Do not mark a checklist item as passed without inspecting the rendered UI, source implementation, or an appropriate test result. If no implementation exists yet, mark the item `PENDING IMPLEMENTATION`, not passed.

### Visual Concept Decision Engine — mandatory before UI library selection

The visual concept decision is a required architecture decision. Complete it before selecting, installing, or recommending UI component libraries. A component library must support the chosen visual direction; the visual direction must not be dictated accidentally by whichever library is easiest to install.

#### Available visual concepts

Treat these as a design vocabulary and candidate set. Do not assume every concept suits every product. The agent may recommend a hybrid only when the concepts are visually compatible and the combination has a clear rationale.

| Concept | Characteristics to evaluate | Suitable use cases / cautions |
|---|---|---|
| Claymorphism | Soft, inflated forms, rounded geometry, layered shadows, tactile surfaces | Friendly consumer products; can reduce density and contrast if overused |
| Cybercore | Futuristic digital systems, technical motifs, high-contrast interfaces, experimental layouts | Creative technology and developer products; avoid decorative complexity that harms task completion |
| Neo-Brutalism | Hard borders, bold blocks, strong typography, visible structure, assertive color | Distinctive brands and editorial/product experiences; validate hierarchy and accessibility |
| Scrapbook | Collage, layered paper, annotations, hand-assembled visual language | Creative, educational and community products; avoid visual clutter in operational workflows |
| Surrealism | Unexpected compositions, dreamlike imagery, visual metaphor, unconventional transitions | Campaigns, art-led products and immersive storytelling; preserve predictable navigation |
| Y2K Aesthetic | Early-digital references, chrome-like finishes, playful geometry, nostalgic interface motifs | Youth/culture and entertainment brands; avoid sacrificing legibility |
| Pixel Art | Pixel-grid visuals, sprite-inspired assets, bitmap typography and deliberate low-resolution motifs | Games, retro tools and playful products; ensure text and controls remain readable |
| Synthwave | Neon palettes, retro-futuristic gradients, grid motifs, atmospheric lighting | Music, gaming and creative experiences; control contrast and visual fatigue |
| Glassmorphism | Translucent surfaces, background blur, layered depth | Selective overlays and visual hierarchy; test contrast over every background and provide opaque fallbacks |
| Neumorphism | Low-contrast raised/inset surfaces and soft shadows | Limited decorative or tactile controls; do not use as the sole affordance because boundaries can disappear |
| Bento Grid | Modular asymmetric or uniform cards, grouped information, clear content zones | Dashboards, product overviews and landing pages; avoid turning every page into an undifferentiated card grid |
| Editorial Design | Strong typographic hierarchy, deliberate pacing, structured imagery and narrative composition | Publishing, portfolios, research and brand-led products; retain efficient task paths |
| Swiss Design | Grid discipline, precise alignment, typography-led hierarchy, restrained color | Enterprise, technical, information-dense and professional products |
| Minimalism | Reduced visual noise, essential content, restrained palette and interaction | Productivity and focused workflows; do not hide discoverability or necessary information |
| Maximalism | Dense expressive compositions, layered visuals, rich color and strong personality | Creative, entertainment and culture-led products; maintain navigational clarity |
| Luxury Typography | Expressive type scale, refined spacing, restrained composition and premium editorial cues | Premium brands and high-consideration products; preserve legibility and localization support |
| Conceptual Sketch | Hand-drawn marks, diagrams, annotations and exploratory visual language | Education, ideation and technical explanation; do not make production controls look unfinished |
| Ethereal | Light, atmospheric imagery, soft transitions, spacious composition | Wellness, art and experiential storytelling; ensure sufficient contrast and avoid vague calls to action |
| Bohemian | Organic shapes, earthy palettes, expressive typography and crafted imagery | Lifestyle, creative and community brands; maintain consistency and usability |
| Victorian | Ornamental detail, classical typography, intricate borders and historical motifs | Heritage, culture and specialty brands; avoid excessive ornament in task-heavy applications |
| Cyberpunk | Dystopian futurism, neon accents, dark surfaces, technical visual motifs | Gaming, entertainment and selected technology brands; distinguish it from Cybercore and Synthwave rather than treating them as synonyms |
| Wabi-Sabi | Imperfect natural textures, asymmetry, muted tones and restrained composition | Craft, culture, lifestyle and reflective experiences; ensure the interface still feels intentional and consistent |

#### Mandatory concept-selection procedure

Before library selection, the agent must:

1. Derive the product category, user context, primary tasks, brand positioning, content density, device targets, accessibility needs and performance constraints from the engineering brief and PRD inputs.
2. Select 3–5 plausible concepts from the catalog for evaluation. Do not present all 22 as equally suitable choices by default.
3. Evaluate each candidate against task clarity and usability; audience and brand fit; content density and information hierarchy; accessibility and contrast; responsive behavior; performance and asset cost; implementation complexity; and consistency with the product's existing identity.
4. Recommend one primary concept. A secondary accent concept is allowed only when its role and boundaries are explicit. Avoid combining incompatible aesthetics merely to make the design appear unique.
5. Record the decision as a Design Direction Decision Record (`DDR-001`, `DDR-002`, etc.) with:
   - selected concept and optional secondary concept;
   - product and user rationale;
   - evidence from Mobbin and inspected visual references, where available;
   - typography, color, shape, spacing, imagery, layout and motion implications;
   - accessibility and performance risks;
   - concepts considered and rejected, with reasons;
   - approval state and date.
6. Show the recommended direction to the user and obtain approval before locking it. If the user has explicitly chosen a concept, respect that choice but still perform feasibility and accessibility checks.
7. If no catalog concept fits, propose a clearly named custom direction and explain why the existing concepts are insufficient. Do not force-fit the product.

#### Concept decision output

Add a `Visual Concept Decision` section to `design.md` before the UI library inventory. Include product context and design objective; candidate concepts evaluated and concise scores/rationale; selected primary concept and any bounded secondary concept; design principles translated into concrete interface rules; typography, color, geometry, spacing, imagery, grid and motion direction; accessibility, responsiveness and performance guardrails; rejected concepts and reasons; and DDR identifier, approval status and date.

The concept is a design direction, not a requirement to reproduce a visual trend literally. Translate it into a coherent system of tokens, components and interaction rules.

#### Library compatibility gate

Only after the visual concept is approved may the agent evaluate UI libraries. For each candidate library, assess compatibility with the selected concept and its visual primitives; customization of tokens, typography, radius, borders, shadows, density and layout; framework and styling compatibility with the approved technology stack; accessibility and keyboard behavior; responsive behavior and component quality; animation support where needed; dependency weight, performance and maintenance; licensing and commercial-use constraints; and whether the library encourages a generic visual result that conflicts with the approved direction.

A library is not automatically suitable because its demo looks attractive. If no library supports the chosen concept without excessive overrides, use a smaller compatible primitive library or build the differentiating visual components. Record the decision and trade-offs.

Do not install multiple overlapping libraries to imitate a concept. Select one primary component system and only add complementary libraries for specific, justified gaps.

#### Re-evaluation triggers

Reopen the concept decision if the target audience, product category, brand positioning, core workflows, device constraints or approved design requirements materially change. Record the revision in the design decision history. Do not silently replace the approved direction during implementation.

### UI library selection policy

The user's candidate catalog may include Animaster, Skiper UI, Vengeance
UI, Magic UI, Eldora UI, Flowbite, Watermelon UI, React Bits, Float UI,
Aceternity UI, Preline UI, Kibo UI, Meraki UI, Animate UI, HyperUI,
Motion Primitives, DaisyUI, Hero UI, Lightswind, 21st.dev, Jolly UI,
Glassy UI, Skyper UI, ReUI, Flyon UI, Coss UI, shadcn/ui, Beautiful UI,
shadcn Studio, Smooth UI, Unlumen UI, Unlumen Components, ChanhDai
Components, Flowkit-UI, Satis UI, Retro UI, 8bitcn, Fancy Components,
TailGrids, and other relevant libraries.

Treat this as a discovery catalog, not a requirement to use all
libraries. Verify exact project identity, current maintenance, framework
compatibility, license, accessibility, bundle impact, and component
quality before recommending any option. Names may be misspelled,
duplicated, renamed, or refer to unrelated projects; resolve identity
before selection. If a library is already listed elsewhere in this skill or
project catalog, do not add a duplicate entry; merge its metadata into the
existing record.

Selection rules: - Choose one primary component system and, at most, a
small number of complementary libraries when there is a clear gap. -
Avoid combining competing component systems with overlapping primitives,
token models, or styling conventions. - Prefer a consistent base such as
the existing project system or shadcn/ui when appropriate; do not force
it. - Use animation libraries only where motion improves comprehension
or feedback. Respect `prefers-reduced-motion`. - Do not use a library
solely because it has visually impressive demos. - Check licensing and
commercial usage before recommending components for a commercial
product. - Specify whether components are copied into the repository,
installed as dependencies, or consumed remotely. - Record exact
components intended for use and any required adaptation. - If a library
is unavailable or unverified, mark it `NOT VERIFIED`; do not claim it
was installed or tested.

### Visual Design Inspiration Registry

The following user-provided websites are approved visual research references. They are not UI component libraries, dependencies, or templates. The agent must inspect the live reference when designing a project, extract applicable principles, and adapt them rather than reproduce the site.

Reference registry:

| Reference | Design discipline to inspect | Potentially transferable principles | Do not assume |
|---|---|---|---|
| https://pacomepertant.com/ | Experimental motion and sound-led portfolio experience | Intentional motion, distinctive entry states, immersive sequencing, sound as an optional interaction layer, strong creative identity | Sound is appropriate for a business application; motion should delay access to core tasks |
| https://aikawakenichi.com/ | Photography-led editorial portfolio | Image-first hierarchy, restrained navigation, whitespace, editorial pacing, minimal chrome, visual storytelling | Minimalism means weak accessibility, hidden navigation, or insufficient product information |
| https://www.podium-studios.com/services | Cinematic creative-agency and service presentation | Art direction, strong media composition, clear service grouping, narrative section progression, CTA placement, brand consistency | Large media assets or cinematic effects are appropriate for every product |
| https://hirotosato0127.github.io/ | Researcher / academic information presentation | Clear information hierarchy, readable typography, scannable research content, publication metadata, restrained navigation, direct access to documents | This is a high-motion or visually experimental reference; use it primarily for information architecture and content clarity |
| https://vshslv.com/ | Creative developer portfolio and design-engineering showcase | Integrated visual identity and code, responsive interaction, WebGL / Three.js / GSAP possibilities, custom visual systems, motion craft, end-to-end design engineering | WebGL, 3D, or intensive animation should be added without a product-specific reason |

#### Required reference-inspection procedure

For each project, the design agent must:

1. Visit each reference URL when browsing is available. If unavailable, record `NOT INSPECTED`; do not claim to have reviewed it.
2. Inspect the actual page and, where practical, its responsive behavior, navigation, interaction states, motion, typography, composition, content density, and media treatment.
3. Record observations separately from interpretation. Example:
   - `OBSERVED`: the page uses a large photographic image as a primary visual anchor.
   - `INTERPRETATION`: a photography-led landing page may suit a visual portfolio product.
   - `ADAPTATION`: use a project-specific hero image with accessible text and a responsive crop.
4. Select only the principles relevant to the project's users, product category, brand, content, and tasks.
5. Record the chosen references and adaptations in `design.md`, including the URL, inspection date, observed principle, intended adaptation, and reason.
6. Record references that were considered but rejected, with a short reason, when they materially influenced the design decision.
7. Never copy source code, proprietary assets, logos, exact compositions, or distinctive visual expression. Create an original implementation informed by high-level design principles.
8. Do not infer a reference site's underlying framework or library from appearance alone. Verify its technology separately before recommending it as a dependency.
9. Do not treat a reference as proof of accessibility, performance, or responsive quality. Test those properties in the target implementation.

#### Design-direction synthesis

Before choosing UI libraries or writing the screen specification, produce a concise design-direction synthesis:

- Product category and user context.
- Primary visual objective: clarity, trust, conversion, exploration, productivity, storytelling, or another justified goal.
- Chosen reference principles (normally 1–3 references, not all five).
- Principles deliberately excluded and why.
- Proposed visual language: layout, typography, color, imagery, density, navigation, and motion.
- Interaction intensity: `STATIC`, `SUBTLE`, `INTERACTIVE`, or `IMMERSIVE`.
- Performance tier: `LOW`, `MODERATE`, or `HIGH`, justified against target devices and network conditions.
- Accessibility and reduced-motion behavior.
- Validation method: responsive checks, keyboard navigation, contrast, loading performance, and usability tests.

Do not blend all five references into one design. Select a coherent direction. A research tool may benefit from Hiroto Sato's content hierarchy; a visual portfolio may benefit from Kenichi Aikawa's editorial image treatment; a brand-led landing page may borrow Podium's narrative sequencing; an experimental creative product may use Pacôme Pertant's intentional motion; and a high-end interactive experience may selectively use the design-engineering techniques demonstrated by Viacheslav Novoseltsev.

#### Motion and immersive-design guardrails

- Motion must communicate state, hierarchy, continuity, or meaningful feedback.
- Never block primary content behind an unnecessary intro animation.
- Sound must be opt-in, have a clear mute/stop control, and never autoplay unexpectedly.
- Provide reduced-motion alternatives and honor `prefers-reduced-motion`.
- Provide static or simplified fallbacks for WebGL, canvas, video, and 3D scenes.
- Lazy-load non-critical media and split heavy rendering code.
- Define a performance budget before adopting Three.js, WebGL, GSAP-heavy sequences, or large video backgrounds.
- Test on the actual minimum target device class, not only a high-end desktop.
- Ensure navigation, forms, text, and primary actions remain usable if animation or media fails.

### Design skills and specialist tools

Discover available skills and integrations before invoking them. Use
only relevant, actually installed skills or tools. The user's requested
specialist skill families include:

-   `impeccable` --- frontend design critique and refinement;
-   `humaniser` / humanizer --- natural, non-generic product copy;
-   `taste` --- landing-page visual direction and quality;
-   `banana` / Claude image-generation skill --- image asset generation,
    if actually available;
-   `apple` design skill --- platform-appropriate UX patterns, if
    actually available.

These names are discovery hints, not proof of installation or official
product names. Search the available skill registry and inspect each
skill's own instructions before use. Do not invent invocation syntax. If
absent, continue using the design criteria in this specification and
report that the specialist skill was unavailable. Use image-generation
tools only for assets that the design actually requires; do not generate
decorative images by default.

Design quality gates: - No generic dashboard template by default. - No
arbitrary gradients, glassmorphism, oversized hero sections, or
decorative animation without a product rationale. - Establish hierarchy,
contrast, spacing, alignment, responsive behavior, keyboard access, and
empty/error states. - Use real product language rather than filler
copy. - Ensure every interactive control has a defined behavior. -
Include a screen inventory and acceptance checklist.

## Stage 9 --- Continuous verification and architecture drift control

When the user asks to verify, update, or continue a project---or when a
relevant change is detected during implementation---compare the actual
repository against the approved artifacts.

Inspect: - dependency manifests and lockfiles; - source imports and
framework usage; - API routes and schemas; - database schema and
migrations; - environment/configuration keys (never print secret
values); - infrastructure and deployment files; - hardware pin maps,
interfaces, BOM, and firmware configuration where relevant; - UI
components, design tokens, and installed UI libraries; - tests, CI, and
runtime evidence.

Classify drift:

-   `INTENTIONAL_APPROVED`
-   `INTENTIONAL_UNAPPROVED`
-   `ACCIDENTAL`
-   `STALE_DOCUMENTATION`
-   `UNVERIFIED`

For every finding, record: - artifact and requirement/ADR affected; -
expected state; - observed state; - evidence (file path, line, command
output, or source); - impact and severity; - recommended correction; -
whether user approval is required.

Update policy: 1. Never overwrite the user's stack selections. 2. Never
silently change an approved architecture. 3. Documentation-only
corrections that preserve approved intent may be made, then reported. 4.
Changes affecting cost, security, privacy, performance, licensing,
hardware, deployment, or product scope require user approval before
implementation. 5. If the repository has intentionally moved beyond the
old architecture, propose a new ADR and revised artifacts. 6. Preserve
prior versions or use Git history before making material changes. 7.
Update all dependent documents consistently. A stack change must trigger
review of architecture, PRD, agent instructions, design, tests, and
deployment configuration as applicable. 8. Do not mark an item verified
without evidence.

## Stage 10 --- Change notification

After every verification or update, provide a concise but complete
change report in the user-facing response.

Use this format:

### Architecture update report

-   **Status:** `NO CHANGE` / `UPDATED` / `BLOCKED — APPROVAL REQUIRED`
-   **Files inspected:** list
-   **Files changed:** list
-   **Changes made:** old value → new value, with reason
-   **Evidence:** tests, commands, source URLs, or repository paths
-   **Compatibility impact:** affected modules and dependencies
-   **User action required:** exact decision, if any
-   **Remaining risks:** unresolved items

If no changes were required, say so explicitly. Do not claim files were
updated when only recommendations were produced. If an update failed,
report the failure and leave the affected artifact in a safe state.

## Stage 11 --- Final consistency audit

Before declaring the specification package complete, verify:

-   [ ] All required artifacts exist at the agreed paths.
-   [ ] Workbook opens and contains the required worksheets and editable
    selections.
-   [ ] User choices are preserved.
-   [ ] `architecture.md` accurately reflects workbook decisions and
    validation state.
-   [ ] PRD uses only the approved stack.
-   [ ] Every P0 requirement has a measurable acceptance criterion.
-   [ ] Agent instructions agree with architecture and PRD.
-   [ ] Design specification agrees with product scope and approved UI
    stack.
-   [ ] Requirement IDs and ADR references are consistent.
-   [ ] Sources and verification dates are present for material
    technology claims.
-   [ ] Risks, assumptions, and unresolved decisions are visible.
-   [ ] No unverified capability is described as implemented.
-   [ ] Change log and user-facing report accurately describe all
    mutations.

Only report `COMPLETE` when every applicable check passes. Otherwise
report `PARTIAL` and list blockers.

## Recommended project directory

Use the repository's existing convention when present. Otherwise:
``` text
project-root/
├── .agents/
│   └── skills/
│       └── project-architecture-orchestrator/
│           └── SKILL.md
├── docs/
│   └── project-spec/
│       ├── architecture.xlsx
│       ├── architecture.md
│       ├── prd.md
│       ├── agent.md
│       ├── design.md
│       └── history/
└── ...
```

## First response behavior

When the user gives a new project idea, do not dump a generic technology
list into chat. First inspect the repository and clarify only critical
unknowns. Then create the engineering brief, research candidate
technologies, and generate the draft workbook plus architecture record.
Tell the user exactly where the files are and how to select the stack.

Wait for the user to complete and return/save the workbook selections.
Then validate those selections and obtain explicit approval before
generating finalized downstream documents.

## Definition of success

The skill succeeds when a developer can start implementation without
guessing the product scope, approved stack, system boundaries,
interfaces, acceptance criteria, agent constraints, or UI
direction---and when future architecture drift is detected, evidenced,
explained, and reported without silently overriding user decisions.