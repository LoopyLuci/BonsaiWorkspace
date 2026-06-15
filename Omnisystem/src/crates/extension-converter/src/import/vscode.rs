//! VSCode (.vsix) → Bonsai Extension IR importer.
//!
//! Parses a `.vsix` file (ZIP containing `package.json` + sources) and
//! maps its `contributes` section to the Unified Extension IR.

use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use tracing::{info, warn};

use crate::ir::{
    Capability, CapabilitySummary, CodeInfo, CodeLanguage, CommandCapability,
    ConversionNote, ConversionTier, ExtensionIr, ExtensionMetadata,
    ExtensionPermissions, KeybindingCapability, LanguageSupportCapability,
    PermissionScope, SnippetCapability, SourceFormat, ThemeCapability,
    ThemeKind, ViewCapability, ViewLocation,
};
use crate::ConversionError;

// ── VSCode package.json schema ────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct VscePackage {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    publisher: Option<String>,
    author: Option<serde_json::Value>,
    license: Option<String>,
    repository: Option<serde_json::Value>,
    icon: Option<String>,
    keywords: Option<Vec<String>>,
    main: Option<String>,
    contributes: Option<VsceContributes>,
    activation_events: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct VsceContributes {
    commands: Option<Vec<VsceCommand>>,
    languages: Option<Vec<VsceLanguage>>,
    grammars: Option<Vec<serde_json::Value>>,
    themes: Option<Vec<VsceTheme>>,
    icon_themes: Option<Vec<VsceTheme>>,
    snippets: Option<Vec<VsceSnippetFile>>,
    keybindings: Option<Vec<VsceKeybinding>>,
    views: Option<serde_json::Value>,
    views_containers: Option<serde_json::Value>,
    menus: Option<serde_json::Value>,
    configuration: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct VsceCommand {
    command: String,
    title: Option<String>,
    category: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VsceLanguage {
    id: String,
    extensions: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct VsceTheme {
    label: Option<String>,
    #[serde(rename = "uiTheme")]
    ui_theme: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VsceSnippetFile {
    language: Option<String>,
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VsceKeybinding {
    key: Option<String>,
    command: Option<String>,
    when: Option<String>,
}

// ── Main import function ──────────────────────────────────────────────────────

/// Parse a `.vsix` file into an `ExtensionIr`.
/// `vsix_path` must point to a valid `.vsix` (ZIP) archive.
pub async fn import_vsix(vsix_path: &Path) -> Result<ExtensionIr, ConversionError> {
    info!(path = %vsix_path.display(), "[vscode-import] parsing VSIX");

    // Extract to temp dir
    let tmp = tempfile::tempdir().map_err(|e| ConversionError::Io(e.to_string()))?;
    extract_zip(vsix_path, tmp.path())?;

    // Locate package.json (may be inside `extension/` subdirectory)
    let pkg_path = find_package_json(tmp.path())
        .ok_or_else(|| ConversionError::ManifestNotFound("package.json".into()))?;

    let pkg_json = std::fs::read_to_string(&pkg_path)
        .map_err(|e| ConversionError::Io(e.to_string()))?;
    let pkg: VscePackage = serde_json::from_str(&pkg_json)
        .map_err(|e| ConversionError::ParseError(format!("package.json: {e}")))?;

    let mut ir = ExtensionIr::default();
    ir.metadata = build_metadata(&pkg);

    let mut notes: Vec<ConversionNote> = Vec::new();

    if let Some(contrib) = &pkg.contributes {
        // Commands
        if let Some(cmds) = &contrib.commands {
            for cmd in cmds {
                ir.capabilities.push(Capability::Command(CommandCapability {
                    id: cmd.command.clone(),
                    title: cmd.title.clone().unwrap_or_else(|| cmd.command.clone()),
                    category: cmd.category.clone(),
                    keybinding: None,
                    when: None,
                    handler_ref: pkg.main.clone(),
                }));
            }
        }

        // Languages
        if let Some(langs) = &contrib.languages {
            for lang in langs {
                ir.capabilities.push(Capability::LanguageSupport(LanguageSupportCapability {
                    language_id: lang.id.clone(),
                    file_extensions: lang.extensions.clone().unwrap_or_default(),
                    ..Default::default()
                }));
            }
        }

        // Themes
        if let Some(themes) = &contrib.themes {
            for theme in themes {
                ir.capabilities.push(Capability::Theme(ThemeCapability {
                    name: theme.label.clone().unwrap_or_else(|| "Theme".into()),
                    kind: ThemeKind::Color,
                    colors: Default::default(),
                }));
            }
        }
        if let Some(icon_themes) = &contrib.icon_themes {
            for theme in icon_themes {
                ir.capabilities.push(Capability::Theme(ThemeCapability {
                    name: theme.label.clone().unwrap_or_else(|| "Icon Theme".into()),
                    kind: ThemeKind::Icon,
                    colors: Default::default(),
                }));
            }
        }

        // Snippet files (we declare without parsing the JSON files)
        if let Some(snippet_files) = &contrib.snippets {
            for sf in snippet_files {
                ir.capabilities.push(Capability::Snippet(SnippetCapability {
                    language_id: sf.language.clone().unwrap_or_default(),
                    prefix: String::new(),
                    body: vec!["// snippets from source extension".into()],
                    description: sf.path.clone(),
                }));
            }
        }

        // Keybindings
        if let Some(kbs) = &contrib.keybindings {
            for kb in kbs {
                if let (Some(key), Some(cmd)) = (&kb.key, &kb.command) {
                    ir.capabilities.push(Capability::Keybinding(KeybindingCapability {
                        key: key.clone(),
                        command: cmd.clone(),
                        when: kb.when.clone(),
                    }));
                }
            }
        }

        // Views — stored as custom for now; full Svelte conversion needs code analysis
        if contrib.views.is_some() {
            ir.capabilities.push(Capability::Custom {
                namespace: "vscode.views".into(),
                data: contrib.views.clone().unwrap_or_default(),
            });
            notes.push(ConversionNote {
                capability_id: "vscode.views".into(),
                tier: ConversionTier::AiAssisted,
                message: "View trees require AI-assisted conversion to Svelte panels.".into(),
                warning: true,
            });
        }

        // Menus / configuration — stored as custom
        if contrib.menus.is_some() {
            ir.capabilities.push(Capability::Custom {
                namespace: "vscode.menus".into(),
                data: contrib.menus.clone().unwrap_or_default(),
            });
        }
        if contrib.configuration.is_some() {
            ir.capabilities.push(Capability::Custom {
                namespace: "vscode.configuration".into(),
                data: contrib.configuration.clone().unwrap_or_default(),
            });
        }
    }

    // Infer permissions from activation events
    ir.permissions = infer_permissions(&pkg);

    // Code info
    ir.code = CodeInfo {
        language: CodeLanguage::TypeScript,
        main_entry: pkg.main.clone(),
        source_files: Vec::new(),
        wasm_blob: None,
    };

    // Determine conversion tier for the code entry point
    if pkg.main.is_some() {
        notes.push(ConversionNote {
            capability_id: "main".into(),
            tier: ConversionTier::ShimBased,
            message: "TypeScript entry point will be compiled against the bonsai-vscode-shim.".into(),
            warning: false,
        });
    }

    ir.conversion_notes = notes;

    info!(
        id = %ir.metadata.id,
        caps = ir.capabilities.len(),
        "[vscode-import] parsed successfully"
    );

    Ok(ir)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn build_metadata(pkg: &VscePackage) -> ExtensionMetadata {
    let name = pkg.name.clone().unwrap_or_else(|| "unknown".into());
    let publisher = pkg.publisher.clone().unwrap_or_else(|| "unknown".into());
    let id = format!("{publisher}.{name}");

    let author = match &pkg.author {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Object(o)) => o
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&publisher)
            .to_string(),
        _ => publisher.clone(),
    };

    let repository = match &pkg.repository {
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        Some(serde_json::Value::Object(o)) => o
            .get("url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        _ => None,
    };

    ExtensionMetadata {
        id,
        name,
        version: pkg.version.clone().unwrap_or_else(|| "0.0.0".into()),
        description: pkg.description.clone().unwrap_or_default(),
        author,
        license: pkg.license.clone().unwrap_or_else(|| "UNKNOWN".into()),
        repository,
        icon: pkg.icon.clone(),
        tags: pkg.keywords.clone().unwrap_or_default(),
        source_format: SourceFormat::VsCode,
    }
}

fn infer_permissions(pkg: &VscePackage) -> ExtensionPermissions {
    let has_workspace = pkg
        .activation_events
        .as_deref()
        .unwrap_or_default()
        .iter()
        .any(|e| e.contains("workspace") || e == "*");

    ExtensionPermissions {
        file_access: if has_workspace { PermissionScope::ReadWrite } else { PermissionScope::None },
        network: PermissionScope::Whitelist,
        process_spawn: PermissionScope::None,
        network_hosts: Vec::new(),
    }
}

fn find_package_json(dir: &Path) -> Option<PathBuf> {
    // Try root first, then extension/ subdirectory (VSIX standard)
    let root_pkg = dir.join("package.json");
    if root_pkg.exists() {
        return Some(root_pkg);
    }
    let ext_pkg = dir.join("extension").join("package.json");
    if ext_pkg.exists() {
        return Some(ext_pkg);
    }
    // Walk for it
    for entry in walkdir::WalkDir::new(dir).max_depth(3) {
        if let Ok(e) = entry {
            if e.file_name() == "package.json" {
                return Some(e.path().to_path_buf());
            }
        }
    }
    None
}

fn extract_zip(zip_path: &Path, out_dir: &Path) -> Result<(), ConversionError> {
    let file = std::fs::File::open(zip_path)
        .map_err(|e| ConversionError::Io(e.to_string()))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| ConversionError::ParseError(e.to_string()))?;
    archive
        .extract(out_dir)
        .map_err(|e| ConversionError::Io(e.to_string()))
}
