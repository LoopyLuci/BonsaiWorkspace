//! CLI

use app_manager_core::{AppManifest, AppRegistry, PublisherId};

fn main() {
    let registry = AppRegistry::new();
    let manifest = AppManifest::new(
        "example-app".to_string(),
        semver::Version::new(1, 0, 0),
        PublisherId::new(),
    );
    let app_id = manifest.id.clone();
    registry
        .register(app_manager_core::RegisteredApp::new(manifest))
        .expect("register app");

    println!("Registered app: {}", app_id);
    println!("Total apps: {}", registry.count());
}
