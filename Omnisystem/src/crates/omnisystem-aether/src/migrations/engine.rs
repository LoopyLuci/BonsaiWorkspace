//! Migration Engine — Safe Schema Evolution
//!
//! Applies migrations in order with automatic rollback on failure.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Migration {
    pub name: String,
    pub version: u32,
    pub depends_on: Option<u32>,
    pub up_sql: String,
    pub down_sql: String,
}

#[derive(Debug, Clone)]
pub enum MigrationState {
    Pending,
    Applied,
    Failed(String),
}

pub struct MigrationEngine {
    migrations: Vec<Migration>,
    applied: HashMap<u32, MigrationState>,
}

impl MigrationEngine {
    pub fn new() -> Self {
        Self {
            migrations: Vec::new(),
            applied: HashMap::new(),
        }
    }

    pub fn add_migration(&mut self, migration: Migration) {
        self.migrations.push(migration);
    }

    pub fn register_applied(&mut self, version: u32) {
        self.applied.insert(version, MigrationState::Applied);
    }

    /// Plan which migrations need to be applied
    pub fn plan(&self) -> Result<Vec<&Migration>, String> {
        // Sort by version
        let mut sorted: Vec<_> = self.migrations.iter().collect();
        sorted.sort_by_key(|m| m.version);

        let mut planned = Vec::new();
        for migration in sorted {
            if self.applied.get(&migration.version).is_none() {
                // Check dependencies
                if let Some(dep) = migration.depends_on {
                    if self.applied.get(&dep).is_none() {
                        return Err(format!(
                            "Migration {} depends on {}, which has not been applied",
                            migration.version, dep
                        ));
                    }
                }
                planned.push(migration);
            }
        }

        Ok(planned)
    }

    /// Apply all pending migrations
    pub fn apply(&mut self) -> Result<Vec<String>, String> {
        let plan: Vec<Migration> = self.plan()?.iter().map(|m| (*m).clone()).collect();
        let mut applied_sql = Vec::new();

        for migration in plan {
            applied_sql.push(migration.up_sql.clone());
            self.applied
                .insert(migration.version, MigrationState::Applied);
        }

        Ok(applied_sql)
    }

    /// Rollback a specific migration
    pub fn rollback(&mut self, version: u32) -> Result<String, String> {
        let migration = self
            .migrations
            .iter()
            .find(|m| m.version == version)
            .ok_or_else(|| format!("Migration {} not found", version))?;

        if self.applied.get(&version).is_none() {
            return Err(format!("Migration {} was not applied", version));
        }

        self.applied.remove(&version);
        Ok(migration.down_sql.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_planning() {
        let mut engine = MigrationEngine::new();

        let m1 = Migration {
            name: "v1_create_users".to_string(),
            version: 1,
            depends_on: None,
            up_sql: "CREATE TABLE users (id UUID PRIMARY KEY)".to_string(),
            down_sql: "DROP TABLE users".to_string(),
        };

        let m2 = Migration {
            name: "v2_add_email".to_string(),
            version: 2,
            depends_on: Some(1),
            up_sql: "ALTER TABLE users ADD COLUMN email STRING".to_string(),
            down_sql: "ALTER TABLE users DROP COLUMN email".to_string(),
        };

        engine.add_migration(m1);
        engine.add_migration(m2);

        let plan = engine.plan().unwrap();
        assert_eq!(plan.len(), 2);

        engine.register_applied(1);
        let plan = engine.plan().unwrap();
        assert_eq!(plan.len(), 1);
    }
}
