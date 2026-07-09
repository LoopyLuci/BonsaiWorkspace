//! CLI

use chrono::Utc;
use message_queue_core::{MessageBroker, Topic};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let broker = MessageBroker::new();
    let topic = Topic {
        topic_id: Uuid::new_v4(),
        name: "events".to_string(),
        partition_count: 10,
        replication_factor: 3,
        created_at: Utc::now(),
    };

    broker.create_topic(&topic).await?;
    println!("Created topic: {}", topic.name);
    println!("Total topics: {}", broker.topic_count());

    Ok(())
}
