//! extension-converter: converts IDE extensions (VSCode, JetBrains, ...) into
//! a platform-agnostic Extension IR, and exports that IR into other targets
//! (currently: standalone MCP servers).

pub mod error;
pub mod ir;
pub mod import;
pub mod export;

pub use error::ConversionError;
pub use ir::{
    Capability, CapabilitySummary, CodeInfo, CodeLanguage, CommandCapability,
    ConversionNote, ConversionTier, ExtensionIr, ExtensionMetadata,
    ExtensionPermissions, KeybindingCapability, LanguageSupportCapability,
    PermissionScope, SnippetCapability, SourceFormat, ThemeCapability,
    ThemeKind, ToolCapability, ViewCapability, ViewLocation,
};
