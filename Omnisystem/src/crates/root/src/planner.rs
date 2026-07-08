use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

use crate::manifest::Component;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallPlan {
    pub component_ids: Vec<String>,
    pub operations: Vec<String>,
    pub total_download_mb: u32,
    pub total_disk_mb: u32,
}

pub fn build_install_plan(requested_ids: &[String], components: &[Component]) -> Result<InstallPlan> {
    let index: HashMap<&str, &Component> = components.iter().map(|c| (c.id.as_str(), c)).collect();
    let mut ordered = Vec::new();
    let mut seen = BTreeSet::new();

    for id in requested_ids {
        resolve_dependencies(id, &index, &mut seen, &mut ordered)?;
    }

    let mut operations = Vec::new();
    let mut total_download_mb = 0u32;
    let mut total_disk_mb = 0u32;

    operations.push("Verify signed manifest".to_string());

    for id in &ordered {
        let comp = *index
            .get(id.as_str())
            .ok_or_else(|| anyhow!("component missing in index: {id}"))?;

        operations.push(format!(
            "Download {} v{} ({} MB)",
            comp.name, comp.version, comp.size_mb
        ));
        operations.push(format!("Verify SHA256 for {}", comp.name));
        operations.push(format!("Extract {} into Bonsai-Ecosystem/{}", comp.name, comp.id));
        operations.push(format!("Register component metadata for {}", comp.name));

        total_download_mb = total_download_mb.saturating_add(comp.size_mb);
        total_disk_mb = total_disk_mb.saturating_add(comp.size_mb.saturating_add(comp.size_mb / 3));
    }

    operations.push("Write transaction journal + rollback snapshot".to_string());
    operations.push("Commit atomic swap".to_string());

    Ok(InstallPlan {
        component_ids: ordered,
        operations,
        total_download_mb,
        total_disk_mb,
    })
}

fn resolve_dependencies(
    id: &str,
    index: &HashMap<&str, &Component>,
    seen: &mut BTreeSet<String>,
    ordered: &mut Vec<String>,
) -> Result<()> {
    if seen.contains(id) {
        return Ok(());
    }

    let comp = *index
        .get(id)
        .ok_or_else(|| anyhow!("requested component not found: {id}"))?;

    for dep in &comp.dependencies {
        resolve_dependencies(dep, index, seen, ordered)?;
    }

    if seen.insert(id.to_string()) {
        ordered.push(id.to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_component(id: &str, deps: Vec<&str>, size_mb: u32) -> Component {
        Component {
            id: id.to_string(),
            name: format!("{} name", id),
            description: format!("{} description", id),
            version: "1.0.0".to_string(),
            size_mb,
            download_url: format!("https://example.com/{}.zip", id),
            hash: "abc".to_string(),
            dependencies: deps.into_iter().map(|v| v.to_string()).collect(),
            launch_cmd: None,
            recommended: true,
            tags: vec![],
            risk_level: "low".to_string(),
        }
    }

    #[test]
    fn includes_dependencies_before_component() {
        let components = vec![
            mk_component("core", vec![], 10),
            mk_component("bridge", vec!["core"], 20),
        ];
        let requested = vec!["bridge".to_string()];

        let plan = build_install_plan(&requested, &components).expect("plan should build");
        assert_eq!(plan.component_ids, vec!["core".to_string(), "bridge".to_string()]);
        assert!(plan.total_download_mb >= 30);
    }
}
