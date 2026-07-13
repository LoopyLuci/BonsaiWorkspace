use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Entity {
    pub id: String,
    pub entity_type: String,
    pub properties: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Relation {
    pub source: String,
    pub target: String,
    pub relation_type: String,
    pub weight: f32,
}

pub struct KnowledgeGraph {
    entities: Arc<DashMap<String, Entity>>,
    relations: Arc<DashMap<String, Relation>>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            entities: Arc::new(DashMap::new()),
            relations: Arc::new(DashMap::new()),
        }
    }

    pub fn add_entity(&self, entity: Entity) {
        self.entities.insert(entity.id.clone(), entity);
    }

    pub fn add_relation(&self, relation: Relation) {
        let key = format!("{}_{}", relation.source, relation.target);
        self.relations.insert(key, relation);
    }

    pub fn get_entity(&self, id: &str) -> Option<Entity> {
        self.entities.get(id).map(|e| e.clone())
    }

    pub fn find_related(&self, source: &str) -> Vec<Relation> {
        self.relations
            .iter()
            .filter(|rel| rel.value().source == source)
            .map(|rel| rel.value().clone())
            .collect()
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    pub fn relation_count(&self) -> usize {
        self.relations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kg_entity() {
        let kg = KnowledgeGraph::new();
        let entity = Entity {
            id: "e1".to_string(),
            entity_type: "Agent".to_string(),
            properties: std::collections::HashMap::new(),
        };
        kg.add_entity(entity);
        assert_eq!(kg.entity_count(), 1);
    }

    #[test]
    fn test_kg_relation() {
        let kg = KnowledgeGraph::new();
        let rel = Relation {
            source: "e1".to_string(),
            target: "e2".to_string(),
            relation_type: "communicates_with".to_string(),
            weight: 0.8,
        };
        kg.add_relation(rel);
        assert_eq!(kg.relation_count(), 1);
    }
}
