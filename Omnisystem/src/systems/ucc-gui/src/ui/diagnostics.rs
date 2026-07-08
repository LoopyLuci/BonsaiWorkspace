//! Diagnostics view - Error analysis, filtering, and detailed diagnostics

use eframe::egui;
use crate::app::UCCApp;

/// Diagnostic filter options
#[derive(Clone, Copy, PartialEq)]
pub enum DiagnosticFilter {
    All,
    ErrorsOnly,
    WarningsOnly,
    SuccessOnly,
}

/// Render comprehensive diagnostics panel
pub fn render(app: &UCCApp, ui: &mut egui::Ui) {
    ui.heading("🔍 Diagnostics & Error Analysis");

    if app.last_build.is_none() {
        ui.label("No build results yet. Run a build to see diagnostics.");
        return;
    }

    let result = app.last_build.as_ref().unwrap();

    // Summary section
    ui.heading("Build Summary");
    ui.group(|ui| {
        ui.columns(4, |cols| {
            cols[0].vertical(|ui| {
                ui.label("Status:");
                let status = if result.success { "✅ Success" } else { "❌ Failed" };
                ui.heading(status);
            });

            cols[1].vertical(|ui| {
                ui.label("Errors:");
                ui.heading(result.errors.to_string());
            });

            cols[2].vertical(|ui| {
                ui.label("Warnings:");
                ui.heading(result.warnings.to_string());
            });

            cols[3].vertical(|ui| {
                ui.label("Duration:");
                ui.heading(format!("{}ms", result.duration_ms));
            });
        });
    });

    ui.separator();

    // Filter controls
    ui.heading("Filters");
    ui.horizontal(|ui| {
        if ui.button("All Messages").clicked() {
            // Filter all
        }
        if ui.button("❌ Errors Only").clicked() {
            // Filter errors
        }
        if ui.button("⚠️ Warnings Only").clicked() {
            // Filter warnings
        }
        if ui.button("📋 Search...").clicked() {
            // Open search dialog
        }
    });

    ui.separator();

    // Error details section
    if result.errors > 0 {
        ui.heading(format!("❌ Errors ({})", result.errors));
        ui.group(|ui| {
            ui.vertical(|ui| {
                // Parse errors from output
                let errors = parse_errors(&result.output);
                for (i, error) in errors.iter().enumerate() {
                    render_error_item(ui, i + 1, error);
                }
            });
        });
        ui.separator();
    }

    // Warning details section
    if result.warnings > 0 {
        ui.heading(format!("⚠️ Warnings ({})", result.warnings));
        ui.group(|ui| {
            ui.vertical(|ui| {
                // Parse warnings from output
                let warnings = parse_warnings(&result.output);
                for (i, warning) in warnings.iter().enumerate() {
                    render_warning_item(ui, i + 1, warning);
                }
            });
        });
        ui.separator();
    }

    // Full output section
    ui.heading("Compilation Output");
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.monospace(&result.output);
        });

    ui.separator();

    // Performance metrics section
    ui.heading("Performance Analysis");
    ui.horizontal(|ui| {
        ui.label("Compilation time:");
        ui.label(format!("{}ms", result.duration_ms));
        ui.separator();
        ui.label("Status:");
        if result.success {
            ui.colored_label(egui::Color32::GREEN, "✅ Successful");
        } else {
            ui.colored_label(egui::Color32::RED, "❌ Failed");
        }
    });
}

/// Render individual error item with details
fn render_error_item(ui: &mut egui::Ui, index: usize, error: &str) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::RED, format!("Error {}:", index));
            ui.label(error);
        });
    });
}

/// Render individual warning item with details
fn render_warning_item(ui: &mut egui::Ui, index: usize, warning: &str) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::YELLOW, format!("Warning {}:", index));
            ui.label(warning);
        });
    });
}

/// Parse errors from compilation output
fn parse_errors(output: &str) -> Vec<String> {
    output
        .lines()
        .filter(|line| line.contains("error") || line.contains("Error"))
        .map(|line| line.to_string())
        .collect()
}

/// Parse warnings from compilation output
fn parse_warnings(output: &str) -> Vec<String> {
    output
        .lines()
        .filter(|line| line.contains("warning") || line.contains("Warning"))
        .map(|line| line.to_string())
        .collect()
}
