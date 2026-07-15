//! Watchdog CLI - inspects the survival knowledge base without running the
//! full launch-supervisor daemon (see `src/main.rs` for that).

use watchdog::KnowledgeBase;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::var("BONSAI_KB_PATH").unwrap_or_else(|_| {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".bonsai/survival_kb.db")
            .to_string_lossy()
            .to_string()
    });

    let kb = KnowledgeBase::open(&path)?;
    kb.seed_defaults()?;

    println!("knowledge base: {}", path);
    println!("total fixes: {}", kb.total_count());
    println!("by category:");
    for (category, count) in kb.category_counts() {
        println!("  {:<10} {}", category, count);
    }

    Ok(())
}
