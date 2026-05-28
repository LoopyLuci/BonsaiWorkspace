pub mod executor;
pub use executor::{execute_skill, SkillResult};

use std::path::Path;
use std::sync::Arc;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tracing::{info, warn};

use bonsai_skill_compiler::{load_compiled_skill, verify_skill_integrity};
use bonsai_tool_registry::ToolRegistry;

/// Watch `path` for new or modified `.json` skill files and hot-register them.
///
/// The returned `RecommendedWatcher` must be kept alive as long as watching
/// should continue — dropping it stops the watch.
pub fn watch_skills_dir(
    registry: Arc<ToolRegistry>,
    path: &Path,
) -> notify::Result<RecommendedWatcher> {
    let mut watcher =
        notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            let event = match res {
                Ok(e) => e,
                Err(e) => { warn!("skills watcher error: {e}"); return; }
            };
            let relevant = matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_)
            );
            if !relevant { return; }
            for p in &event.paths {
                if p.extension().map_or(false, |e| e == "json") {
                    reload_one(&registry, p);
                }
            }
        })?;
    watcher.watch(path, RecursiveMode::NonRecursive)?;
    Ok(watcher)
}

/// Load all existing `.json` skill files from `path` into `registry`.
pub fn load_initial(registry: &Arc<ToolRegistry>, path: &Path) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(path)? {
        let p = entry?.path();
        if p.extension().map_or(false, |e| e == "json") {
            reload_one(registry, &p);
        }
    }
    Ok(())
}

fn reload_one(registry: &Arc<ToolRegistry>, json_path: &Path) {
    // The file stem is the skill id with '/' replaced by "__".
    let stem = json_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.replace("__", "/"))
        .unwrap_or_default();

    match load_compiled_skill(&stem) {
        Ok(skill) if verify_skill_integrity(&skill) => {
            info!(skill = %skill.name, "skills: registered");
            registry.register(skill);
        }
        Ok(_) => warn!(id = %stem, "skills: integrity check failed — skipping"),
        Err(e) => warn!(id = %stem, error = %e, "skills: load failed"),
    }
}
