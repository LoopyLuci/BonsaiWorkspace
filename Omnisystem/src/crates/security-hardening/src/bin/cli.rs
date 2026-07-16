//! CLI: encrypt/decrypt a secret at rest, scan text for leaked secrets,
//! generate an SBOM from this crate's own Cargo.toml, scan it for known
//! vulnerabilities, and verify the manifest as a supply-chain artifact.

use security_hardening::{
    EncryptionManager, KeyManager, SbomGenerator, SecretScanner, SupplyChainVerifier,
    VulnerabilityScanner,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Encryption at rest
    let key_manager = KeyManager::new();
    let key = key_manager.current_key()?;
    let encryption = EncryptionManager::new();
    let ciphertext = encryption.encrypt_at_rest(b"db-password-123", &key)?;
    let plaintext = encryption.decrypt_at_rest(&ciphertext, &key)?;
    println!(
        "encrypted {} bytes, round-trip matched: {}",
        ciphertext.len(),
        plaintext == b"db-password-123"
    );

    // Secret scanning
    let scanner = SecretScanner::new();
    let findings = scanner
        .scan_text("aws_key = \"AKIAIOSFODNN7EXAMPLE\"\nlet x = 1;")
        .await?;
    println!("secret scan found {} finding(s)", findings.len());

    // SBOM generation from this crate's own manifest
    let manifest_path = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let sbom_generator = SbomGenerator::new();
    let sbom = sbom_generator.generate(manifest_path).await?;
    println!("generated SBOM with {} component(s)", sbom.components.len());

    // Vulnerability scan against that SBOM
    let vuln_scanner = VulnerabilityScanner::new();
    let vulns = vuln_scanner.scan(&sbom).await?;
    println!("vulnerability scan found {} issue(s)", vulns.len());

    // Supply-chain verification of the manifest file itself
    let verifier = SupplyChainVerifier::new();
    let verified = verifier.verify_artifact(manifest_path).await?;
    println!("supply-chain artifact verified: {}", verified);

    Ok(())
}
