//! CLI for exercising the omnisystem-submodule crate: loads a demo module,
//! runs it through its full lifecycle, and checks version compatibility.

use async_trait::async_trait;
use omnisystem_submodule::{ModuleMetadata, ModuleState, ModuleVersion, Result, SubModule, SubModuleManager, VersionResolver};

struct EchoModule {
    state: ModuleState,
    metadata: ModuleMetadata,
}

#[async_trait]
impl SubModule for EchoModule {
    fn metadata(&self) -> &ModuleMetadata {
        &self.metadata
    }

    fn state(&self) -> ModuleState {
        self.state
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state = ModuleState::Initialized;
        Ok(())
    }

    async fn start(&mut self) -> Result<()> {
        self.state = ModuleState::Running;
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        self.state = ModuleState::Stopped;
        Ok(())
    }

    async fn unload(&mut self) -> Result<()> {
        self.state = ModuleState::Unloaded;
        Ok(())
    }

    async fn handle_message(&mut self, msg: &str) -> Result<String> {
        Ok(format!("echo: {msg}"))
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let manager = SubModuleManager::new();

    let module = EchoModule {
        state: ModuleState::Unloaded,
        metadata: ModuleMetadata {
            name: "echo".to_string(),
            version: ModuleVersion::new(1, 2, 0),
            author: "omnisystem".to_string(),
            description: "Echoes messages back".to_string(),
            dependencies: vec![],
            capabilities: vec!["echo".to_string()],
        },
    };

    manager.load_module("echo".to_string(), Box::new(module)).await?;
    println!("Loaded module 'echo', state: {:?}", manager.get_state("echo").await);

    manager.start_module("echo").await?;
    println!("Started module 'echo', state: {:?}", manager.get_state("echo").await);

    manager.stop_module("echo").await?;
    println!("Stopped module 'echo', state: {:?}", manager.get_state("echo").await);

    let required = ModuleVersion::new(1, 0, 0);
    let available = ModuleVersion::new(1, 2, 0);
    match VersionResolver::is_compatible(&required, &available) {
        Ok(()) => println!("Version {} satisfies required {}", available, required),
        Err(e) => println!("Version incompatible: {}", e),
    }

    println!("Total modules loaded: {}", manager.module_count());
    Ok(())
}
