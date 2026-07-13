use app_registry::{AppRegistry, AppManifest, AppCategory, LaunchConfig, Dependencies, PlatformSupport, CapabilityRequirements, DocsConfig, SimpleConfig, DeveloperConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🌿 BonsaiWorkspace App Registry");
    println!("================================\n");

    let registry = AppRegistry::new();

    // Register workspace app
    let workspace_manifest = AppManifest {
        id: "ai.bonsai.workspace".to_string(),
        name: "Bonsai Workspace".to_string(),
        tagline: "Your AI-powered coding environment".to_string(),
        description: "Full IDE with local inference, code execution, and fine-tuning".to_string(),
        version: "0.2.0".to_string(),
        channel: "stable".to_string(),
        category: AppCategory::Developer,
        icon: Some("assets/workspace.svg".to_string()),
        screenshots: vec![],
        demo_video: None,
        launch: LaunchConfig {
            binary: "bonsai-workspace".to_string(),
            args: vec!["--mode".to_string(), "workspace".to_string()],
            modes: Some(vec!["workspace".to_string(), "buddy".to_string(), "ecosystem".to_string()]),
            deep_link: Some("bonsai://launch/workspace".to_string()),
            health_check: Some("http://localhost:11369/health".to_string()),
        },
        dependencies: Dependencies {
            required: vec![],
            optional: vec![],
        },
        platform: PlatformSupport {
            windows: true,
            macos: true,
            linux: true,
            android: false,
            ios: false,
        },
        capabilities: CapabilityRequirements {
            required: vec!["network.local".to_string(), "storage.user".to_string()],
            optional: vec!["gpu".to_string(), "microphone".to_string(), "camera".to_string()],
            requestable: vec!["microphone".to_string(), "camera".to_string()],
        },
        docs: DocsConfig {
            overview: "docs/reference/00-OVERVIEW.md".to_string(),
            quickstart: "docs/reference/01-GETTING-STARTED.md".to_string(),
            manual: "docs/reference/02-CORE-IDE.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            issues_url: Some("https://github.com/rechargedideas/BonsaiWorkspace/issues".to_string()),
            doc_version: Some("0.2.0".to_string()),
        },
        session_restore: Some(app_registry::SessionConfig {
            restore: true,
            snapshot_on_close: true,
        }),
        sync_profile: None,
        simple: SimpleConfig {
            what_it_does: "Write code, talk to AI, run programs — all on your computer, no internet needed.".to_string(),
            first_steps: vec![
                "Open a folder".to_string(),
                "Chat with Bonsai".to_string(),
                "Run your code".to_string(),
            ],
        },
        developer: DeveloperConfig {
            crate_name: Some("bonsai-workspace".to_string()),
            api_port: Some(11369),
            source_path: Some("BonsaiEcosystem/workspace".to_string()),
        },
    };

    registry.register(workspace_manifest)?;

    // List all registered apps
    let all_apps = registry.list(None);
    println!("Registered apps: {}\n", all_apps.len());

    for app in all_apps {
        println!("  ✓ {}", app.manifest.name);
        println!("    ID: {}", app.manifest.id);
        println!("    Version: {}", app.manifest.version);
        println!("    Category: {:?}", app.manifest.category);
        println!();
    }

    println!("✅ Registry initialized successfully");
    Ok(())
}
