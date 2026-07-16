use crate::{Entity, Relationship, Triple, GraphQuery, PathResult, GraphError, GraphResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct KnowledgeGraph {
    entities: Arc<DashMap<Uuid, Entity>>,
    relationships: Arc<DashMap<Uuid, Relationship>>,
    triples: Arc<DashMap<Uuid, Triple>>,
    queries: Arc<DashMap<Uuid, GraphQuery>>,
    paths: Arc<DashMap<Uuid, PathResult>>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            entities: Arc::new(DashMap::new()),
            relationships: Arc::new(DashMap::new()),
            triples: Arc::new(DashMap::new()),
            queries: Arc::new(DashMap::new()),
            paths: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_entity(&self, name: &str, entity_type: &str) -> GraphResult<Entity> {
        let entity = Entity {
            entity_id: Uuid::new_v4(),
            name: name.to_string(),
            entity_type: entity_type.to_string(),
            properties: vec![],
            created_at: Utc::now(),
        };

        self.entities.insert(entity.entity_id, entity.clone());
        Ok(entity)
    }

    pub async fn create_relationship(&self, source: Uuid, target: Uuid, rel_type: &str) -> GraphResult<Relationship> {
        if self.entities.get(&source).is_none() || self.entities.get(&target).is_none() {
            return Err(GraphError::EntityNotFound);
        }

        let relationship = Relationship {
            relationship_id: Uuid::new_v4(),
            source_entity: source,
            target_entity: target,
            relationship_type: rel_type.to_string(),
            properties: vec![],
            created_at: Utc::now(),
        };

        self.relationships.insert(relationship.relationship_id, relationship.clone());
        Ok(relationship)
    }

    pub async fn add_triple(&self, subject: Uuid, predicate: &str, object: &str) -> GraphResult<Triple> {
        if self.entities.get(&subject).is_none() {
            return Err(GraphError::EntityNotFound);
        }

        let triple = Triple {
            triple_id: Uuid::new_v4(),
            subject,
            predicate: predicate.to_string(),
            object: object.to_string(),
        };

        self.triples.insert(triple.triple_id, triple.clone());
        Ok(triple)
    }

    pub async fn query_graph(&self, pattern: &str) -> GraphResult<GraphQuery> {
        let mut count = 0;

        for entry in self.triples.iter() {
            if entry.value().predicate.contains(pattern) {
                count += 1;
            }
        }

        let query = GraphQuery {
            query_id: Uuid::new_v4(),
            pattern: pattern.to_string(),
            limit: 1000,
            results_count: count,
        };

        self.queries.insert(query.query_id, query.clone());
        Ok(query)
    }

    /// Find the shortest path between two entities via breadth-first search
    /// over the (directed) relationship graph. Returns a `PathResult` with
    /// an empty `path_entities` list if no path exists.
    pub async fn find_path(&self, start: Uuid, end: Uuid) -> GraphResult<PathResult> {
        if self.entities.get(&start).is_none() || self.entities.get(&end).is_none() {
            return Err(GraphError::EntityNotFound);
        }

        use std::collections::{HashMap, HashSet, VecDeque};

        let mut adjacency: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for entry in self.relationships.iter() {
            adjacency
                .entry(entry.value().source_entity)
                .or_default()
                .push(entry.value().target_entity);
        }

        let path_entities = if start == end {
            vec![start]
        } else {
            let mut visited: HashSet<Uuid> = HashSet::new();
            let mut parent: HashMap<Uuid, Uuid> = HashMap::new();
            let mut queue: VecDeque<Uuid> = VecDeque::new();

            visited.insert(start);
            queue.push_back(start);
            let mut reached = false;

            while let Some(current) = queue.pop_front() {
                if current == end {
                    reached = true;
                    break;
                }
                if let Some(neighbors) = adjacency.get(&current) {
                    for &next in neighbors {
                        if visited.insert(next) {
                            parent.insert(next, current);
                            queue.push_back(next);
                        }
                    }
                }
            }

            if reached {
                let mut path = vec![end];
                let mut cur = end;
                while cur != start {
                    cur = parent[&cur];
                    path.push(cur);
                }
                path.reverse();
                path
            } else {
                vec![]
            }
        };

        let path_length = path_entities.len().saturating_sub(1) as u32;

        let path = PathResult {
            path_id: Uuid::new_v4(),
            start_entity: start,
            end_entity: end,
            path_length,
            path_entities,
        };

        self.paths.insert(path.path_id, path.clone());
        Ok(path)
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_entity() {
        let graph = KnowledgeGraph::new();
        let entity = graph.create_entity("Alice", "Person").await.unwrap();

        assert_eq!(entity.name, "Alice");
        assert_eq!(entity.entity_type, "Person");
        assert_eq!(graph.entity_count(), 1);
    }

    #[tokio::test]
    async fn test_create_relationship() {
        let graph = KnowledgeGraph::new();
        let alice = graph.create_entity("Alice", "Person").await.unwrap();
        let bob = graph.create_entity("Bob", "Person").await.unwrap();

        let rel = graph.create_relationship(alice.entity_id, bob.entity_id, "knows").await.unwrap();
        assert_eq!(rel.relationship_type, "knows");
    }

    #[tokio::test]
    async fn test_add_triple() {
        let graph = KnowledgeGraph::new();
        let entity = graph.create_entity("Python", "Language").await.unwrap();

        let triple = graph.add_triple(entity.entity_id, "is_a", "Programming_Language").await.unwrap();
        assert_eq!(triple.predicate, "is_a");
    }

    #[tokio::test]
    async fn test_query_graph() {
        let graph = KnowledgeGraph::new();
        let entity = graph.create_entity("resource", "Type").await.unwrap();

        graph.add_triple(entity.entity_id, "has_property", "value").await.unwrap();

        let query = graph.query_graph("has_property").await.unwrap();
        assert_eq!(query.results_count, 1);
    }

    #[tokio::test]
    async fn test_find_path_direct_edge() {
        let graph = KnowledgeGraph::new();
        let alice = graph.create_entity("Alice", "Person").await.unwrap();
        let bob = graph.create_entity("Bob", "Person").await.unwrap();
        graph.create_relationship(alice.entity_id, bob.entity_id, "knows").await.unwrap();

        let path = graph.find_path(alice.entity_id, bob.entity_id).await.unwrap();
        assert_eq!(path.path_length, 1);
        assert_eq!(path.path_entities, vec![alice.entity_id, bob.entity_id]);
    }

    #[tokio::test]
    async fn test_find_path_multi_hop_bfs() {
        let graph = KnowledgeGraph::new();
        let alice = graph.create_entity("Alice", "Person").await.unwrap();
        let bob = graph.create_entity("Bob", "Person").await.unwrap();
        let carol = graph.create_entity("Carol", "Person").await.unwrap();

        // Alice -> Bob -> Carol: no direct edge from Alice to Carol.
        graph.create_relationship(alice.entity_id, bob.entity_id, "knows").await.unwrap();
        graph.create_relationship(bob.entity_id, carol.entity_id, "knows").await.unwrap();

        let path = graph.find_path(alice.entity_id, carol.entity_id).await.unwrap();
        assert_eq!(path.path_length, 2);
        assert_eq!(path.path_entities, vec![alice.entity_id, bob.entity_id, carol.entity_id]);
    }

    #[tokio::test]
    async fn test_find_path_no_path_returns_empty() {
        let graph = KnowledgeGraph::new();
        let alice = graph.create_entity("Alice", "Person").await.unwrap();
        let isolated = graph.create_entity("Isolated", "Person").await.unwrap();

        let path = graph.find_path(alice.entity_id, isolated.entity_id).await.unwrap();
        assert_eq!(path.path_length, 0);
        assert!(path.path_entities.is_empty());
    }

    #[tokio::test]
    async fn test_find_path_unknown_entity_errors() {
        let graph = KnowledgeGraph::new();
        let alice = graph.create_entity("Alice", "Person").await.unwrap();
        let result = graph.find_path(alice.entity_id, Uuid::new_v4()).await;
        assert!(matches!(result, Err(GraphError::EntityNotFound)));
    }

    #[tokio::test]
    async fn test_create_relationship_unknown_entity_errors() {
        let graph = KnowledgeGraph::new();
        let alice = graph.create_entity("Alice", "Person").await.unwrap();
        let result = graph.create_relationship(alice.entity_id, Uuid::new_v4(), "knows").await;
        assert!(matches!(result, Err(GraphError::EntityNotFound)));
    }
}
