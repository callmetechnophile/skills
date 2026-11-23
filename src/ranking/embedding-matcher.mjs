/**
 * Pluggable embedding matcher with deterministic local n-gram vector fallback.
 */

export class EmbeddingMatcher {
  constructor(options = {}) {
    this.embedFn = options.embedFn || null;
  }

  /**
   * Deterministic local n-gram vectorizer for zero-cost offline semantic approximation.
   */
  _toNGramVector(text, n = 3) {
    const cleaned = String(text || "").toLowerCase().replace(/[^a-z0-9]/g, "");
    const vec = new Map();
    for (let i = 0; i <= cleaned.length - n; i++) {
      const gram = cleaned.slice(i, i + n);
      vec.set(gram, (vec.get(gram) || 0) + 1);
    }
    return vec;
  }

  _cosineSimilarity(vecA, vecB) {
    let dot = 0;
    let magA = 0;
    let magB = 0;

    for (const [k, v] of vecA) {
      magA += v * v;
      if (vecB.has(k)) {
        dot += v * vecB.get(k);
      }
    }
    for (const [, v] of vecB) {
      magB += v * v;
    }

    if (magA === 0 || magB === 0) return 0;
    return dot / (Math.sqrt(magA) * Math.sqrt(magB));
  }

  async computeSimilarity(skills, query) {
    if (typeof this.embedFn === "function") {
      try {
        const queryVector = await this.embedFn(query);
        const skillTexts = skills.map(s => `${s.name} ${s.description} ${(s.tags || []).join(" ")}`);
        const skillVectors = await Promise.all(skillTexts.map(t => this.embedFn(t)));

        return skills.map((s, idx) => {
          const sim = this._cosineFromArrays(queryVector, skillVectors[idx]);
          return { skill: s, semanticScore: Math.max(0, Math.min(1, sim)) };
        });
      } catch {
        // Fall back to local n-gram vectorizer
      }
    }

    const queryVec = this._toNGramVector(query);
    return skills.map(s => {
      const text = `${s.name} ${s.description} ${(s.tags || []).join(" ")}`;
      const docVec = this._toNGramVector(text);
      const sim = this._cosineSimilarity(queryVec, docVec);
      return { skill: s, semanticScore: Math.max(0, Math.min(1, sim)) };
    });
  }

  _cosineFromArrays(a, b) {
    let dot = 0;
    let normA = 0;
    let normB = 0;
    for (let i = 0; i < a.length; i++) {
      dot += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }
    return dot / (Math.sqrt(normA) * Math.sqrt(normB) || 1);
  }
}
