export function rankSkills(skills, query) {
  if (!Array.isArray(skills)) throw new TypeError("skills must be an array");
  const normalized = String(query || "").toLowerCase().trim();
  if (!normalized) return [];
  const tokens = normalized.split(/[^a-z0-9+#.-]+/).filter(Boolean);
  return skills.map(skill => {
    const name = String(skill.name || "").toLowerCase();
    const description = String(skill.description || "").toLowerCase();
    const tags = (skill.tags || []).join(" ").toLowerCase();
    const category = String(skill.category || "").toLowerCase();
    let score = name.includes(normalized) ? 12 : 0;
    for (const token of tokens) {
      if (name.includes(token)) score += 5;
      if (tags.includes(token)) score += 3;
      if (description.includes(token)) score += 2;
      if (category.includes(token)) score += 1;
    }
    if (skill.verified) score += 0.5;
    score += Math.log10(Math.max(0, Number(skill.stars || 0)) + 1) / 10;
    return { ...skill, relevanceScore: Number(score.toFixed(3)) };
  }).filter(skill => skill.relevanceScore > 0)
    .sort((a,b) => b.relevanceScore - a.relevanceScore || Number(b.stars || 0) - Number(a.stars || 0));
}
