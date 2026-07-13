//! CLI

use chrono::Utc;
use infrastructure_database::{DatabaseConfig, DatabaseEngine, DatabaseId, DatabaseManager, DatabaseProvisioner};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provisioner = DatabaseProvisioner::new();
    let config = DatabaseConfig {
        id: DatabaseId(Uuid::new_v4()),
        name: "production".to_string(),
        engine: DatabaseEngine::PostgreSQL,
        version: "14.5".to_string(),
        host: "localhost".to_string(),
        port: 5432,
        username: "admin".to_string(),
        password: "password".to_string(),
        database_name: "production".to_string(),
        max_connections: 20,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        tags: Default::default(),
    };

    let db = provisioner.create_database(config).await?;
    println!("Created database: {} ({})", db.name, db.engine);
    println!("Total databases: {}", provisioner.instance_count());

    Ok(())
}
