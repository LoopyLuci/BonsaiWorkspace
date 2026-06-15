//! UI rendering module - Main entry point for all UI components

pub mod menu_bar;
pub mod dashboard;
pub mod build_graph;
pub mod timeline;
pub mod diagnostics;
pub mod status_bar;

use eframe::egui;
use crate::app::{UCCApp, ViewMode};

/// Render complete UI - called once per frame
pub fn render_menu_bar(app: &mut UCCApp, ctx: &egui::Context) {
    // Delegate to menu_bar module
    menu_bar::render(app, ctx);
}

/// Render main content area with view switching
pub fn render_main_content(app: &mut UCCApp, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        // Tab selector
        ui.horizontal(|ui| {
            if ui.selectable_label(app.current_view == ViewMode::Dashboard, "📊 Dashboard").clicked() {
                app.current_view = ViewMode::Dashboard;
            }
            if ui.selectable_label(app.current_view == ViewMode::BuildGraph, "🌳 Build Graph").clicked() {
                app.current_view = ViewMode::BuildGraph;
            }
            if ui.selectable_label(app.current_view == ViewMode::Timeline, "⏱️ Timeline").clicked() {
                app.current_view = ViewMode::Timeline;
            }
            if ui.selectable_label(app.current_view == ViewMode::Diagnostics, "🔍 Diagnostics").clicked() {
                app.current_view = ViewMode::Diagnostics;
            }
        });

        ui.separator();

        // Render selected view
        match app.current_view {
            ViewMode::Dashboard => dashboard::render(app, ui),
            ViewMode::BuildGraph => build_graph::render(app, ui),
            ViewMode::Timeline => timeline::render(app, ui),
            ViewMode::Diagnostics => diagnostics::render(app, ui),
            ViewMode::Settings => {} // Handled by modal
        }
    });
}

/// Render status bar - called once per frame
pub fn render_status_bar(app: &UCCApp, ctx: &egui::Context) {
    // Delegate to status_bar module
    status_bar::render(app, ctx);
}

