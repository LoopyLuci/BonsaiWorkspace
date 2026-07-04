use anyhow::Result;
use clap::{Parser, Subcommand};
use crate::client::{LauncherClient, MockLauncherClient};
use std::sync::Arc;
use std::io::{self, Write};

/// Launcher CLI - Command-line interface for app launching
#[derive(Parser, Debug)]
#[command(name = "launcher-cli")]
#[command(about = "Application launcher command-line interface", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// List all available applications
    List,

    /// Launch an application
    #[command(visible_alias = "run")]
    Launch {
        /// Application ID to launch
        app_id: String,

        /// Arguments to pass to the application
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Search for applications
    Search {
        /// Search query
        query: String,
    },

    /// Show system status
    Status,

    /// List running application instances
    Instances,

    /// Terminate a running application
    Terminate {
        /// Instance ID to terminate
        instance_id: String,
    },

    /// Show application details
    Show {
        /// Application ID
        app_id: String,
    },
}

pub struct UI;

impl UI {
    pub async fn render() -> Result<()> {
        let args = Args::parse();
        let client = Arc::new(MockLauncherClient::new());
        let cli = CLIInterface::new(client);

        // Check for interactive mode flag
        let raw_args: Vec<String> = std::env::args().collect();
        if raw_args.len() == 2 && (raw_args[1] == "-i" || raw_args[1] == "--interactive") {
            cli.interactive_mode().await?;
        } else {
            cli.execute(args.command).await?;
        }
        Ok(())
    }
}

pub struct CLIInterface {
    client: Arc<dyn LauncherClient>,
}

impl CLIInterface {
    pub fn new(client: Arc<dyn LauncherClient>) -> Self {
        Self { client }
    }

    pub async fn execute(&self, command: Command) -> Result<()> {
        match command {
            Command::List => self.list_apps().await?,
            Command::Launch { app_id, args } => self.launch_app(&app_id, args).await?,
            Command::Search { query } => self.search_apps(&query).await?,
            Command::Status => self.show_status().await?,
            Command::Instances => self.list_instances().await?,
            Command::Terminate { instance_id } => self.terminate_app(&instance_id).await?,
            Command::Show { app_id } => self.show_app(&app_id).await?,
        }
        Ok(())
    }

    async fn list_apps(&self) -> Result<()> {
        let apps = self.client.list_apps().await?;

        if apps.is_empty() {
            println!("No applications available");
            return Ok(());
        }

        println!("{:<20} {:<30} {:<10}", "ID", "Name", "Version");
        println!("{}", "─".repeat(60));
        for app in apps {
            println!("{:<20} {:<30} {:<10}", app.id, app.name, app.version);
        }
        Ok(())
    }

    async fn launch_app(&self, app_id: &str, args: Vec<String>) -> Result<()> {
        let response = self.client.launch_app(crate::client::LaunchRequest {
            app_id: app_id.to_string(),
            args,
            priority: "normal".to_string(),
        }).await?;

        println!("✓ Launched: {} ({})", app_id, response.instance_id);
        println!("  Status: {}", response.status);
        Ok(())
    }

    async fn search_apps(&self, query: &str) -> Result<()> {
        let results = self.client.search_apps(query).await?;

        if results.is_empty() {
            println!("No results for: {}", query);
            return Ok(());
        }

        println!("Search results for '{}': ", query);
        println!("{:<20} {:<30} {:<40}", "ID", "Name", "Description");
        println!("{}", "─".repeat(90));
        for app in results {
            println!(
                "{:<20} {:<30} {:<40}",
                app.id,
                app.name,
                app.description.chars().take(40).collect::<String>()
            );
        }
        Ok(())
    }

    async fn show_status(&self) -> Result<()> {
        let status = self.client.get_system_status().await?;

        println!("System Status:");
        println!("  Health: {}", if status.healthy { "✓ Healthy" } else { "✗ Unhealthy" });
        println!("  Uptime: {}s", status.uptime_ms / 1000);
        println!("  Active Instances: {}", status.active_instances);
        println!("  Total Apps: {}", status.total_apps);
        Ok(())
    }

    async fn list_instances(&self) -> Result<()> {
        let instances = self.client.list_instances().await?;

        if instances.is_empty() {
            println!("No running instances");
            return Ok(());
        }

        println!("{:<40} {:<20} {:<15} {:<10}", "Instance ID", "App ID", "Status", "PID");
        println!("{}", "─".repeat(85));
        for instance in instances {
            let pid = instance.pid.map(|p| p.to_string()).unwrap_or_default();
            println!(
                "{:<40} {:<20} {:<15} {:<10}",
                instance.instance_id.to_string(),
                instance.app_id,
                instance.status,
                pid
            );
        }
        Ok(())
    }

    async fn terminate_app(&self, instance_id: &str) -> Result<()> {
        let id = uuid::Uuid::parse_str(instance_id)
            .map_err(|_| anyhow::anyhow!("Invalid UUID format"))?;
        self.client.terminate_app(&id).await?;
        println!("✓ Terminated instance: {}", instance_id);
        Ok(())
    }

    async fn show_app(&self, app_id: &str) -> Result<()> {
        let app = self.client.get_app(app_id).await?
            .ok_or_else(|| anyhow::anyhow!("App not found: {}", app_id))?;

        println!("Application: {}", app.name);
        println!("  ID: {}", app.id);
        println!("  Version: {}", app.version);
        println!("  Description: {}", app.description);
        println!("  Executable: {}", app.executable);
        if let Some(icon) = app.icon {
            println!("  Icon: {}", icon);
        }
        Ok(())
    }

