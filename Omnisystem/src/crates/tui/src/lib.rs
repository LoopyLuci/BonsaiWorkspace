//! bti - the Bonsai TUI library
//!
//! A ratatui-based terminal interface for the Bonsai Ecosystem daemon. The
//! interactive application (event loop, panels, widgets, daemon client) is
//! implemented here as a library so it can be exercised from both the
//! `tui` binary (`src/main.rs`) and from tests, without requiring a live
//! terminal or a running daemon.

#![allow(dead_code)]

pub mod app;
pub mod client;
pub mod core;
pub mod error;
pub mod mode;
pub mod panel;
pub mod panels;
pub mod theme;
pub mod types;
pub mod widgets;

pub use app::{App, CliArgs, PanelId};
pub use client::DaemonClient;
pub use core::Core;
pub use error::{Error, Result};
pub use mode::Mode;
pub use panel::{Panel, PanelMeta};
pub use theme::Theme;
pub use types::State;
