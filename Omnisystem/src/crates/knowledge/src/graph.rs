//! KnowledgeGraph — concurrent, queryable typed hypergraph.

use std::sync::Arc;

use dashmap::DashMap;
use tracing::debug;

use crate::types::*;

// ── Embedding index (simple flat cosine search — no external deps) ────────────

struct EmbeddingIndex {
    entries: DashMap<String, (String /* id */, Vec<f32> /* embedding */)>,
}

impl EmbeddingIndex {
    fn new() -> Self {
        Self {
            entries: DashMap::new(),
        }
    }

    fn upsert(&self, id: impl Into<String>, embedding: Vec<f32>) {
        let id = id.into();
        self.entries.insert(id.clone(), (id, embedding));
    }

    fn search(&self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        let mut scored: Vec<(String, f32)> = self
            .entries
            .iter()
            .map(|e| {
                let sim = cosine_similarity(query, &e.value().1);
                (e.value().0.clone(), sim)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    let dot: f32 = a[..len]
        .iter()
        .zip(b[..len].iter())
        .map(|(x, y)| x * y)
        .sum();
    let norm_a: f32 = a[..len].iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b[..len].iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

// ── KnowledgeGraph ────────────────────────────────────────────────────────────

/// Concurrent knowledge graph. All operations are thread-safe via DashMap.
pub struct KnowledgeGraph {
    entities: DashMap<EntityId, Entity>,
    relations: DashMap<RelationId, Relation>,
    beliefs: DashMap<BeliefId, Belief>,
    /// name → entity ID
    name_index: DashMap<String, EntityId>,
    /// concept label → [entity IDs]
    concept_index: DashMap<String, Vec<EntityId>>,
    embedding_index: Arc<EmbeddingIndex>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            entities: DashMap::new(),
            relations: DashMap::new(),
            beliefs: DashMap::new(),
            name_index: DashMap::new(),
            concept_index: DashMap::new(),
            embedding_index: Arc::new(EmbeddingIndex::new()),
        }
    }

    // ── Entities ──────────────────────────────────────────────────────────────

    pub fn upsert_entity(&self, entity: Entity) -> EntityId {
        let id = entity.id.clone();
        self.name_index.insert(entity.name.clone(), id.clone());
        // Update concept index
        let concept = format!("{:?}", entity.entity_type);
        self.concept_index
            .entry(concept)
            .or_default()
            .push(id.clone());
        // Update embedding index
        if let Some(ref emb) = entity.embeddings {
            self.embedding_index.upsert(id.clone(), emb.clone());
        }
        self.entities.insert(id.clone(), entity);
        debug!(id=%id, "entity upserted");
        id
    }

    pub fn get_entity(&self, id: &EntityId) -> Option<Entity> {
        self.entities.get(id).map(|e| e.clone())
    }

    pub fn find_entity_by_name(&self, name: &str) -> Option<Entity> {
        let id = self.name_index.get(name)?;
        self.entities.get(id.value()).map(|e| e.clone())
    }

    pub fn find_by_type(&self, entity_type: &EntityType) -> Vec<Entity> {
        self.entities
            .iter()
            .filter(|e| &e.entity_type == entity_type)
            .map(|e| e.clone())
            .collect()
    }

    pub fn find_by_concept(&self, concept: &str) -> Vec<Entity> {
        if let Some(ids) = self.concept_index.get(concept) {
            ids.iter()
                .filter_map(|id| self.entities.get(id).map(|e| e.clone()))
                .collect()
        } else {
            // Fallback: name substring search
            self.entities
                .iter()
                .filter(|e| e.name.to_lowercase().contains(&concept.to_lowercase()))
                .map(|e| e.clone())
                .collect()
        }
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    // ── Relations ─────────────────────────────────────────────────────────────

    pub fn add_relation(&self, relation: Relation) -> Result<RelationId, KnowledgeError> {
        if !self.entities.contains_key(&relation.subject) {
            return Err(KnowledgeError::EntityNotFound(relation.subject.clone()));
        }
        if let RelationTarget::Entity(ref target_id) = relation.object {
            if !self.entities.contains_key(target_id) {
                return Err(KnowledgeError::EntityNotFound(target_id.clone()));
            }
        }
        let id = relation.id.clone();
        self.relations.insert(id.clone(), relation);
        Ok(id)
    }

    pub fn relations_of(&self, entity_id: &EntityId) -> Vec<Relation> {
        self.relations
            .iter()
            .filter(|r| &r.subject == entity_id)
            .map(|r| r.clone())
            .collect()
    }

    pub fn find_by_predicate(
        &self,
        subject: &EntityId,
        predicate: &Predicate,
    ) -> Vec<RelationTarget> {
        self.relations
            .iter()
            .filter(|r| &r.subject == subject && &r.predicate == predicate)
            .map(|r| r.object.clone())
            .collect()
    }

    pub fn relation_count(&self) -> usize {
        self.relations.len()
    }

    /// Transitive closure for a given predicate (Warshall's algorithm).
    pub fn transitive_closure(&self, predicate: &Predicate) -> Vec<Relation> {
        let direct: Vec<Relation> = self
            .relations
            .iter()
            .filter(|r| &r.predicate == predicate)
            .map(|r| r.clone())
            .collect();

        let mut inferred = Vec::new();
        for r1 in &direct {
            for r2 in &direct {
                if let (RelationTarget::Entity(mid), RelationTarget::Entity(_)) =
                    (&r1.object, &r2.object)
                {
                    if mid == &r2.subject && r1.subject != r2.subject {
                        inferred.push(Relation {
                            id: new_relation_id(),
                            subject: r1.subject.clone(),
                            predicate: predicate.clone(),
                            object: r2.object.clone(),
                            confidence: (r1.confidence * r2.confidence * 0.9).min(1.0),
                            source: ProvenanceSource::Derived {
                                from_beliefs: vec![],
                                rule: "transitive_closure".into(),
                            },
                            temporal_bounds: None,
                            created_at: chrono::Utc::now().timestamp_millis(),
                        });
                    }
                }
            }
        }
        inferred
    }

    // ── Beliefs ───────────────────────────────────────────────────────────────

    pub fn add_belief(&self, belief: Belief) -> BeliefId {
        let id = belief.id.clone();
        self.beliefs.insert(id.clone(), belief);
        id
    }

    pub fn get_belief(&self, id: &BeliefId) -> Option<Belief> {
        self.beliefs.get(id).map(|b| b.clone())
    }

    pub fn update_belief(&self, id: &BeliefId, f: impl FnOnce(&mut Belief)) -> bool {
        if let Some(mut b) = self.beliefs.get_mut(id) {
            f(&mut b);
            true
        } else {
            false
        }
    }

    pub fn all_beliefs(&self) -> Vec<Belief> {
        self.beliefs.iter().map(|b| b.clone()).collect()
    }

    pub fn belief_count(&self) -> usize {
        self.beliefs.len()
    }

    // ── Semantic search ───────────────────────────────────────────────────────

    /// Search entities by embedding similarity. Returns empty if no embeddings exist.
    pub fn semantic_search_entities(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Vec<SearchResult> {
        let hits = self.embedding_index.search(query_embedding, top_k);
        hits.into_iter()
            .filter_map(|(id, score)| {
                self.entities.get(&id).map(|e| SearchResult {
                    score,
                    kind: SearchResultKind::Entity(e.clone()),
                })
            })
            .collect()
    }

    /// Full-text search over entity names and belief statements.
    pub fn text_search(&self, query: &str, top_k: usize) -> Vec<SearchResult> {
        let q = query.to_lowercase();
        let mut results: Vec<SearchResult> = Vec::new();

        for e in self.entities.iter() {
            if e.name.to_lowercase().contains(&q) {
                let score = if e.name.to_lowercase() == q { 1.0 } else { 0.7 };
                results.push(SearchResult {
                    score,
                    kind: SearchResultKind::Entity(e.clone()),
                });
            }
        }
        for b in self.beliefs.iter() {
            if b.statement.to_lowercase().contains(&q) {
                results.push(SearchResult {
                    score: 0.6,
                    kind: SearchResultKind::Belief(b.clone()),
                });
            }
        }

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(top_k);
        results
    }

    /// Summary stats for health checks.
    pub fn stats(&self) -> GraphStats {
        GraphStats {
            entity_count: self.entities.len(),
            relation_count: self.relations.len(),
            belief_count: self.beliefs.len(),
        }
    }
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphStats {
    pub entity_count: usize,
    pub relation_count: usize,
    pub belief_count: usize,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_crud() {
        let g = KnowledgeGraph::new();
        let e = Entity::new("Rust", EntityType::Concept);
        let id = g.upsert_entity(e.clone());
        assert!(g.get_entity(&id).is_some());
        assert_eq!(g.find_entity_by_name("Rust").unwrap().name, "Rust");
    }

    #[test]
    fn relation_add_and_query() {
        let g = KnowledgeGraph::new();
        let a = g.upsert_entity(Entity::new("Dog", EntityType::Concept));
        let b = g.upsert_entity(Entity::new("Animal", EntityType::Concept));
        let rel = Relation::new(
            a.clone(),
            Predicate::IsA,
            RelationTarget::Entity(b.clone()),
            ProvenanceSource::Derived {
                from_beliefs: vec![],
                rule: "test".into(),
            },
        );
        assert!(g.add_relation(rel).is_ok());
        let targets = g.find_by_predicate(&a, &Predicate::IsA);
        assert_eq!(targets.len(), 1);
    }

    #[test]
    fn transitive_closure_two_hops() {
        let g = KnowledgeGraph::new();
        let x = g.upsert_entity(Entity::new("Poodle", EntityType::Concept));
        let y = g.upsert_entity(Entity::new("Dog", EntityType::Concept));
        let z = g.upsert_entity(Entity::new("Animal", EntityType::Concept));
        let src = ProvenanceSource::Derived {
            from_beliefs: vec![],
            rule: "test".into(),
        };
        g.add_relation(Relation::new(
            x.clone(),
            Predicate::IsA,
            RelationTarget::Entity(y.clone()),
            src.clone(),
        ))
        .unwrap();
        g.add_relation(Relation::new(
            y.clone(),
            Predicate::IsA,
            RelationTarget::Entity(z.clone()),
            src,
        ))
        .unwrap();
        let inferred = g.transitive_closure(&Predicate::IsA);
        // Should infer Poodle → Animal
        assert!(inferred.iter().any(|r| r.subject == x));
    }

    #[test]
    fn text_search_finds_belief() {
        let g = KnowledgeGraph::new();
        g.add_belief(Belief::new("The sky is blue", 0.95));
        let results = g.text_search("sky", 5);
        assert!(!results.is_empty());
    }
}
