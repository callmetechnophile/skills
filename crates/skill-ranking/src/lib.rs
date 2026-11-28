use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use skill_registry::SkillMetadata;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RankingError {
    #[error("Ranking failed: {0}")]
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingWeights {
    pub relevance: f32,
    pub completeness: f32,
    pub provenance: f32,
    pub security: f32,
    pub compatibility: f32,
}

impl Default for RankingWeights {
    fn default() -> Self {
        Self {
            relevance: 0.40,
            completeness: 0.20,
            provenance: 0.15,
            security: 0.15,
            compatibility: 0.10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawMetrics {
    pub lexical: f32,
    pub semantic: f32,
    pub security_score: f32,
    pub compatibility_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingBreakdown {
    pub task_relevance: f32,
    pub capability_completeness: f32,
    pub provenance_maintenance: f32,
    pub security_trust: f32,
    pub runtime_compatibility: f32,
    pub raw_metrics: RawMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedSkill {
    pub skill: SkillMetadata,
    #[serde(rename = "relevanceScore")]
    pub relevance_score: f32,
    #[serde(rename = "scoreBreakdown")]
    pub score_breakdown: RankingBreakdown,
    #[serde(rename = "securityResult")]
    pub security_result: String,
    #[serde(rename = "compatibilityResult")]
    pub compatibility_result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectionRecord {
    pub skill: SkillMetadata,
    pub reason: String,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingResult {
    pub selected: Option<RankedSkill>,
    pub alternatives: Vec<RankedSkill>,
    pub ranked: Vec<RankedSkill>,
    pub rejections: Vec<RejectionRecord>,
}

pub struct LexicalMatcher {
    pub k1: f32,
    pub b: f32,
}

impl Default for LexicalMatcher {
    fn default() -> Self {
        Self { k1: 1.5, b: 0.75 }
    }
}

impl LexicalMatcher {
    pub fn tokenize(&self, text: &str) -> Vec<String> {
        let cleaned: String = text
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '+' || c == '#' || c == '.' || c == '-' {
                    c
                } else {
                    ' '
                }
            })
            .collect();

        cleaned
            .split_whitespace()
            .filter(|t| t.len() > 1)
            .map(|s| s.to_string())
            .collect()
    }

    pub fn compute_bm25(&self, skills: &[SkillMetadata], query: &str) -> Vec<(SkillMetadata, f32)> {
        let query_tokens = self.tokenize(query);
        if query_tokens.is_empty() || skills.is_empty() {
            return skills.iter().map(|s| (s.clone(), 0.0)).collect();
        }

        let n = skills.len() as f32;
        let mut docs_tokens = Vec::with_capacity(skills.len());

        for s in skills {
            let name_tokens = self.tokenize(&s.name);
            let tag_tokens = self.tokenize(&s.tags.join(" "));
            let desc_tokens = self.tokenize(&s.description);

            let mut tokens = Vec::new();
            for _ in 0..3 {
                tokens.extend(name_tokens.clone());
            }
            for _ in 0..2 {
                tokens.extend(tag_tokens.clone());
            }
            tokens.extend(desc_tokens);

            let len = if tokens.is_empty() {
                1.0
            } else {
                tokens.len() as f32
            };
            docs_tokens.push((tokens, len, s));
        }

        let avg_doc_len = docs_tokens.iter().map(|(_, l, _)| *l).sum::<f32>() / n;

        let mut df_map: HashMap<String, f32> = HashMap::new();
        for q_tok in &query_tokens {
            let mut count = 0.0;
            for (tokens, _, _) in &docs_tokens {
                if tokens.contains(q_tok) {
                    count += 1.0;
                }
            }
            df_map.insert(q_tok.clone(), count);
        }

        let norm_query = query.to_lowercase().trim().to_string();
        let mut results = Vec::new();

        for (tokens, doc_len, skill) in docs_tokens {
            let mut tf_map: HashMap<&str, f32> = HashMap::new();
            for tok in &tokens {
                *tf_map.entry(tok.as_str()).or_insert(0.0) += 1.0;
            }

            let mut score: f32 = 0.0;
            for q_tok in &query_tokens {
                let tf = tf_map.get(q_tok.as_str()).copied().unwrap_or(0.0);
                if tf == 0.0 {
                    continue;
                }
                let df = df_map.get(q_tok).copied().unwrap_or(0.0);
                let idf = ((1.0 + (n - df + 0.5) / (df + 0.5)).ln()).max(0.0);
                let num = tf * (self.k1 + 1.0);
                let denom = tf + self.k1 * (1.0 - self.b + self.b * (doc_len / avg_doc_len));
                score += idf * (num / denom);
            }

            if skill.name.to_lowercase().contains(&norm_query) {
                score += 3.0;
            }

            results.push(((*skill).clone(), score.max(0.0)));
        }

        results
    }
}

pub struct EmbeddingMatcher;

impl Default for EmbeddingMatcher {
    fn default() -> Self {
        Self
    }
}

impl EmbeddingMatcher {
    fn to_ngram_vector(&self, text: &str, n: usize) -> HashMap<String, f32> {
        let cleaned: String = text
            .to_lowercase()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect();
        let mut vec = HashMap::new();
        if cleaned.len() < n {
            return vec;
        }
        for i in 0..=(cleaned.len() - n) {
            let gram = &cleaned[i..i + n];
            *vec.entry(gram.to_string()).or_insert(0.0) += 1.0;
        }
        vec
    }

    fn cosine_similarity(&self, vec_a: &HashMap<String, f32>, vec_b: &HashMap<String, f32>) -> f32 {
        let mut dot = 0.0;
        let mut mag_a = 0.0;
        let mut mag_b = 0.0;

        for (k, v) in vec_a {
            mag_a += v * v;
            if let Some(other_v) = vec_b.get(k) {
                dot += v * other_v;
            }
        }
        for v in vec_b.values() {
            mag_b += v * v;
        }

        if mag_a == 0.0 || mag_b == 0.0 {
            return 0.0;
        }

        dot / (mag_a.sqrt() * mag_b.sqrt())
    }

    pub fn compute_similarity(
        &self,
        skills: &[SkillMetadata],
        query: &str,
    ) -> Vec<(SkillMetadata, f32)> {
        let query_vec = self.to_ngram_vector(query, 3);
        skills
            .iter()
            .map(|s| {
                let text = format!("{} {} {}", s.name, s.description, s.tags.join(" "));
                let doc_vec = self.to_ngram_vector(&text, 3);
                let sim = self.cosine_similarity(&query_vec, &doc_vec);
                (s.clone(), sim.clamp(0.0, 1.0))
            })
            .collect()
    }
}

pub struct HybridRanker {
    pub weights: RankingWeights,
    pub lexical_matcher: LexicalMatcher,
    pub embedding_matcher: EmbeddingMatcher,
    pub target_agent: String,
}

impl Default for HybridRanker {
    fn default() -> Self {
        Self::new("antigravity")
    }
}

impl HybridRanker {
    pub fn new(target_agent: &str) -> Self {
        Self {
            weights: RankingWeights::default(),
            lexical_matcher: LexicalMatcher::default(),
            embedding_matcher: EmbeddingMatcher,
            target_agent: target_agent.to_string(),
        }
    }

    fn score_completeness(&self, skill: &SkillMetadata) -> f32 {
        let mut score: f32 = 0.0;
        if skill.name.len() > 2 {
            score += 0.25;
        }
        if skill.description.len() > 15 {
            score += 0.35;
        }
        if !skill.tags.is_empty() {
            score += 0.20;
        }
        if skill.path.is_some() || skill.skill_directory.is_some() || skill.repository.is_some() {
            score += 0.20;
        }
        score.min(1.0)
    }

    fn score_provenance(&self, skill: &SkillMetadata) -> f32 {
        let mut score: f32 = 0.2;
        if skill.verified {
            score += 0.4;
        }
        if skill.source.as_deref() == Some("local") || skill.registry == "local" {
            score += 0.4;
        } else if let Some(repo) = &skill.repository {
            if repo.contains('/') {
                score += 0.2;
            }
        }

        let stars = skill.stars as f64;
        let star_bonus = (0.2_f64).min(stars.max(1.0).log10() / 20.0) as f32;
        score += star_bonus;
        score.min(1.0)
    }

    fn score_security(&self, skill: &SkillMetadata) -> f32 {
        let status = skill.security_status.to_uppercase();
        if status.contains("REJECTED") || status.contains("QUARANTINED") {
            0.0
        } else if status.contains("RESTRICTED") {
            0.3
        } else if status.contains("UNSCANNED") {
            0.5
        } else if status.contains("APPROVED") || status.contains("LOCAL") {
            1.0
        } else {
            0.5
        }
    }

    fn score_compatibility(&self, skill: &SkillMetadata, target_agent: &str) -> f32 {
        let agents = match &skill.supported_agents {
            Some(a) => a,
            None => return 0.9,
        };
        let agent_lower = target_agent.to_lowercase();
        if agents
            .iter()
            .any(|a| a == "*" || a.to_lowercase() == agent_lower)
        {
            1.0
        } else {
            0.1
        }
    }

    pub fn rank(
        &self,
        skills: &[SkillMetadata],
        query: &str,
        target_agent: Option<&str>,
        strict_compatibility: bool,
    ) -> RankingResult {
        let empty_res = RankingResult {
            selected: None,
            alternatives: vec![],
            ranked: vec![],
            rejections: vec![],
        };

        if skills.is_empty() || query.trim().is_empty() {
            return empty_res;
        }

        let agent = target_agent.unwrap_or(&self.target_agent);
        let lexical_results = self.lexical_matcher.compute_bm25(skills, query);
        let max_lexical = lexical_results
            .iter()
            .map(|(_, s)| *s)
            .fold(1.0_f32, f32::max);

        let semantic_results = self.embedding_matcher.compute_similarity(skills, query);
        let semantic_map: HashMap<String, f32> = semantic_results
            .into_iter()
            .map(|(s, score)| (s.id.clone(), score))
            .collect();

        let mut scored = Vec::new();
        let mut rejections = Vec::new();

        for (skill, raw_lexical) in lexical_results {
            let norm_lexical = (raw_lexical / max_lexical).min(1.0);
            let raw_semantic = semantic_map.get(&skill.id).copied().unwrap_or(0.0);

            let relevance_score = (norm_lexical * 0.6) + (raw_semantic * 0.4);
            let completeness_score = self.score_completeness(&skill);
            let provenance_score = self.score_provenance(&skill);
            let security_score = self.score_security(&skill);
            let compatibility_score = self.score_compatibility(&skill, agent);

            if security_score == 0.0 {
                rejections.push(RejectionRecord {
                    skill: skill.clone(),
                    reason: "SECURITY_REJECTION".to_string(),
                    details: format!(
                        "Skill has untrusted security status: {}",
                        skill.security_status
                    ),
                });
                continue;
            }

            if compatibility_score < 0.2 && strict_compatibility {
                rejections.push(RejectionRecord {
                    skill: skill.clone(),
                    reason: "INCOMPATIBLE_RUNTIME".to_string(),
                    details: format!("Skill does not support target agent '{}'", agent),
                });
                continue;
            }

            let total_score = (relevance_score * self.weights.relevance)
                + (completeness_score * self.weights.completeness)
                + (provenance_score * self.weights.provenance)
                + (security_score * self.weights.security)
                + (compatibility_score * self.weights.compatibility);

            if relevance_score <= 0.01 && !query.contains('*') {
                continue;
            }

            let rounded_total = (total_score * 1000.0).round() / 1000.0;
            scored.push(RankedSkill {
                skill: skill.clone(),
                relevance_score: rounded_total,
                score_breakdown: RankingBreakdown {
                    task_relevance: ((relevance_score * self.weights.relevance) * 1000.0).round()
                        / 1000.0,
                    capability_completeness: ((completeness_score * self.weights.completeness)
                        * 1000.0)
                        .round()
                        / 1000.0,
                    provenance_maintenance: ((provenance_score * self.weights.provenance) * 1000.0)
                        .round()
                        / 1000.0,
                    security_trust: ((security_score * self.weights.security) * 1000.0).round()
                        / 1000.0,
                    runtime_compatibility: ((compatibility_score * self.weights.compatibility)
                        * 1000.0)
                        .round()
                        / 1000.0,
                    raw_metrics: RawMetrics {
                        lexical: (raw_lexical * 100.0).round() / 100.0,
                        semantic: (raw_semantic * 100.0).round() / 100.0,
                        security_score,
                        compatibility_score,
                    },
                },
                security_result: if security_score >= 0.8 {
                    "APPROVED".to_string()
                } else if security_score >= 0.3 {
                    "RESTRICTED".to_string()
                } else {
                    "REJECTED".to_string()
                },
                compatibility_result: if compatibility_score >= 0.5 {
                    "COMPATIBLE".to_string()
                } else {
                    "INCOMPATIBLE".to_string()
                },
            });
        }

        scored.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let selected = scored.first().cloned();
        let alternatives = if scored.len() > 1 {
            scored[1..].iter().take(5).cloned().collect()
        } else {
            vec![]
        };

        RankingResult {
            selected,
            alternatives,
            ranked: scored,
            rejections,
        }
    }
}

#[async_trait]
pub trait SkillRanker: Send + Sync {
    async fn rank_candidates(
        &self,
        candidates: &[SkillMetadata],
        query: &str,
        target_agent: &str,
    ) -> Result<RankingResult, RankingError>;
}

#[async_trait]
impl SkillRanker for HybridRanker {
    async fn rank_candidates(
        &self,
        candidates: &[SkillMetadata],
        query: &str,
        target_agent: &str,
    ) -> Result<RankingResult, RankingError> {
        Ok(self.rank(candidates, query, Some(target_agent), true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ranking_bm25_and_hybrid() {
        let ranker = HybridRanker::new("antigravity");
        let skills = vec![
            SkillMetadata {
                id: "sort-the-issue".to_string(),
                name: "sort-the-issue".to_string(),
                slug: "sort-the-issue".to_string(),
                description: "Mandatory triage and decomposition".to_string(),
                path: Some("skills/core/sort-the-issue/SKILL.md".to_string()),
                repository: Some("callmetechnophile/skills".to_string()),
                skill_directory: Some("sort-the-issue".to_string()),
                license: "Apache-2.0".to_string(),
                security_status: "approved-local".to_string(),
                verified: true,
                supported_agents: Some(vec!["antigravity".to_string()]),
                tags: vec!["core".to_string(), "triage".to_string()],
                stars: 50,
                registry: "local".to_string(),
                source: Some("local".to_string()),
            },
            SkillMetadata {
                id: "unrelated".to_string(),
                name: "unrelated".to_string(),
                slug: "unrelated".to_string(),
                description: "Does something completely different".to_string(),
                path: None,
                repository: None,
                skill_directory: None,
                license: "MIT".to_string(),
                security_status: "approved-local".to_string(),
                verified: true,
                supported_agents: None,
                tags: vec![],
                stars: 0,
                registry: "local".to_string(),
                source: None,
            },
        ];

        let result = ranker.rank(
            &skills,
            "sort-the-issue triage decomposition",
            Some("antigravity"),
            true,
        );
        assert!(result.selected.is_some());
        assert_eq!(result.selected.unwrap().skill.id, "sort-the-issue");
    }
}
