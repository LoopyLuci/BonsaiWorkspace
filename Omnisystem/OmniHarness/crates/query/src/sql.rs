//! Embedded SQLite query surface for Bonsai agents (100% offline).
//!
//! Wraps `rusqlite` to provide a JSON-oriented SQL execution API.

use rusqlite::{params_from_iter, types::ValueRef, Connection};
use serde_json::{Map, Value as JsonValue};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SqlError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("{0}")]
    Other(String),
}

pub type SqlResult<T> = Result<T, SqlError>;

/// An in-process SQLite connection.
pub struct SqlEngine {
    conn: Connection,
}

impl SqlEngine {
    /// Open an in-memory database.
    pub fn in_memory() -> SqlResult<Self> {
        Ok(Self {
            conn: Connection::open_in_memory()?,
        })
    }

    /// Open a file-based database.
    pub fn open(path: &str) -> SqlResult<Self> {
        Ok(Self {
            conn: Connection::open(path)?,
        })
    }

    /// Execute a DDL or DML statement (CREATE, INSERT, UPDATE, DELETE).
    pub fn execute(&self, sql: &str) -> SqlResult<usize> {
        Ok(self.conn.execute(sql, [])?)
    }

    /// Execute a parametrised DML statement.
    pub fn execute_params(&self, sql: &str, params: &[JsonValue]) -> SqlResult<usize> {
        let p: Vec<rusqlite::types::Value> = params.iter().map(json_to_sqlite).collect();
        Ok(self.conn.execute(sql, params_from_iter(p.iter()))?)
    }

    /// Execute a SELECT and return rows as JSON objects.
    pub fn query_json(&self, sql: &str) -> SqlResult<Vec<Map<String, JsonValue>>> {
        let mut stmt = self.conn.prepare(sql)?;
        let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        let rows = stmt.query_map([], |row| {
            let mut map = Map::new();
            for (i, name) in col_names.iter().enumerate() {
                let val = match row.get_ref(i)? {
                    ValueRef::Null => JsonValue::Null,
                    ValueRef::Integer(n) => JsonValue::Number(n.into()),
                    ValueRef::Real(f) => serde_json::Number::from_f64(f)
                        .map(JsonValue::Number)
                        .unwrap_or(JsonValue::Null),
                    ValueRef::Text(s) => JsonValue::String(String::from_utf8_lossy(s).into()),
                    ValueRef::Blob(b) => JsonValue::String(format!("<blob {} bytes>", b.len())),
                };
                map.insert(name.clone(), val);
            }
            Ok(map)
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Execute a parametrised SELECT.
    pub fn query_json_params(
        &self,
        sql: &str,
        params: &[JsonValue],
    ) -> SqlResult<Vec<Map<String, JsonValue>>> {
        let mut stmt = self.conn.prepare(sql)?;
        let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        let p: Vec<rusqlite::types::Value> = params.iter().map(json_to_sqlite).collect();
        let rows = stmt.query_map(params_from_iter(p.iter()), |row| {
            let mut map = Map::new();
            for (i, name) in col_names.iter().enumerate() {
                let val = match row.get_ref(i)? {
                    ValueRef::Null => JsonValue::Null,
                    ValueRef::Integer(n) => JsonValue::Number(n.into()),
                    ValueRef::Real(f) => serde_json::Number::from_f64(f)
                        .map(JsonValue::Number)
                        .unwrap_or(JsonValue::Null),
                    ValueRef::Text(s) => JsonValue::String(String::from_utf8_lossy(s).into()),
                    ValueRef::Blob(b) => JsonValue::String(format!("<blob {} bytes>", b.len())),
                };
                map.insert(name.clone(), val);
            }
            Ok(map)
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Return a flat list of scalar values from the first column.
    pub fn query_scalars(&self, sql: &str) -> SqlResult<Vec<JsonValue>> {
        let rows = self.query_json(sql)?;
        Ok(rows
            .into_iter()
            .filter_map(|mut m| {
                m.values_mut()
                    .next()
                    .map(|v| std::mem::replace(v, JsonValue::Null))
            })
            .collect())
    }

    /// Convenience: insert rows from JSON objects (must all share the same keys).
    pub fn insert_json_rows(
        &self,
        table: &str,
        rows: &[Map<String, JsonValue>],
    ) -> SqlResult<usize> {
        if rows.is_empty() {
            return Ok(0);
        }
        let cols: Vec<&str> = rows[0].keys().map(String::as_str).collect();
        let placeholders: Vec<&str> = vec!["?"; cols.len()];
        let sql = format!(
            "INSERT INTO {table} ({}) VALUES ({})",
            cols.join(", "),
            placeholders.join(", ")
        );
        let mut total = 0;
        for row in rows {
            let vals: Vec<JsonValue> = cols
                .iter()
                .map(|k| row.get(*k).cloned().unwrap_or(JsonValue::Null))
                .collect();
            total += self.execute_params(&sql, &vals)?;
        }
        Ok(total)
    }
}

fn json_to_sqlite(v: &JsonValue) -> rusqlite::types::Value {
    use rusqlite::types::Value;
    match v {
        JsonValue::Null => Value::Null,
        JsonValue::Bool(b) => Value::Integer(if *b { 1 } else { 0 }),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                Value::Real(f)
            } else {
                Value::Null
            }
        }
        JsonValue::String(s) => Value::Text(s.clone()),
        other => Value::Text(other.to_string()),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_db() -> SqlEngine {
        let db = SqlEngine::in_memory().unwrap();
        db.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, score REAL)")
            .unwrap();
        db.execute_params(
            "INSERT INTO users VALUES (?, ?, ?)",
            &[1.into(), "Alice".into(), serde_json::json!(88.5)],
        )
        .unwrap();
        db.execute_params(
            "INSERT INTO users VALUES (?, ?, ?)",
            &[2.into(), "Bob".into(), serde_json::json!(72.0)],
        )
        .unwrap();
        db
    }

    #[test]
    fn query_all() {
        let db = make_db();
        let rows = db.query_json("SELECT * FROM users ORDER BY id").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], JsonValue::String("Alice".into()));
    }

    #[test]
    fn query_params() {
        let db = make_db();
        let rows = db
            .query_json_params(
                "SELECT name FROM users WHERE score > ?",
                &[serde_json::json!(80)],
            )
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], JsonValue::String("Alice".into()));
    }

    #[test]
    fn scalars() {
        let db = make_db();
        let names = db
            .query_scalars("SELECT name FROM users ORDER BY id")
            .unwrap();
        assert_eq!(
            names,
            vec![
                JsonValue::String("Alice".into()),
                JsonValue::String("Bob".into())
            ]
        );
    }

    #[test]
    fn insert_json_rows() {
        let db = SqlEngine::in_memory().unwrap();
        db.execute("CREATE TABLE items (name TEXT, qty INTEGER)")
            .unwrap();
        let rows = vec![
            {
                let mut m = serde_json::Map::new();
                m.insert("name".into(), "apple".into());
                m.insert("qty".into(), serde_json::json!(5));
                m
            },
            {
                let mut m = serde_json::Map::new();
                m.insert("name".into(), "banana".into());
                m.insert("qty".into(), serde_json::json!(3));
                m
            },
        ];
        let n = db.insert_json_rows("items", &rows).unwrap();
        assert_eq!(n, 2);
        let r = db.query_json("SELECT COUNT(*) as c FROM items").unwrap();
        assert_eq!(r[0]["c"], serde_json::json!(2));
    }
}
