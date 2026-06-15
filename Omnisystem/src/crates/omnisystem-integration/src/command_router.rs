use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct RoutedCommand {
    pub command_id: String,
    pub source_system: String,
    pub target_system: String,
    pub command_type: String,
    pub arguments: Vec<String>,
    pub executed: bool,
}

pub struct CommandRouter {
    routes: Arc<DashMap<String, String>>,
    commands: Arc<DashMap<String, RoutedCommand>>,
}

impl CommandRouter {
    pub fn new() -> Self {
        Self {
            routes: Arc::new(DashMap::new()),
            commands: Arc::new(DashMap::new()),
        }
    }

    pub fn register_route(&self, command_type: String, target_system: String) {
        self.routes.insert(command_type, target_system);
    }

    pub fn route_command(&self, source: String, cmd_type: String, args: Vec<String>) -> Option<String> {
        if let Some(target) = self.routes.get(&cmd_type) {
            let command_id = format!("routed_cmd_{}", self.commands.len());
            let routed = RoutedCommand {
                command_id: command_id.clone(),
                source_system: source,
                target_system: target.value().clone(),
                command_type: cmd_type,
                arguments: args,
                executed: false,
            };
            self.commands.insert(command_id.clone(), routed);
            Some(command_id)
        } else {
            None
        }
    }

    pub fn mark_executed(&self, command_id: &str) -> bool {
        if let Some(mut cmd) = self.commands.get_mut(command_id) {
            cmd.executed = true;
            true
        } else {
            false
        }
    }

    pub fn get_command(&self, command_id: &str) -> Option<RoutedCommand> {
        self.commands.get(command_id).map(|c| c.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_routing() {
        let cr = CommandRouter::new();
        cr.register_route("search".to_string(), "usee".to_string());
        let cmd_id = cr.route_command("buddy".to_string(), "search".to_string(), vec!["query".to_string()]);
        assert!(cmd_id.is_some());
    }

    #[test]
    fn test_mark_executed() {
        let cr = CommandRouter::new();
        cr.register_route("search".to_string(), "usee".to_string());
        let cmd_id = cr.route_command("buddy".to_string(), "search".to_string(), vec![]).unwrap();
        assert!(cr.mark_executed(&cmd_id));
    }
}
