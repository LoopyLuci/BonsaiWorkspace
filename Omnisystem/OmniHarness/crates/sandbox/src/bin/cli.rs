//! CLI demo: initialize an Enclave project and lock its (empty) dependencies.

use sandbox::{Enclave, EnclaveConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = EnclaveConfig::new(std::env::current_dir()?)?;
    let mut enclave = Enclave::new(config).await?;

    let lockfile = enclave.lock().await?;
    println!("Locked {} package(s)", lockfile.packages.len());

    Ok(())
}
