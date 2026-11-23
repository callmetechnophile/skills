/**
 * Legacy registry ranking interface wrapper backed by HybridRanker / LexicalMatcher.
 */
import { LexicalMatcher } from "./ranking/lexical-matcher.mjs";
import { HybridRanker } from "./ranking/hybrid-ranker.mjs";

const lexicalMatcher = new LexicalMatcher();
const hybridRanker = new HybridRanker({ lexicalMatcher });

export function rankSkills(skills, query, options = {}) {
  if (!Array.isArray(skills)) throw new TypeError("skills must be an array");
  const normalized = String(query || "").trim();
  if (!normalized) return [];

  const bm25Results = lexicalMatcher.computeBM25(skills, normalized);
  const maxScore = Math.max(...bm25Results.map(r => r.lexicalScore), 1.0);

  const ranked = bm25Results
    .filter(r => r.lexicalScore > 0)
    .map(r => {
      const s = r.skill;
      const normalizedLexical = Math.min(1.0, r.lexicalScore / maxScore);
      // Small logarithmic star signal (capped at 0.05)
      const starBonus = Math.min(0.05, Math.log10(Math.max(1, Number(s.stars || 0))) / 100);
      const score = Number((normalizedLexical * 10 + starBonus).toFixed(3));
      return {
        ...s,
        relevanceScore: score
      };
    })
    .sort((a, b) => b.relevanceScore - a.relevanceScore || Number(b.stars || 0) - Number(a.stars || 0));

  return ranked;
}

export { HybridRanker };
