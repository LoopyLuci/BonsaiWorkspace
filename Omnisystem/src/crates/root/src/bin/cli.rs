//! Bonsai Root CLI: builds a sample signed manifest, verifies it, plans an
//! install for a requested component set (resolving dependencies), and
//! walks the installer state machine through a full happy-path run.

use anyhow::Result;
use ed25519_dalek::{Signer, SigningKey};
use root::{build_install_plan, Component, InstallerMode, Manifest, StateMachine};

fn sample_manifest() -> Manifest {
    Manifest {
        version: 1,
        components: vec![
            Component {
                id: "core".to_string(),
                name: "Bonsai Core".to_string(),
                description: "Core runtime".to_string(),
                version: "1.0.0".to_string(),
                size_mb: 50,
                download_url: "https://example.com/core.zip".to_string(),
                hash: "0".repeat(64),
                dependencies: vec![],
                launch_cmd: None,
                recommended: true,
                tags: vec!["core".to_string()],
                risk_level: "low".to_string(),
            },
            Component {
                id: "workspace".to_string(),
                name: "Bonsai Workspace".to_string(),
                description: "IDE".to_string(),
                version: "2.0.0".to_string(),
                size_mb: 250,
                download_url: "https://example.com/workspace.zip".to_string(),
                hash: "1".repeat(64),
                dependencies: vec!["core".to_string()],
                launch_cmd: Some("workspace.exe".to_string()),
                recommended: true,
                tags: vec!["ide".to_string()],
                risk_level: "low".to_string(),
            },
        ],
        launcher_version: "1.0.0".to_string(),
        signature: None,
        public_key_id: "cli-demo".to_string(),
    }
}

fn main() -> Result<()> {
    // 1. Sign and verify the manifest, exactly as the real launcher would.
    let signing_key = SigningKey::from_bytes(&[42u8; 32]);
    let verify_key = signing_key.verifying_key();

    let mut manifest = sample_manifest();
    let payload = manifest.signed_payload_bytes()?;
    let signature = signing_key.sign(&payload);
    manifest.signature = Some(hex::encode(signature.to_bytes()));

    manifest.verify(&verify_key.to_bytes())?;
    println!("Manifest signature verified ({} components)", manifest.components.len());

    // 2. Build an install plan for "workspace" -- its "core" dependency
    // should be pulled in and ordered first.
    let requested = vec!["workspace".to_string()];
    let plan = build_install_plan(&requested, &manifest.components)?;

    println!("\nInstall order: {}", plan.component_ids.join(" -> "));
    println!("Total download: {} MB, total disk: {} MB", plan.total_download_mb, plan.total_disk_mb);
    println!("\nOperations:");
    for op in &plan.operations {
        println!("  - {op}");
    }

    // 3. Walk the installer state machine through a full run.
    let mut sm = StateMachine::default();
    for mode in [
        InstallerMode::DetectInstall,
        InstallerMode::FetchManifest,
        InstallerMode::VerifyManifest,
        InstallerMode::Welcome,
        InstallerMode::SimpleInstall,
        InstallerMode::PlanExecute,
        InstallerMode::Verify,
        InstallerMode::Complete,
        InstallerMode::Launcher,
    ] {
        sm.transition(mode)?;
    }
    println!("\nInstaller reached: {:?}", sm.current_mode);

    Ok(())
}
