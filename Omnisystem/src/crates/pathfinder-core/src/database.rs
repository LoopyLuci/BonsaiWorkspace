// Database module for PATHFINDER
use sqlx::{postgres::PgPoolOptions, PgPool};
use anyhow::Result;

pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::query("CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            email TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            name TEXT,
            role TEXT NOT NULL DEFAULT 'student',
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE TABLE IF NOT EXISTS skills (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            difficulty TEXT NOT NULL,
            prerequisites TEXT ARRAY,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE TABLE IF NOT EXISTS exercises (
            id TEXT PRIMARY KEY,
            skill_id TEXT NOT NULL REFERENCES skills(id),
            title TEXT NOT NULL,
            description TEXT,
            exercise_type TEXT NOT NULL,
            options TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE TABLE IF NOT EXISTS skill_progress (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL REFERENCES users(id),
            skill_id TEXT NOT NULL REFERENCES skills(id),
            p_know FLOAT NOT NULL DEFAULT 0.0,
            attempts INT NOT NULL DEFAULT 0,
            correct_attempts INT NOT NULL DEFAULT 0,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}
