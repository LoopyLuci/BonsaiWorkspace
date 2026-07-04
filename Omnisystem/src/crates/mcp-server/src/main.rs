use clap::{Parser, Subcommand};
use mcp_server::uacs::{HeadlessConfig, HITLConfig, ApprovalCategory};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "uacs")]
#[command(about = "Universal Agent Control System — Visual & Headless Agent Control with Human-In-The-Loop")]
struct Cli {
    #[command(subcommand)]
    mode: Option<UacsMode>,
}

#[derive(Subcommand)]
enum UacsMode {
    /// Run in Visual Agent Control mode with a live dashboard and HITL approval modal.
    Visual {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 11426)]
        port: u16,
        #[arg(long, default_value = "destructive,network")]
        hitl_categories: String,
        #[arg(long)]
        no_hitl: bool,
    },
    /// Run in Headless Agent Control mode with HITL terminal/notification prompts.
    Headless {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 11425)]
        port: u16,
        #[arg(long)]
        quiet: bool,
        #[arg(long)]
        verbose: bool,
        #[arg(long)]
        notify_on_error: bool,
        #[arg(long)]
        notify_on_success: bool,
        #[arg(long)]
        popup_on_approval: bool,
        #[arg(long, default_value = "uacs-agent.log")]
        log_path: String,
        #[arg(long, default_value = "destructive,network")]
        hitl_categories: String,
        #[arg(long)]
        no_hitl: bool,
        #[arg(long)]
        fallback_terminal: bool,
    },
}

fn parse_hitl_categories(s: &str) -> Vec<ApprovalCategory> {
    s.split(',')
        .filter_map(|part| match part.trim().to_lowercase().as_str() {
            "destructive" => Some(ApprovalCategory::Destructive),
            "network" => Some(ApprovalCategory::Network),
            "model" => Some(ApprovalCategory::ModelMutation),
            "system" => Some(ApprovalCategory::SystemModification),
            "all" => Some(ApprovalCategory::All),
            _ => None,
        })
        .collect()
}

fn build_hitl_config(enabled: bool, categories: &str, fallback_terminal: bool) -> HITLConfig {
    HITLConfig {
        enabled,
        approval_categories: if enabled {
            parse_hitl_categories(categories)
        } else {
            vec![]
        },
        fallback_terminal,
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.mode {
        Some(UacsMode::Visual {
            host,
            port,
            hitl_categories,
            no_hitl,
        }) => {
            let hitl = build_hitl_config(!no_hitl, &hitl_categories, false);
            tracing::info!("Starting Universal Agent Control System in Visual Mode");
            if hitl.enabled {
                tracing::info!("Human-In-The-Loop: ENABLED (categories: {})", hitl_categories);
            } else {
                tracing::warn!("Human-In-The-Loop: DISABLED");
            }
            mcp_server::server::run_uacs_visual(&host, port, hitl)
                .await
                .unwrap();
        }
        Some(UacsMode::Headless {
            host,
            port,
            quiet,
            verbose,
            notify_on_error,
            notify_on_success,
            popup_on_approval,
            log_path,
            hitl_categories,
            no_hitl,
            fallback_terminal,
        }) => {
            let config = HeadlessConfig {
                quiet,
                verbose: verbose && !quiet,
                notify_on_error,
                notify_on_success,
                popup_on_approval,
                log_path: PathBuf::from(log_path),
            };
            let hitl = build_hitl_config(!no_hitl, &hitl_categories, fallback_terminal);
            tracing::info!("Starting Universal Agent Control System in Headless Mode");
            if hitl.enabled {
                tracing::info!("Human-In-The-Loop: ENABLED (categories: {})", hitl_categories);
                if fallback_terminal {
                    tracing::info!("HITL fallback: Terminal prompts enabled");
                }
            } else {
                tracing::warn!("Human-In-The-Loop: DISABLED");
            }
            mcp_server::server::run_uacs_headless(&host, port, config, hitl)
                .await
                .unwrap();
        }
        None => {
            let config = HeadlessConfig::default();
            let hitl = HITLConfig::default();
            tracing::info!("Starting Universal Agent Control System in Headless Mode (default)");
            tracing::info!("Human-In-The-Loop: ENABLED (default categories)");
            mcp_server::server::run_uacs_headless("127.0.0.1", 11425, config, hitl)
                .await
                .unwrap();
        }
    }
}
