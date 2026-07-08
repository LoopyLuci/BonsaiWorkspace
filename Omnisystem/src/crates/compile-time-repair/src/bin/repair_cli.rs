//! Repair CLI binary entry point

use compile_time_repair::cli::RepairCLI;
use std::env;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if let Err(e) = RepairCLI::run(&args).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
