use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Command {
    pub command_id: String,
    pub session_id: String,
    pub command_type: String,
    pub arguments: Vec<String>,
    pub executed: bool,
}

pub struct CommandExecutor {
    commands: Arc<DashMap<String, Command>>,
}

impl CommandExecutor {
    pub fn new() -> Self {
        Self {
            commands: Arc::new(DashMap::new()),
        }
    }

    pub fn execute_command(&self, session_id: String, command_type: String, args: Vec<String>) -> String {
        let command_id = format!("cmd_{}", self.commands.len());
        let command = Command {
            command_id: command_id.clone(),
            session_id,
            command_type,
            arguments: args,
            executed: true,
        };
        self.commands.insert(command_id.clone(), command);
        command_id
    }

    pub fn get_command(&self, command_id: &str) -> Option<Command> {
        self.commands.get(command_id).map(|c| c.clone())
    }

    pub fn command_count(&self) -> usize {
        self.commands.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_execution() {
        let ce = CommandExecutor::new();
        let cmd_id = ce.execute_command(
            "session1".to_string(),
            "file_list".to_string(),
            vec!["/home".to_string()],
        );
        let cmd = ce.get_command(&cmd_id).unwrap();
        assert!(cmd.executed);
    }
}
