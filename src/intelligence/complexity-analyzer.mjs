/**
 * Multi-dimensional task complexity analyzer with layered deterministic heuristics
 * and pluggable LLM provider support.
 */

export class ComplexityAnalyzer {
  constructor(options = {}) {
    this.llmProvider = options.llmProvider || null;
    this.uncertaintyThreshold = options.uncertaintyThreshold ?? 0.6;
  }

  /**
   * Deterministic heuristic scoring based on task text and structural tokens.
   */
  scoreHeuristics(task, domains = []) {
    const text = String(task || "").toLowerCase();

    // 1. Scope Breadth (0: local/single file, 1: bounded module, 2: broad/cross-system/end-to-end)
    let scope_breadth = 0;
    const nonTestingDomains = domains.filter(d => d !== "testing");
    if (/(entire|whole|all services|across repo|end-to-end|platform|system-wide|full-stack)/i.test(text) || nonTestingDomains.length >= 3) {
      scope_breadth = 2;
    } else if (/(component|module|service|package|multiple files|workflow)/i.test(text) || nonTestingDomains.length >= 2) {
      scope_breadth = 1;
    }

    // 2. Technical Coupling (0: isolated, 1: single interface/API, 2: multi-tier database/auth/api/ui)
    let technical_coupling = 0;
    const tierMatches = [
      /(database|sql|postgres|mongo|migration|schema)/i.test(text),
      /(frontend|react|ui|css|client)/i.test(text),
      /(backend|api|server|microservice)/i.test(text),
      /(auth|oauth|jwt|session)/i.test(text),
      /(devops|docker|kubernetes|ci\/cd|pipeline)/i.test(text)
    ].filter(Boolean).length;

    if (tierMatches >= 3 || /(integration|coupled|cross-cutting|full-stack)/i.test(text)) {
      technical_coupling = 2;
    } else if (tierMatches >= 2) {
      technical_coupling = 1;
    }

    // 3. Uncertainty (0: clear/bugfix, 1: moderate investigation, 2: open-ended research/architecture)
    let uncertainty = 0;
    if (/(explore|research|unclear|unknown|spike|evaluate architectural|feasibility|ambiguous)/i.test(text)) {
      uncertainty = 2;
    } else if (/(investigate|diagnose|audit|debug intermittent|analyze cause)/i.test(text)) {
      uncertainty = 1;
    }

    // 4. Risk / Blast Radius (0: low/test/docs, 1: business logic, 2: security/payment/production/data deletion)
    let risk = 0;
    const isTestTask = /(test|tests|unit test|spec)/i.test(text);
    if (!isTestTask && /(security|auth|credential|secret|payment|billing|production deploy|drop table|delete data|cve|vulnerability)/i.test(text)) {
      risk = 2;
    } else if (/(migration|refactor|api change|database|state management|auth|security)/i.test(text)) {
      risk = 1;
    }

    // 5. Parallelism (0: strictly sequential, 1: separable, 2: independent modules/multi-agent)
    let parallelism = 0;
    if (tierMatches >= 3 || /(independent|parallel|multi-agent|sub-agent|concurrently|separate workstreams)/i.test(text)) {
      parallelism = 2;
    } else if (tierMatches === 2 || /(and|as well as|both)/i.test(text)) {
      parallelism = 1;
    }

    const total_score = scope_breadth + technical_coupling + uncertainty + risk + parallelism;
    const band = total_score <= 3 ? "LOW" : total_score <= 6 ? "MEDIUM" : "HIGH";

    return {
      scope_breadth,
      technical_coupling,
      uncertainty,
      risk,
      parallelism,
      total_score,
      band,
      method: "deterministic-heuristic"
    };
  }

  /**
   * Analyze complexity using configured provider with heuristic fallback.
   */
  async analyze(task, domains = []) {
    if (this.llmProvider) {
      try {
        const llmResult = await this.llmProvider.analyzeComplexity(task, domains);
        if (this._validateComplexityOutput(llmResult)) {
          return { ...llmResult, method: "llm-structured" };
        }
      } catch (err) {
        // Fall back gracefully on provider errors or timeouts
      }
    }
    return this.scoreHeuristics(task, domains);
  }

  _validateComplexityOutput(obj) {
    if (!obj || typeof obj !== "object") return false;
    const fields = ["scope_breadth", "technical_coupling", "uncertainty", "risk", "parallelism", "total_score", "band"];
    for (const f of fields) {
      if (obj[f] === undefined) return false;
    }
    return ["LOW", "MEDIUM", "HIGH"].includes(obj.band);
  }
}
