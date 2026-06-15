//! Status bar - Real-time compilation status, metrics, and resource usage

use eframe::egui;
use crate::app::UCCApp;

/// Render status bar with compilation status and metrics
pub fn render(app: &UCCApp, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("status_bar")
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Left side: Build status
                if let Some(result) = &app.last_build {
                    let status_icon = if result.success { "✅" } else { "❌" };
                    let status_text = if result.success { "Success" } else { "Failed" };

                    ui.label(format!("{} {}", status_icon, status_text));

                    ui.separator();

                    ui.label(format!("Errors: {}", result.errors));
                    ui.separator();
                    ui.label(format!("Warnings: {}", result.warnings));
                    ui.separator();
                    ui.label(format!("Time: {}ms", result.duration_ms));
                } else {
                    ui.label("🟢 Ready to compile");
                }

                ui.separator();

                // Center: Build counter
                ui.label(format!("Builds: {}", app.build_history.len()));

                ui.separator();

                // Build progress indicator
                if app.is_building {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(egui::Spinner::new().size(16.0));
                        ui.label("Compiling...");
                    });
                } else {
                    // Right side: Project path
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(project) = &app.project_path {
                            let path_str = project
                                .file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or("Unknown");
                            ui.label(format!("📂 {}", path_str));
                        } else {
                            ui.label("📂 No project loaded");
                        }
                    });
                }
            });
        });
}

/// Render extended status information
pub fn render_extended(app: &UCCApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label("Cache Hit Rate:");
        ui.label(format!("{:.1}%", app.metrics.cache_hit_rate * 100.0));

        ui.separator();

        ui.label("Avg Build Time:");
        ui.label(format!("{}ms", app.metrics.average_build_time_ms));

        ui.separator();

        ui.label("Success Rate:");
        let rate = if app.metrics.total_builds == 0 {
            100.0
        } else {
            (app.metrics.successful_builds as f32 / app.metrics.total_builds as f32) * 100.0
        };
        ui.label(format!("{:.0}%", rate));
    });
}
