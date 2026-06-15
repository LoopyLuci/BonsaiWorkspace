use crate::{Topic, Partition, Message, ConsumerGroup, ConsumerOffset, Broker, QueueError, QueueResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct MessageBroker {
    topics: Arc<DashMap<String, Topic>>,
    partitions: Arc<DashMap<String, Vec<Partition>>>,
    messages: Arc<DashMap<String, Vec<Message>>>,
    consumer_groups: Arc<DashMap<String, ConsumerGroup>>,
    offsets: Arc<DashMap<String, ConsumerOffset>>,
    brokers: Arc<DashMap<u32, Broker>>,
}

impl MessageBroker {
    pub fn new() -> Self {
        Self {
            topics: Arc::new(DashMap::new()),
            partitions: Arc::new(DashMap::new()),
            messages: Arc::new(DashMap::new()),
            consumer_groups: Arc::new(DashMap::new()),
            offsets: Arc::new(DashMap::new()),
            brokers: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_topic(&self, topic: &Topic) -> QueueResult<()> {
        self.topics.insert(topic.name.clone(), topic.clone());
        Ok(())
    }

    pub async fn get_topic(&self, topic_name: &str) -> QueueResult<Topic> {
        self.topics
            .get(topic_name)
            .map(|t| t.clone())
            .ok_or(QueueError::TopicNotFound)
    }

    pub async fn publish_message(&self, message: &Message) -> QueueResult<()> {
        if !self.topics.contains_key(&message.topic_name) {
            return Err(QueueError::TopicNotFound);
        }

        let key = format!("{}-{}", message.topic_name, message.partition);
        self.messages.entry(key).or_insert_with(Vec::new).push(message.clone());
        Ok(())
    }

    pub async fn consume_message(&self, topic_name: &str, partition: u32, offset: u64) -> QueueResult<Message> {
        let key = format!("{}-{}", topic_name, partition);
        
        if let Some(messages) = self.messages.get(&key) {
            if let Some(msg) = messages.get(offset as usize) {
                Ok(msg.clone())
            } else {
                Err(QueueError::OffsetOutOfRange)
            }
        } else {
            Err(QueueError::PartitionNotFound)
        }
    }

    pub async fn create_consumer_group(&self, group: &ConsumerGroup) -> QueueResult<()> {
        self.consumer_groups.insert(group.name.clone(), group.clone());
        Ok(())
    }

    pub async fn commit_offset(&self, offset: &ConsumerOffset) -> QueueResult<()> {
        let key = format!("{}-{}-{}", offset.group_name, offset.topic_name, offset.partition);
        self.offsets.insert(key, offset.clone());
        Ok(())
    }

    pub async fn register_broker(&self, broker: &Broker) -> QueueResult<()> {
        self.brokers.insert(broker.broker_id, broker.clone());
        Ok(())
    }

    pub fn topic_count(&self) -> usize {
        self.topics.len()
    }

    pub fn broker_count(&self) -> usize {
        self.brokers.len()
    }
}

impl Default for MessageBroker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_topic() {
        let broker = MessageBroker::new();
        let topic = Topic {
            topic_id: Uuid::new_v4(),
            name: "events".to_string(),
            partition_count: 10,
            replication_factor: 3,
            created_at: Utc::now(),
        };

        broker.create_topic(&topic).await.unwrap();
        assert_eq!(broker.topic_count(), 1);
    }

    #[tokio::test]
    async fn test_publish_message() {
        let broker = MessageBroker::new();
        let topic = Topic {
            topic_id: Uuid::new_v4(),
            name: "logs".to_string(),
            partition_count: 5,
            replication_factor: 2,
            created_at: Utc::now(),
        };

        broker.create_topic(&topic).await.unwrap();

        let message = Message {
            message_id: Uuid::new_v4(),
            topic_name: "logs".to_string(),
            partition: 0,
            offset: 0,
            key: Some("app1".to_string()),
            value: b"log message".to_vec(),
            timestamp: Utc::now(),
        };

        broker.publish_message(&message).await.unwrap();
    }

    #[tokio::test]
    async fn test_create_consumer_group() {
        let broker = MessageBroker::new();
        let group = ConsumerGroup {
            group_id: Uuid::new_v4(),
            name: "analytics".to_string(),
            members: vec!["consumer1".to_string()],
            topics: vec!["events".to_string()],
            created_at: Utc::now(),
        };

        broker.create_consumer_group(&group).await.unwrap();
    }

    #[tokio::test]
    async fn test_register_broker() {
        let broker_mgr = MessageBroker::new();
        let broker = Broker {
            broker_id: 1,
            host: "localhost".to_string(),
            port: 9092,
            is_leader: true,
        };

        broker_mgr.register_broker(&broker).await.unwrap();
        assert_eq!(broker_mgr.broker_count(), 1);
    }
}
