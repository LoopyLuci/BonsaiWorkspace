// OMNISYSTEM CLI FRAMEWORK
// Complete command-line interface builder with subcommands and arguments

use std::collections::HashMap;
use std::sync::Arc;

// ============================================================================
// ARGUMENT TYPES
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ArgValue {
    String(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
    List(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct Argument {
    pub name: String,
    pub short: Option<char>,
    pub long: Option<String>,
    pub description: String,
    pub required: bool,
    pub value_type: String,
    pub default: Option<ArgValue>,
}

impl Argument {
    pub fn new(name: &str) -> Self {
        Argument {
            name: name.to_string(),
            short: None,
            long: None,
            description: String::new(),
            required: false,
            value_type: "string".to_string(),
            default: None,
        }
    }

    pub fn short(mut self, c: char) -> Self {
        self.short = Some(c);
        self
    }

    pub fn long(mut self, l: &str) -> Self {
        self.long = Some(l.to_string());
        self
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn value_type(mut self, t: &str) -> Self {
        self.value_type = t.to_string();
        self
    }

    pub fn default(mut self, val: ArgValue) -> Self {
        self.default = Some(val);
        self
    }
}

// ============================================================================
// COMMAND DEFINITION
// ============================================================================

pub type CommandHandler = Arc<dyn Fn(&CommandContext) -> Result<String, String> + Send + Sync>;

#[derive(Clone)]
pub struct Command {
    pub name: String,
    pub description: String,
    pub arguments: Vec<Argument>,
    pub handler: Option<CommandHandler>,
}

impl Command {
    pub fn new(name: &str, description: &str) -> Self {
        Command {
            name: name.to_string(),
            description: description.to_string(),
            arguments: Vec::new(),
            handler: None,
        }
    }

    pub fn with_handler(mut self, handler: CommandHandler) -> Self {
        self.handler = Some(handler);
        self
    }

    pub fn add_argument(mut self, arg: Argument) -> Self {
        self.arguments.push(arg);
        self
    }
}

// ============================================================================
// COMMAND CONTEXT
// ============================================================================

#[derive(Debug, Clone)]
pub struct CommandContext {
    pub command: String,
    pub arguments: HashMap<String, ArgValue>,
    pub flags: HashMap<String, bool>,
}

impl CommandContext {
    pub fn new(command: &str) -> Self {
        CommandContext {
            command: command.to_string(),
            arguments: HashMap::new(),
            flags: HashMap::new(),
        }
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        self.arguments.get(key).and_then(|v| {
            if let ArgValue::String(s) = v {
                Some(s.clone())
            } else {
                None
            }
        })
    }

    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.arguments.get(key).and_then(|v| {
            if let ArgValue::Integer(i) = v {
                Some(*i)
            } else {
                None
            }
        })
    }

    pub fn get_bool(&self, key: &str) -> bool {
        self.flags.get(key).copied().unwrap_or(false)
    }

    pub fn set(&mut self, key: String, value: ArgValue) {
        self.arguments.insert(key, value);
    }

    pub fn set_flag(&mut self, key: String, value: bool) {
        self.flags.insert(key, value);
    }
}

// ============================================================================
// CLI APPLICATION
// ============================================================================

pub struct CliApp {
    name: String,
    version: String,
    description: String,
    commands: HashMap<String, Command>,
    global_flags: Vec<Argument>,
}

impl CliApp {
    pub fn new(name: &str, version: &str, description: &str) -> Self {
        CliApp {
            name: name.to_string(),
            version: version.to_string(),
            description: description.to_string(),
            commands: HashMap::new(),
            global_flags: Vec::new(),
        }
    }

    pub fn add_command(&mut self, command: Command) {
        println!("📝 Command registered: {}", command.name);
        self.commands.insert(command.name.clone(), command);
    }

    pub fn add_global_flag(&mut self, flag: Argument) {
        self.global_flags.push(flag);
    }

    pub fn print_help(&self) {
        println!("\n{} v{}", self.name, self.version);
        println!("{}\n", self.description);

        println!("USAGE:");
        println!("    {} [COMMAND] [OPTIONS]\n", self.name);

        println!("COMMANDS:");
        for (_, cmd) in &self.commands {
            println!("    {:<20} {}", cmd.name, cmd.description);
        }

        if !self.global_flags.is_empty() {
            println!("\nGLOBAL FLAGS:");
            for flag in &self.global_flags {
                let name = if let Some(long) = &flag.long {
                    format!("--{}", long)
                } else {
                    format!("-{}", flag.short.unwrap_or('?'))
                };
                println!("    {:<20} {}", name, flag.description);
            }
        }
        println!();
    }

    pub fn print_version(&self) {
        println!("{} v{}", self.name, self.version);
    }

    pub fn execute(&self, args: Vec<String>) -> Result<String, String> {
        if args.is_empty() {
            self.print_help();
            return Ok("No command provided".to_string());
        }

        let command_name = &args[0];

        // Check for special commands
        if command_name == "--help" || command_name == "-h" || command_name == "help" {
            self.print_help();
            return Ok(String::new());
        }

        if command_name == "--version" || command_name == "-v" || command_name == "version" {
            self.print_version();
            return Ok(String::new());
        }

        // Find and execute command
        if let Some(command) = self.commands.get(command_name) {
            if let Some(handler) = &command.handler {
                let mut context = CommandContext::new(command_name);

                // Parse remaining arguments
                for (i, arg) in args.iter().enumerate().skip(1) {
                    if arg.starts_with("--") {
                        let key = &arg[2..];
                        context.set_flag(key.to_string(), true);
                    } else if arg.starts_with("-") {
                        let key = &arg[1..];
                        context.set_flag(key.to_string(), true);
                    } else if i > 0 && args[i - 1].starts_with("-") {
                        let prev_key = &args[i - 1][1..];
                        context.set(prev_key.to_string(), ArgValue::String(arg.clone()));
                    }
                }

                return handler(&context);
            }
        }

        Err(format!("Unknown command: {}", command_name))
    }
}

// ============================================================================
// INTERACTIVE CLI
// ============================================================================

pub struct InteractiveCli {
    app: Arc<CliApp>,
    prompt: String,
    history: Vec<String>,
}

impl InteractiveCli {
    pub fn new(app: Arc<CliApp>, prompt: &str) -> Self {
        InteractiveCli {
            app,
            prompt: prompt.to_string(),
            history: Vec::new(),
        }
    }

    pub fn print_welcome(&self) {
        println!("\n🎯 {} v{}", self.app.name, self.app.version);
        println!("Type 'help' for available commands\n");
    }

    pub fn run_command(&mut self, input: &str) -> Result<String, String> {
        let args: Vec<String> = input.split_whitespace()
            .map(|s| s.to_string())
            .collect();

        if args.is_empty() {
            return Ok(String::new());
        }

        self.history.push(input.to_string());
        self.app.execute(args)
    }

    pub fn show_history(&self) {
        println!("\nCommand History:");
        for (i, cmd) in self.history.iter().enumerate() {
            println!("  {}: {}", i + 1, cmd);
        }
        println!();
    }
}

// ============================================================================
// OUTPUT FORMATTING
// ============================================================================

pub struct TableBuilder {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl TableBuilder {
    pub fn new(headers: Vec<&str>) -> Self {
        TableBuilder {
            headers: headers.iter().map(|h| h.to_string()).collect(),
            rows: Vec::new(),
        }
    }

    pub fn add_row(&mut self, row: Vec<&str>) {
        self.rows.push(row.iter().map(|r| r.to_string()).collect());
    }

    pub fn print(&self) {
        if self.headers.is_empty() {
            return;
        }

        // Calculate column widths
        let mut widths: Vec<usize> = self.headers.iter().map(|h| h.len()).collect();

        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    widths[i] = widths[i].max(cell.len());
                }
            }
        }

        // Print header
        print!("┌");
        for (i, w) in widths.iter().enumerate() {
            print!("{}", "─".repeat(w + 2));
            if i < widths.len() - 1 {
                print!("┬");
            }
        }
        println!("┐");

        // Print headers
        print!("│");
        for (i, header) in self.headers.iter().enumerate() {
            print!(" {:<width$} ", header, width = widths[i]);
            print!("│");
        }
        println!();

        // Print separator
        print!("├");
        for (i, w) in widths.iter().enumerate() {
            print!("{}", "─".repeat(w + 2));
            if i < widths.len() - 1 {
                print!("┼");
            }
        }
        println!("┤");

        // Print rows
        for row in &self.rows {
            print!("│");
            for (i, cell) in row.iter().enumerate() {
                print!(" {:<width$} ", cell, width = widths[i]);
                print!("│");
            }
            println!();
        }

        // Print footer
        print!("└");
        for (i, w) in widths.iter().enumerate() {
            print!("{}", "─".repeat(w + 2));
            if i < widths.len() - 1 {
                print!("┴");
            }
        }
        println!("┘");
    }
}

// ============================================================================
// EXAMPLE APPLICATION
// ============================================================================

pub fn example_cli_app() -> CliApp {
    let mut app = CliApp::new(
        "omnisystem-cli",
        "1.0.0",
        "Omnisystem Command-Line Interface",
    );

    // Build command
    let build_cmd = Command::new("build", "Build a project")
        .add_argument(Argument::new("target").short('t').required().description("Build target"))
        .add_argument(Argument::new("release").short('r').description("Release mode"))
        .with_handler(Arc::new(|ctx| {
            let target = ctx.get_string("target").unwrap_or_else(|| "unknown".to_string());
            let release = ctx.get_bool("release");
            let mode = if release { "release" } else { "debug" };

            println!("🔨 Building {} in {} mode", target, mode);
            Ok("Build successful".to_string())
        }));

    // Run command
    let run_cmd = Command::new("run", "Run a project")
        .add_argument(Argument::new("script").short('s').required().description("Script to run"))
        .with_handler(Arc::new(|ctx| {
            let script = ctx.get_string("script").unwrap_or_else(|| "main".to_string());
            println!("▶️  Running: {}", script);
            Ok("Execution complete".to_string())
        }));

    // Deploy command
    let deploy_cmd = Command::new("deploy", "Deploy to environment")
        .add_argument(Argument::new("env").short('e').required().description("Environment"))
        .add_argument(Argument::new("version").short('v').description("Version"))
        .with_handler(Arc::new(|ctx| {
            let env = ctx.get_string("env").unwrap_or_else(|| "staging".to_string());
            let version = ctx.get_string("version").unwrap_or_else(|| "latest".to_string());
            println!("🚀 Deploying to {} (v{})", env, version);
            Ok("Deployment successful".to_string())
        }));

    // Status command
    let status_cmd = Command::new("status", "Show system status")
        .with_handler(Arc::new(|_| {
            let mut table = TableBuilder::new(vec!["Component", "Status", "Health"]);
            table.add_row(vec!["Titan", "✅ Running", "100%"]);
            table.add_row(vec!["Sylva", "✅ Running", "95%"]);
            table.add_row(vec!["Aether", "✅ Running", "98%"]);
            table.add_row(vec!["Framework", "✅ Ready", "100%"]);
            table.print();
            Ok(String::new())
        }));

    app.add_command(build_cmd);
    app.add_command(run_cmd);
    app.add_command(deploy_cmd);
    app.add_command(status_cmd);

    app.add_global_flag(
        Argument::new("verbose")
            .short('v')
            .description("Verbose output"),
    );

    app
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argument_builder() {
        let arg = Argument::new("name")
            .short('n')
            .long("name")
            .required()
            .description("User name");

        assert_eq!(arg.name, "name");
        assert_eq!(arg.short, Some('n'));
        assert_eq!(arg.long, Some("name".to_string()));
        assert!(arg.required);
    }

    #[test]
    fn test_command_creation() {
        let cmd = Command::new("test", "Test command");
        assert_eq!(cmd.name, "test");
        assert_eq!(cmd.description, "Test command");
    }

    #[test]
    fn test_command_context() {
        let mut ctx = CommandContext::new("test");
        ctx.set("key".to_string(), ArgValue::String("value".to_string()));

        assert_eq!(ctx.get_string("key"), Some("value".to_string()));
    }

    #[test]
    fn test_cli_app_creation() {
        let app = CliApp::new("test", "1.0.0", "Test app");
        assert_eq!(app.name, "test");
        assert_eq!(app.version, "1.0.0");
    }

    #[test]
    fn test_table_builder() {
        let mut table = TableBuilder::new(vec!["A", "B", "C"]);
        table.add_row(vec!["1", "2", "3"]);
        table.add_row(vec!["4", "5", "6"]);

        assert_eq!(table.headers.len(), 3);
        assert_eq!(table.rows.len(), 2);
    }

    #[test]
    fn test_interactive_cli() {
        let app = Arc::new(example_cli_app());
        let cli = InteractiveCli::new(app, "> ");
        assert_eq!(cli.prompt, "> ");
    }
}
