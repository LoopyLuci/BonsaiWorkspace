//! Unified Extension Intermediate Representation (Extension IR).
//!
//! A platform-agnostic description of what an extension *does*, independent
//! of VSCode / JetBrains / Visual Studio implementation details.
//! All converters parse into this IR, then emit the target format from it.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Metadata ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtensionMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub repository: Option<String>,
    pub icon: Option<String>,
    pub tags: Vec<String>,
    pub source_format: SourceFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceFormat {
    #[default]
    Unknown,
    Bonsai,
    VsCode,
    VisualStudio,
    JetBrains,
    Eclipse,
    Neovim,
    SublimeText,
    Mcp,
}

impl std::fmt::Display for SourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceFormat::Unknown      => write!(f, "Unknown"),
            SourceFormat::Bonsai       => write!(f, "Bonsai"),
            SourceFormat::VsCode       => write!(f, "VSCode"),
            SourceFormat::VisualStudio => write!(f, "Visual Studio"),
            SourceFormat::JetBrains    => write!(f, "JetBrains"),
            SourceFormat::Eclipse      => write!(f, "Eclipse"),
            SourceFormat::Neovim       => write!(f, "Neovim"),
            SourceFormat::SublimeText  => write!(f, "Sublime Text"),
            SourceFormat::Mcp          => write!(f, "MCP"),
        }
    }
}

// ── Capabilities ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Capability {
    /// A command the user or other code can invoke.
    Command(CommandCapability),
    /// Language features: completion, hover, diagnostics, formatting.
    LanguageSupport(LanguageSupportCapability),
    /// A UI view/panel contributed to the IDE.
    View(ViewCapability),
    /// An AI/LLM-callable tool.
    Tool(ToolCapability),
    /// A colour/icon theme.
    Theme(ThemeCapability),
    /// Code snippet.
    Snippet(SnippetCapability),
    /// Keyboard shortcut.
    Keybinding(KeybindingCapability),
    /// Anything that doesn't map to a first-class type.
    Custom { namespace: String, data: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandCapability {
    pub id: String,
    pub title: String,
    pub category: Option<String>,
    pub keybinding: Option<String>,
    pub when: Option<String>,
    pub handler_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LanguageSupportCapability {
    pub language_id: String,
    pub file_extensions: Vec<String>,
    pub has_completion: bool,
    pub has_hover: bool,
    pub has_definitions: bool,
    pub has_diagnostics: bool,
    pub has_formatting: bool,
    pub grammar_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewCapability {
    pub id: String,
    pub title: String,
    pub location: ViewLocation,
    pub when: Option<String>,
    pub html_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ViewLocation {
    #[default]
    Sidebar,
    Panel,
    Editor,
    Dialog,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCapability {
    pub id: String,
    pub title: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub handler_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeCapability {
    pub name: String,
    pub kind: ThemeKind,
    pub colors: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeKind {
    #[default]
    Color,
    Icon,
    ProductIcon,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetCapability {
    pub language_id: String,
    pub prefix: String,
    pub body: Vec<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingCapability {
    pub key: String,
    pub command: String,
    pub when: Option<String>,
}

// ── Permissions ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtensionPermissions {
    pub file_access: PermissionScope,
    pub network: PermissionScope,
    pub process_spawn: PermissionScope,
    pub network_hosts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionScope {
    #[default]
    None,
    ReadOnly,
    ReadWrite,
    Sandboxed,
    Whitelist,
    All,
}

// ── Code entry points ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodeInfo {
    pub language: CodeLanguage,
    pub main_entry: Option<String>,
    pub source_files: Vec<String>,
    pub wasm_blob: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CodeLanguage {
    #[default]
    Unknown,
    TypeScript,
    JavaScript,
    Rust,
    CSharp,
    Java,
    Kotlin,
    Python,
    Lua,
    Wasm,
    Sylva,
}

// ── Conversion annotation ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionNote {
    pub capability_id: String,
    pub tier: ConversionTier,
    pub message: String,
    pub warning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversionTier {
    /// Manifest-only, no code needed.
    ManifestOnly,
    /// Shim-based — standard VSCode API subset.
    ShimBased,
    /// AI-assisted rewrite required.
    AiAssisted,
    /// Not convertible; skipped.
    Skipped,
}

// ── Top-level Extension IR ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtensionIr {
    pub metadata: ExtensionMetadata,
    pub capabilities: Vec<Capability>,
    pub permissions: ExtensionPermissions,
    pub code: CodeInfo,
    pub conversion_notes: Vec<ConversionNote>,
}

impl ExtensionIr {
    /// Count capabilities by type tag.
    pub fn capability_summary(&self) -> CapabilitySummary {
        let mut s = CapabilitySummary::default();
        for cap in &self.capabilities {
            match cap {
                Capability::Command(_)         => s.commands += 1,
                Capability::LanguageSupport(_) => s.language_support += 1,
                Capability::View(_)            => s.views += 1,
                Capability::Tool(_)            => s.tools += 1,
                Capability::Theme(_)           => s.themes += 1,
                Capability::Snippet(_)         => s.snippets += 1,
                Capability::Keybinding(_)      => s.keybindings += 1,
                Capability::Custom { .. }      => s.custom += 1,
            }
        }
        s
    }

    /// Return true if any conversion note has a warning.
    pub fn has_warnings(&self) -> bool {
        self.conversion_notes.iter().any(|n| n.warning)
    }

    /// Notes that are warnings.
    pub fn warnings(&self) -> Vec<&ConversionNote> {
        self.conversion_notes.iter().filter(|n| n.warning).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapabilitySummary {
    pub commands: usize,
    pub language_support: usize,
    pub views: usize,
    pub tools: usize,
    pub themes: usize,
    pub snippets: usize,
    pub keybindings: usize,
    pub custom: usize,
}