    pub async fn interactive_mode(&self) -> Result<()> {
        println!("╔══════════════════════════════════════╗");
        println!("║   LAUNCHER INTERACTIVE MODE          ║");
        println!("║   Type 'help' for available commands ║");
        println!("╚══════════════════════════════════════╝\n");

        let mut history = Vec::new();

        loop {
            print!("> ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                continue;
            }

            history.push(input.to_string());

            match input {
                "quit" | "exit" => {
                    println!("Goodbye!");
                    break;
                }
                "help" => self.show_interactive_help(),
                "list" => {
                    if let Err(e) = self.list_apps().await {
                        println!("Error: {}", e);
                    }
                }
                "status" => {
                    if let Err(e) = self.show_status().await {
                        println!("Error: {}", e);
                    }
                }
                "instances" => {
                    if let Err(e) = self.list_instances().await {
                        println!("Error: {}", e);
                    }
                }
                "history" => self.show_history(&history),
                cmd if cmd.starts_with("launch ") => {
                    let app_id = &cmd[7..];
                    if let Err(e) = self.launch_app(app_id, vec![]).await {
                        println!("Error: {}", e);
                    }
                }
                cmd if cmd.starts_with("search ") => {
                    let query = &cmd[7..];
                    if let Err(e) = self.search_apps(query).await {
                        println!("Error: {}", e);
                    }
                }
                cmd if cmd.starts_with("show ") => {
                    let app_id = &cmd[5..];
                    if let Err(e) = self.show_app(app_id).await {
                        println!("Error: {}", e);
                    }
                }
                _ => println!("Unknown command. Type 'help' for available commands."),
            }
            println!();
        }

        Ok(())
    }

    fn show_interactive_help(&self) {
        println!("\n📚 Available Commands:");
        println!("  list              - Show all available applications");
        println!("  launch <app_id>   - Launch an application");
        println!("  search <query>    - Search for applications");
        println!("  show <app_id>     - Show app details");
        println!("  status            - Show system status");
        println!("  instances         - List running instances");
        println!("  history           - Show command history");
        println!("  help              - Show this help message");
        println!("  quit/exit         - Exit interactive mode");
        println!();
    }

    fn show_history(&self, history: &[String]) {
        println!("📜 Command History ({} commands):", history.len());
        for (i, cmd) in history.iter().enumerate() {
            println!("  {}: {}", i + 1, cmd);
        }
    }

    pub fn generate_shell_completion(shell: &str) -> String {
        match shell {
            "bash" => Self::bash_completion(),
            "zsh" => Self::zsh_completion(),
            "fish" => Self::fish_completion(),
            _ => "Unknown shell".to_string(),
        }
    }

    fn bash_completion() -> String {
        r#"
_launcher_cli() {
    local cur prev words cword
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"

    if [[ ${cur} == -* ]] ; then
        COMPREPLY=( $(compgen -W "--help --version" -- ${cur}) )
        return 0
    fi

    case "${prev}" in
        launcher-cli)
            COMPREPLY=( $(compgen -W "list launch search show status instances terminate help" -- ${cur}) )
            return 0
            ;;
        launch)
            COMPREPLY=( $(launcher-cli list 2>/dev/null | awk '{print $1}' | grep "^${cur}") )
            return 0
            ;;
        search)
            return 0
            ;;
    esac

    COMPREPLY=( $(compgen -W "list launch search show status instances terminate help" -- ${cur}) )
}

complete -F _launcher_cli launcher-cli
        "#.to_string()
    }

    fn zsh_completion() -> String {
        r#"
#compdef launcher-cli

_launcher_cli() {
    local -a commands
    commands=(
        "list:List all applications"
        "launch:Launch an application"
        "search:Search for applications"
        "show:Show application details"
        "status:Show system status"
        "instances:List running instances"
        "terminate:Terminate an instance"
        "help:Show help message"
    )

    _describe 'command' commands
}

_launcher_cli "$@"
        "#.to_string()
    }

    fn fish_completion() -> String {
        r#"
complete -c launcher-cli -n "__fish_use_subcommand_from_list" -a "list" -d "List all applications"
complete -c launcher-cli -n "__fish_use_subcommand_from_list" -a "launch" -d "Launch an application"
complete -c launcher-cli -n "__fish_use_subcommand_from_list" -a "search" -d "Search for applications"
complete -c launcher-cli -n "__fish_use_subcommand_from_list" -a "show" -d "Show application details"
complete -c launcher-cli -n "__fish_use_subcommand_from_list" -a "status" -d "Show system status"
complete -c launcher-cli -n "__fish_use_subcommand_from_list" -a "instances" -d "List running instances"
complete -c launcher-cli -n "__fish_use_subcommand_from_list" -a "terminate" -d "Terminate an instance"
        "#.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cli_list_apps() {
        let client = Arc::new(MockLauncherClient::new());
        let cli = CLIInterface::new(client);
        assert!(cli.list_apps().await.is_ok());
    }

    #[tokio::test]
    async fn test_cli_search() {
        let client = Arc::new(MockLauncherClient::new());
        let cli = CLIInterface::new(client);
        assert!(cli.search_apps("text").await.is_ok());
    }

    #[tokio::test]
    async fn test_cli_status() {
        let client = Arc::new(MockLauncherClient::new());
        let cli = CLIInterface::new(client);
        assert!(cli.show_status().await.is_ok());
    }

    #[tokio::test]
    async fn test_cli_show_app() {
        let client = Arc::new(MockLauncherClient::new());
        let cli = CLIInterface::new(client);
        assert!(cli.show_app("app1").await.is_ok());
    }

    #[test]
    fn test_cli_ui_exists() {
        // Verify UI struct is accessible
        let _ = std::mem::size_of::<UI>();
    }
}
