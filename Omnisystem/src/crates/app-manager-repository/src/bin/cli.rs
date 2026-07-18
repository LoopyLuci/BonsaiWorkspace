//! app-manager-repository CLI
//!
//! Small utility for working with the local package cache: compute a
//! package's checksum, verify it against an expected value, or list what's
//! currently cached.

use app_manager_repository::{LocalLoader, PackageValidator};
use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("hash") => {
            let path = args.get(2).ok_or("usage: cli hash <file>")?;
            let data = std::fs::read(path)?;
            println!("{}", PackageValidator::calculate_hash(&data));
        }
        Some("verify") => {
            let path = args.get(2).ok_or("usage: cli verify <file> <expected-sha256>")?;
            let expected = args.get(3).ok_or("usage: cli verify <file> <expected-sha256>")?;
            let data = std::fs::read(path)?;
            let ok = PackageValidator::validate_checksum(&data, expected)?;
            println!("{}", if ok { "OK" } else { "MISMATCH" });
            if !ok {
                std::process::exit(1);
            }
        }
        Some("list-cache") => {
            let cache_dir = args
                .get(2)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("./.cache"));
            let loader = LocalLoader::new(cache_dir);
            let cached = loader.list_cached().await?;
            for path in cached {
                println!("{}", path.display());
            }
        }
        _ => {
            println!("app-manager-repository CLI");
            println!("usage:");
            println!("  cli hash <file>");
            println!("  cli verify <file> <expected-sha256>");
            println!("  cli list-cache [cache-dir]");
        }
    }

    Ok(())
}
