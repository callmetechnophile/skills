/**
 * BM25 lexical matcher with multi-field term weighting.
 */

export class LexicalMatcher {
  constructor(options = {}) {
    this.k1 = options.k1 ?? 1.5;
    this.b = options.b ?? 0.75;
  }

  tokenize(text) {
    if (!text) return [];
    return String(text)
      .toLowerCase()
      .replace(/[^a-z0-9+#.-]+/g, " ")
      .split(/\s+/)
      .filter(t => t.length > 1);
  }

  computeBM25(skills, query) {
    const queryTokens = this.tokenize(query);
    if (!queryTokens.length || !skills.length) {
      return skills.map(s => ({ skill: s, lexicalScore: 0 }));
    }

    const N = skills.length;
    // Build document representations with weighted fields
    const docs = skills.map(s => {
      const nameTokens = this.tokenize(s.name);
      const tagTokens = this.tokenize((s.tags || []).join(" "));
      const descTokens = this.tokenize(s.description);
      const catTokens = this.tokenize(s.category);

      // Weighted token list
      const tokens = [
        ...nameTokens, ...nameTokens, ...nameTokens, // 3x weight
        ...tagTokens, ...tagTokens,                  // 2x weight
        ...descTokens,                               // 1x weight
        ...catTokens                                 // 1x weight
      ];

      return {
        tokens,
        len: tokens.length || 1,
        rawSkill: s
      };
    });

    const avgDocLen = docs.reduce((sum, d) => sum + d.len, 0) / N;

    // Calculate document frequencies for query tokens
    const dfMap = new Map();
    for (const token of queryTokens) {
      let count = 0;
      for (const d of docs) {
        if (d.tokens.includes(token)) count++;
      }
      dfMap.set(token, count);
    }

    // Score each document
    const scored = docs.map(doc => {
      let score = 0;
      const tfMap = new Map();
      for (const t of doc.tokens) {
        tfMap.set(t, (tfMap.get(t) || 0) + 1);
      }

      for (const token of queryTokens) {
        const tf = tfMap.get(token) || 0;
        if (tf === 0) continue;
        const df = dfMap.get(token) || 0;
        const idf = Math.log(1 + (N - df + 0.5) / (df + 0.5));
        const num = tf * (this.k1 + 1);
        const denom = tf + this.k1 * (1 - this.b + this.b * (doc.len / avgDocLen));
        score += idf * (num / denom);
      }

      // Check exact query inclusion in name
      const normQuery = query.toLowerCase().trim();
      if (doc.rawSkill.name && doc.rawSkill.name.toLowerCase().includes(normQuery)) {
        score += 3.0;
      }

      return {
        skill: doc.rawSkill,
        lexicalScore: Math.max(0, score)
      };
    });

    return scored;
  }
}
