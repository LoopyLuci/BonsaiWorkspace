use crate::{EventError, EventResult, EventSubscription};
use dashmap::DashMap;
use std::sync::Arc;

pub struct Subscription {
    subs: Arc<DashMap<String, EventSubscription>>,
}

impl Subscription {
    pub fn new() -> Self {
        Self {
            subs: Arc::new(DashMap::new()),
        }
    }

    pub async fn subscribe(&self, sub: &EventSubscription) -> EventResult<()> {
        self.subs.insert(sub.subscription_id.clone(), sub.clone());
        Ok(())
    }

    pub async fn unsubscribe(&self, sub_id: &str) -> EventResult<()> {
        if self.subs.remove(sub_id).is_some() {
            Ok(())
        } else {
            Err(EventError::UnsubscribeFailed)
        }
    }

    pub fn count(&self) -> usize {
        self.subs.len()
    }
}

impl Default for Subscription {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_subscribe() {
        let sub_mgr = Subscription::new();
        let sub = EventSubscription {
            subscription_id: "sub1".to_string(),
            event_type: "user.created".to_string(),
            handler_id: "handler1".to_string(),
        };

        sub_mgr.subscribe(&sub).await.unwrap();
        assert_eq!(sub_mgr.count(), 1);
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let sub_mgr = Subscription::new();
        let sub = EventSubscription {
            subscription_id: "sub1".to_string(),
            event_type: "user.created".to_string(),
            handler_id: "handler1".to_string(),
        };

        sub_mgr.subscribe(&sub).await.unwrap();
        sub_mgr.unsubscribe("sub1").await.unwrap();
        assert_eq!(sub_mgr.count(), 0);
    }
}
