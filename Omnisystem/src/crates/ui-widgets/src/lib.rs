//! UI Widgets - a design-system data model for a UI component library
//!
//! - [`component`]: base widget component config (type/variant/size/aria)
//! - [`advanced_widgets`]: richer widgets (data table, chart, notification,
//!   rich text editor, file picker)
//! - [`theme`]: theme/color definitions plus a theme registry that renders
//!   CSS custom properties
//! - [`animation`]: easing functions and an animation engine (real
//!   quadratic ease-in/out math, not a lookup table)
//! - [`accessibility`]: WCAG compliance profiles, keyboard navigation, and
//!   screen-reader support metadata
//! - [`database`]: an in-memory catalog of widget definitions with
//!   search/filter/category lookup
//!
//! Companion Svelte components and a Tauri demo shell live alongside this
//! crate under `svelte/` and `tauri/` (not part of the Rust build).

pub mod accessibility;
pub mod advanced_widgets;
pub mod animation;
pub mod component;
pub mod database;
pub mod theme;

pub use accessibility::{AccessibilityProfile, KeyboardNavigation, ScreenReaderSupport, WCAG};
pub use advanced_widgets::{
    Chart, ChartData, ChartDataset, ChartType, ColumnDef, DataTable, FilePicker, FileSelection,
    Notification, NotificationType, RichTextEditor,
};
pub use animation::{Animation, AnimationDirection, AnimationEngine, Easing};
pub use component::{Component, ComponentSize, ComponentState, ComponentType, ComponentVariant};
pub use database::{PropDefinition, WidgetDatabase, WidgetEntry};
pub use theme::{Color, Theme, ThemeManager};
