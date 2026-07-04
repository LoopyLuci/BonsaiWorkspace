//! Schema Definition — Types as Database Tables
//!
//! In Aether, `type` declarations simultaneously define:
//! - Aether language structs
//! - AriaDB table schemas
//! - Type-safe query targets

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Schema {
    pub name: String,
    pub entities: HashMap<String, EntityType>,
}

#[derive(Debug, Clone)]
pub struct EntityType {
    pub name: String,
    pub fields: Vec<Field>,
    pub indexes: Vec<Index>,
    pub relationships: Vec<Relationship>,
    pub temporal: Option<TemporalConfig>,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub field_type: FieldType,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub constraints: FieldConstraints,
}

#[derive(Debug, Clone)]
pub enum FieldType {
    Uuid,
    String,
    Int,
    Float,
    Bool,
    Timestamp,
    Json,
    Vector(usize), // dimension count
    List(Box<FieldType>),
    Set(Box<FieldType>),
    Map(Box<FieldType>, Box<FieldType>),
    Ref(String), // Reference to another entity
    Custom(String),
}

#[derive(Debug, Clone, Default)]
pub struct FieldConstraints {
    pub unique: bool,
    pub indexed: bool,
    pub vector_index: Option<VectorIndexConfig>,
}

#[derive(Debug, Clone)]
pub struct VectorIndexConfig {
    pub algorithm: String, // hnsw, flat, ivfflat, etc.
    pub metric: String,    // l2, cosine, hamming, etc.
    pub ef: usize,         // Effor HNSW
    pub m: usize,          // M for HNSW
}

#[derive(Debug, Clone)]
pub struct Index {
    pub name: String,
    pub fields: Vec<String>,
    pub index_type: IndexType,
}

#[derive(Debug, Clone)]
pub enum IndexType {
    BTree,
    Hash,
    VectorHnsw,
}

#[derive(Debug, Clone)]
pub struct Relationship {
    pub name: String,
    pub from_entity: String,
    pub to_entity: String,
    pub relation_type: RelationType,
}

#[derive(Debug, Clone)]
pub enum RelationType {
    OneToMany,
    ManyToMany,
    Edge, // Graph edge
}

#[derive(Debug, Clone)]
pub struct TemporalConfig {
    pub retention: TemporalRetention,
}

#[derive(Debug, Clone)]
pub enum TemporalRetention {
    Forever,
    Days(u32),
    Versions(u32),
}

impl Schema {
    pub fn new(name: String) -> Self {
        Self {
            name,
            entities: HashMap::new(),
        }
    }

    pub fn add_entity(&mut self, entity: EntityType) {
        self.entities.insert(entity.name.clone(), entity);
    }

    pub fn get_entity(&self, name: &str) -> Option<&EntityType> {
        self.entities.get(name)
    }

    /// Compile schema to SQL DDL statements
    pub fn to_sql(&self) -> String {
        let mut sql = String::new();
        for entity in self.entities.values() {
            sql.push_str(&entity.to_sql());
            sql.push('\n');
        }
        sql
    }
}

impl EntityType {
    pub fn new(name: String) -> Self {
        Self {
            name,
            fields: Vec::new(),
            indexes: Vec::new(),
            relationships: Vec::new(),
            temporal: None,
        }
    }

    pub fn add_field(&mut self, field: Field) {
        self.fields.push(field);
    }

    pub fn to_sql(&self) -> String {
        let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (\n", self.name);

        for (i, field) in self.fields.iter().enumerate() {
            sql.push_str(&format!("  {} {}", field.name, field.field_type.to_sql()));

            if !field.nullable {
                sql.push_str(" NOT NULL");
            }

            if let Some(default) = &field.default_value {
                sql.push_str(&format!(" DEFAULT {}", default));
            }

            if field.constraints.unique {
                sql.push_str(" UNIQUE");
            }

            if i < self.fields.len() - 1 {
                sql.push(',');
            }
            sql.push('\n');
        }

        sql.push_str(");\n");

        for index in &self.indexes {
            sql.push_str(&index.to_sql(&self.name));
        }

        sql
    }
}

impl FieldType {
    pub fn to_sql(&self) -> String {
        match self {
            FieldType::Uuid => "UUID".to_string(),
            FieldType::String => "VARCHAR(4096)".to_string(),
            FieldType::Int => "INT64".to_string(),
            FieldType::Float => "FLOAT64".to_string(),
            FieldType::Bool => "BOOLEAN".to_string(),
            FieldType::Timestamp => "TIMESTAMP".to_string(),
            FieldType::Json => "JSON".to_string(),
            FieldType::Vector(dim) => format!("VECTOR({})", dim),
            FieldType::List(t) => format!("ARRAY<{}>", t.to_sql()),
            FieldType::Set(t) => format!("SET<{}>", t.to_sql()),
            FieldType::Map(k, v) => format!("MAP<{},{}>", k.to_sql(), v.to_sql()),
            FieldType::Ref(entity) => format!("UUID -- foreign key to {}", entity),
            FieldType::Custom(name) => name.clone(),
        }
    }
}

impl Index {
    pub fn to_sql(&self, table: &str) -> String {
        let fields = self.fields.join(", ");
        let idx_type = match self.index_type {
            IndexType::BTree => "BTREE",
            IndexType::Hash => "HASH",
            IndexType::VectorHnsw => "HNSW",
        };
        format!(
            "CREATE INDEX {} ON {} ({}) USING {};\n",
            self.name, table, fields, idx_type
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_generation() {
        let mut entity = EntityType::new("User".to_string());
        entity.add_field(Field {
            name: "id".to_string(),
            field_type: FieldType::Uuid,
            nullable: false,
            default_value: None,
            constraints: FieldConstraints::default(),
        });

        let mut schema = Schema::new("TestDB".to_string());
        schema.add_entity(entity);

        let sql = schema.to_sql();
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS User"));
        assert!(sql.contains("id UUID"));
    }
}
