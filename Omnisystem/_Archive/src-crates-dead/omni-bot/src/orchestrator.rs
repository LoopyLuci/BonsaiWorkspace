use crate::{Result, OmniBotError, ServiceBridge};
use dashmap::DashMap;
use std::sync::Arc;

pub struct OmniBot {
    id: String,
    version: String,
    services: Arc<ServiceBridge>,
    state: Arc<std::sync::Mutex<BotState>>,
    active_tasks: Arc<DashMap<String, TaskStatus>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotState {
    Initializing,
    Ready,
    Processing,
    Error,
}

#[derive(Debug, Clone)]
pub struct TaskStatus {
    pub id: String,
    pub name: String,
    pub progress: f32,
    pub status: String,
}

impl OmniBot {
    pub fn new(version: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            version,
            services: Arc::new(ServiceBridge::new()),
            state: Arc::new(std::sync::Mutex::new(BotState::Initializing)),
            active_tasks: Arc::new(DashMap::new()),
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        tracing::info!("OmniBot initializing...");
        self.services.connect_all().await?;
        *self.state.lock().unwrap() = BotState::Ready;
        Ok(())
    }

    pub async fn execute_request(&self, request: RequestCommand) -> Result<String> {
        if *self.state.lock().unwrap() != BotState::Ready {
            return Err(OmniBotError::OrchestrationError("Not ready".to_string()));
        }

        *self.state.lock().unwrap() = BotState::Processing;
        
        let result = match request.service.as_str() {
            "iot" => self.services.route_to_iot(&request.payload).await,
            "search" => self.services.route_to_search(&request.payload).await,
            "fabrication" => self.services.route_to_fabrication(&request.payload).await,
            "agents" => self.services.route_to_agents(&request.payload).await,
            "network" => self.services.route_to_network(&request.payload).await,
            _ => Err(OmniBotError::ServiceNotAvailable(request.service)),
        };

        *self.state.lock().unwrap() = BotState::Ready;
        result
    }

    pub fn create_task(&self, name: String) -> String {
        let task_id = uuid::Uuid::new_v4().to_string();
        self.active_tasks.insert(
            task_id.clone(),
            TaskStatus {
                id: task_id.clone(),
                name,
                progress: 0.0,
                status: "pending".to_string(),
            },
        );
        task_id
    }

    pub fn update_task(&self, task_id: &str, progress: f32, status: String) -> Result<()> {
        if let Some(mut task) = self.active_tasks.get_mut(task_id) {
            task.progress = progress;
            task.status = status;
            Ok(())
        } else {
            Err(OmniBotError::OrchestrationError("Task not found".to_string()))
        }
    }

    pub fn get_state(&self) -> BotState {
        *self.state.lock().unwrap()
    }

    pub fn active_task_count(&self) -> usize {
        self.active_tasks.len()
    }
}

#[derive(Debug, Clone)]
pub struct RequestCommand {
    pub service: String,
    pub payload: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_omnibot_creation() {
        let bot = OmniBot::new("1.0.0".to_string());
        assert_eq!(bot.get_state(), BotState::Initializing);
    }

    #[tokio::test]
    async fn test_task_management() {
        let bot = OmniBot::new("1.0.0".to_string());
        let task_id = bot.create_task("test_task".to_string());
        assert!(bot.update_task(&task_id, 0.5, "running".to_string()).is_ok());
        assert_eq!(bot.active_task_count(), 1);
    }
}
