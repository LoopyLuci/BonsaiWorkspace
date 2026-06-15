// Bonsai Bot - Main Executable
// Production-grade, fully intelligent, bleeding-edge automation system

use bedf::bot::{BonsaiBot, AutomationType};
use std::env;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║                      BONSAI BOT                           ║");
    println!("║     Fully Intelligent • Production-Grade • Next-Gen       ║");
    println!("║    Capable of Automating Anything in the Bonsai Ecosystem ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();

    let args: Vec<String> = env::args().collect();

    // Initialize bot
    println!("🤖 Initializing Bonsai Bot...");
    let bot = BonsaiBot::new().await?;
    println!("✅ Bot initialized successfully!");
    println!("   Bot ID: {}", bot.get_id());
    println!("   Capabilities: {:?}", bot.get_capabilities());
    println!();

    // Handle commands
    if args.len() > 1 {
        match args[1].as_str() {
            "status" => {
                let metrics = bot.get_metrics();
                println!("📊 Bot Status:");
                println!("   Total Tasks: {}", metrics.total_tasks);
                println!("   Completed: {}", metrics.completed_tasks);
                println!("   Failed: {}", metrics.failed_tasks);
                println!("   Success Rate: {:.2}%", metrics.success_rate * 100.0);
            }
            "health" => {
                let health = bot.analyze_system_health().await?;
                println!("❤️  System Health:");
                println!("   Healthy Systems: {}/{}", health.healthy_systems, health.total_systems);
                println!("   Last Check: {}", health.last_check);
            }
            "optimize" => {
                println!("🚀 Starting performance optimization...");
                let result = bot.optimize_performance().await?;
                println!("✅ {}", result);
            }
            "execute" => {
                println!("⚡ Executing all pending tasks...");
                let completed = bot.execute_pending_tasks().await?;
                println!("✅ Completed {} tasks", completed.len());
            }
            "submit" => {
                if args.len() > 2 {
                    let task_desc = args[2..].join(" ");
                    println!("📝 Submitting task: {}", task_desc);
                    let task = bot
                        .submit_task(&task_desc, AutomationType::BuildAndTest)
                        .await?;
                    println!("✅ Task submitted: {}", task.id);
                } else {
                    println!("❌ Error: task description required");
                }
            }
            "interactive" => {
                interactive_mode(&bot).await?;
            }
            _ => {
                println!("❓ Unknown command: {}", args[1]);
                print_help();
            }
        }
    } else {
        print_help();
    }

    println!();
    println!("═══════════════════════════════════════════════════════════");
    println!("🤖 Bonsai Bot Ready for Ecosystem Automation");
    println!("═══════════════════════════════════════════════════════════");

    Ok(())
}

fn print_help() {
    println!("Available Commands:");
    println!("  status              Show bot status and metrics");
    println!("  health              Check system health");
    println!("  optimize            Run performance optimization");
    println!("  execute             Execute pending tasks");
    println!("  submit <task>       Submit a new automation task");
    println!("  interactive         Enter interactive mode");
}

async fn interactive_mode(bot: &BonsaiBot) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::{self, BufRead};

    println!("🤖 Interactive Mode - Type 'help' for commands");
    println!();

    let stdin = io::stdin();
    let reader = stdin.lock();

    for line in reader.lines() {
        let input = line?;
        let parts: Vec<&str> = input.trim().split_whitespace().collect();

        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "exit" => break,
            "help" => {
                println!("Available Commands:");
                println!("  status    - Show status");
                println!("  health    - Check health");
                println!("  optimize  - Optimize performance");
                println!("  execute   - Execute pending tasks");
                println!("  submit    - Submit task");
                println!("  exit      - Exit interactive mode");
            }
            "status" => {
                let metrics = bot.get_metrics();
                println!("Total Tasks: {}", metrics.total_tasks);
                println!("Completed: {}", metrics.completed_tasks);
            }
            "health" => {
                match bot.analyze_system_health().await {
                    Ok(health) => {
                        println!(
                            "Health: {}/{}",
                            health.healthy_systems, health.total_systems
                        );
                    }
                    Err(e) => println!("Error: {}", e),
                }
            }
            "execute" => {
                match bot.execute_pending_tasks().await {
                    Ok(tasks) => println!("Executed {} tasks", tasks.len()),
                    Err(e) => println!("Error: {}", e),
                }
            }
            _ => println!("Unknown command. Type 'help' for available commands."),
        }
    }

    Ok(())
}
