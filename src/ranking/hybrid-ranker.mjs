/**
 * Hybrid Skill Ranking Engine combining Lexical BM25, Semantic vector scores,
 * and 5-factor normalized reranking with explainable breakdowns.
 */

import { LexicalMatcher } from "./lexical-matcher.mjs";
import { EmbeddingMatcher } from "./embedding-matcher.mjs";

export const DEFAULT_WEIGHTS = {
  relevance: 0.40,
  completeness: 0.20,
  provenance: 0.15,
  security: 0.15,
  compatibility: 0.10
};

export class HybridRanker {
  constructor(options = {}) {
    this.weights = { ...DEFAULT_WEIGHTS, ...(options.weights || {}) };
    this.lexicalMatcher = options.lexicalMatcher || new LexicalMatcher();
    this.embeddingMatcher = options.embeddingMatcher || new EmbeddingMatcher();
    this.targetAgent = options.targetAgent || "antigravity";
  }

  _scoreCompleteness(skill) {
    let score = 0;
    if (skill.name && skill.name.length > 2) score += 0.25;
    if (skill.description && skill.description.length > 15) score += 0.35;
    if (Array.isArray(skill.tags) && skill.tags.length > 0) score += 0.20;
    if (skill.path || skill.skill_directory || skill.repository) score += 0.20;
    return Math.min(1.0, score);
  }

  _scoreProvenance(skill) {
    let score = 0.2; // Base baseline
    if (skill.verified) score += 0.4;
    if (skill.source === "local" || skill.registry === "local") score += 0.4;
    else if (skill.repository && skill.repository.includes("/")) score += 0.2;

    // Logarithmic stars contribution, strictly capped at 0.2 so stars never dominate
    const stars = Number(skill.stars || 0);
    const starBonus = Math.min(0.2, Math.log10(Math.max(1, stars)) / 20);
    score += starBonus;

    return Math.min(1.0, score);
  }

  _scoreSecurity(skill) {
    const status = String(skill.security_status || skill.trustStatus || "UNSCANNED").toUpperCase();
    if (status.includes("REJECTED") || status.includes("QUARANTINED")) return 0.0;
    if (status.includes("RESTRICTED")) return 0.3;
    if (status.includes("UNSCANNED")) return 0.5;
    if (status.includes("APPROVED") || status.includes("LOCAL")) return 1.0;
    return 0.5;
  }

  _scoreCompatibility(skill, targetAgent) {
    // If skill explicitly targets or is compatible with target agent
    if (!skill.supported_agents || !Array.isArray(skill.supported_agents)) {
      return 0.9; // General default compatibility
    }
    const agent = (targetAgent || this.targetAgent).toLowerCase();
    if (skill.supported_agents.includes("*") || skill.supported_agents.map(a => a.toLowerCase()).includes(agent)) {
      return 1.0;
    }
    return 0.1;
  }

  async rank(skills, query, options = {}) {
    if (!Array.isArray(skills) || skills.length === 0) {
      return {
        selected: null,
        alternatives: [],
        ranked: [],
        rejections: []
      };
    }

    const normQuery = String(query || "").trim();
    if (!normQuery) {
      return {
        selected: null,
        alternatives: [],
        ranked: [],
        rejections: []
      };
    }

    const targetAgent = options.targetAgent || this.targetAgent;

    // 1. BM25 Lexical scores
    const lexicalResults = this.lexicalMatcher.computeBM25(skills, normQuery);
    const maxLexical = Math.max(...lexicalResults.map(r => r.lexicalScore), 1.0);

    // 2. Semantic scores
    const semanticResults = await this.embeddingMatcher.computeSimilarity(skills, normQuery);
    const semanticMap = new Map(semanticResults.map(r => [r.skill, r.semanticScore]));

    const scored = [];
    const rejections = [];

    for (const item of lexicalResults) {
      const skill = item.skill;
      const normalizedLexical = Math.min(1.0, item.lexicalScore / maxLexical);
      const semanticScore = semanticMap.get(skill) ?? 0;

      // Combined relevance (60% lexical / 40% semantic within relevance component)
      const relevanceScore = (normalizedLexical * 0.6) + (semanticScore * 0.4);

      const completenessScore = this._scoreCompleteness(skill);
      const provenanceScore = this._scoreProvenance(skill);
      const securityScore = this._scoreSecurity(skill);
      const compatibilityScore = this._scoreCompatibility(skill, targetAgent);

      // Check hard security rejection
      if (securityScore === 0.0) {
        rejections.push({
          skill,
          reason: "SECURITY_REJECTION",
          details: `Skill has untrusted security status: ${skill.security_status || skill.trustStatus}`
        });
        continue;
      }

      // Check hard compatibility rejection
      if (compatibilityScore < 0.2 && options.strictCompatibility) {
        rejections.push({
          skill,
          reason: "INCOMPATIBLE_RUNTIME",
          details: `Skill does not support target agent '${targetAgent}'`
        });
        continue;
      }

      // Compute weighted total
      const totalScore = (
        (relevanceScore * this.weights.relevance) +
        (completenessScore * this.weights.completeness) +
        (provenanceScore * this.weights.provenance) +
        (securityScore * this.weights.security) +
        (compatibilityScore * this.weights.compatibility)
      );

      // Filter out zero-relevance matches unless specifically matching all
      if (relevanceScore <= 0.01 && !normQuery.includes("*")) {
        continue;
      }

      scored.push({
        skill,
        relevanceScore: Number(totalScore.toFixed(3)),
        scoreBreakdown: {
          task_relevance: Number((relevanceScore * this.weights.relevance).toFixed(3)),
          capability_completeness: Number((completenessScore * this.weights.completeness).toFixed(3)),
          provenance_maintenance: Number((provenanceScore * this.weights.provenance).toFixed(3)),
          security_trust: Number((securityScore * this.weights.security).toFixed(3)),
          runtime_compatibility: Number((compatibilityScore * this.weights.compatibility).toFixed(3)),
          raw_metrics: {
            lexical: Number(item.lexicalScore.toFixed(2)),
            semantic: Number(semanticScore.toFixed(2)),
            security_score: securityScore,
            compatibility_score: compatibilityScore
          }
        },
        securityResult: securityScore >= 0.8 ? "APPROVED" : securityScore >= 0.3 ? "RESTRICTED" : "REJECTED",
        compatibilityResult: compatibilityScore >= 0.5 ? "COMPATIBLE" : "INCOMPATIBLE"
      });
    }

    scored.sort((a, b) => b.relevanceScore - a.relevanceScore);

    const selected = scored.length > 0 ? scored[0] : null;
    const alternatives = scored.slice(1, options.maxAlternatives || 5);

    return {
      selected,
      alternatives,
      ranked: scored,
      rejections
    };
  }
}
