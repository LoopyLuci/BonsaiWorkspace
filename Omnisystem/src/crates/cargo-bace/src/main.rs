use anyhow::Result;
use bace_rustc::BaceRustc;
use compile_cache::CompilationCache;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo-bace <build|run|optimize>");
        return Ok(());
    }

    let command = &args[1];
    let compiler = BaceRustc::new();
    let cache = CompilationCache::new(true);

    match command.as_str() {
        "build" => {
            let bcos = compiler.compile_crate("src/main.rs").await?;
            for bco in &bcos {
                cache.put(bco).await?;
                println!("Compiled: {} (hash: {})", bco.function_name, hex::encode(bco.function_hash.0));
            }
        }
        "run" => {
            println!("🚀 Running with hot-reload enabled...");
        }
        _ => println!("Unknown command: {}", command),
    }

    Ok(())
}
