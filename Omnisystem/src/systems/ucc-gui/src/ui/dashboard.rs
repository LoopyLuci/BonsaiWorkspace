//! Dashboard view - Key metrics, charts, and build history

use eframe::egui;
use crate::app::UCCApp;

/// Render main dashboard with metrics and charts
pub fn render(app: &UCCApp, ui: &mut egui::Ui) {
    ui.heading("📊 Build Dashboard");

    if app.project_path.is_none() {
        ui.label("No project loaded. Open a project to see build metrics.");
        return;
    }

    ui.separator();

    // Key metrics row
    render_metrics_row(app, ui);

    ui.separator();

    // Charts and visualizations
    ui.columns(2, |cols| {
        // Left column: Last build details
        cols[0].heading("Latest Build");
        if let Some(result) = &app.last_build {
            cols[0].group(|ui| {
                ui.vertical(|ui| {
                    let status = if result.success { "✅ Success" } else { "❌ Failed" };
                    ui.horizontal(|ui| {
                        ui.label("Status:");
                        ui.heading(status);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Time:");
                        ui.label(format!("{}ms", result.duration_ms));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Errors:");
                        ui.label(result.errors.to_string());
                        ui.separator();
                        ui.label("Warnings:");
                        ui.label(result.warnings.to_string());
                    });

                    ui.horizontal(|ui| {
                        ui.label("Timestamp:");
                        ui.small(result.timestamp.format("%Y-%m-%d %H:%M:%S").to_string());
                    });

                    ui.add(egui::ProgressBar::new(
                        if result.success { 1.0 } else { 0.3 },
                    ));
                });
            });
        } else {
            cols[0].group(|ui| {
                ui.label("📝 No builds yet. Click 🔨 Build to start.");
            });
        }

        // Right column: Detected languages
        cols[1].heading("Project Info");
        cols[1].group(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Project:");
                    if let Some(project) = &app.project_path {
                        ui.label(
                            project
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("Unknown"),
                        );
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Languages:");
                    ui.label(format!("({})", app.detected_languages.len()));
                });

                // Language list
                for lang in &app.detected_languages {
                    ui.label(format!("  • {}", lang));
                }
            });
        });
    });

    ui.separator();

    // Build history timeline
    ui.heading("Build History (Last 10)");
    render_build_history_table(app, ui);

    ui.separator();

    // Success trend
    ui.heading("Build Trends");
    render_success_trend(app, ui);

    ui.separator();

    // Quick actions
    ui.heading("Quick Actions");
    ui.horizontal(|ui| {
        if ui.button("🔨 Build Now").clicked() {
            // Trigger build
        }
        if ui.button("🧹 Clean Build").clicked() {
            // Trigger clean
        }
        if ui.button("📋 View Log").clicked() {
            // Open log viewer
        }
        if ui.button("🎯 Build Settings").clicked() {
            // Open settings
        }
    });
}

/// Render key metrics in a row layout
fn render_metrics_row(app: &UCCApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        // Total Builds
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label("Total Builds");
                ui.heading(app.metrics.total_builds.to_string());
            });
        });

        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label("Successful");
                ui.colored_label(
                    egui::Color32::GREEN,
                    app.metrics.successful_builds.to_string(),
                );
            });
        });

        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label("Failed");
                let failed = app.metrics.total_builds - app.metrics.successful_builds;
                ui.colored_label(egui::Color32::RED, failed.to_string());
            });
        });

        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label("Success Rate");
                let rate = if app.metrics.total_builds == 0 {
                    100.0
                } else {
                    (app.metrics.successful_builds as f32 / app.metrics.total_builds as f32) * 100.0
                };
                ui.heading(format!("{:.0}%", rate));
            });
        });

        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label("Avg Time");
                ui.heading(format!("{}ms", app.metrics.average_build_time_ms));
            });
        });

        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label("Cache Hit Rate");
                ui.heading(format!("{:.0}%", app.metrics.cache_hit_rate * 100.0));
            });
        });
    });
}


/// Render build history as a table
fn render_build_history_table(app: &UCCApp, ui: &mut egui::Ui) {
    egui::Grid::new("build_history_grid")
        .striped(true)
        .show(ui, |ui| {
            ui.label("Time");
            ui.label("Status");
            ui.label("Duration");
            ui.label("Errors");
            ui.label("Warnings");
            ui.end_row();

            for build in app.build_history.iter().rev().take(10) {
                let status = if build.success { "✅" } else { "❌" };
                ui.label(build.timestamp.format("%H:%M:%S").to_string());
                ui.label(status);
                ui.label(format!("{}ms", build.duration_ms));
                ui.colored_label(
                    if build.errors > 0 {
                        egui::Color32::RED
                    } else {
                        egui::Color32::GREEN
                    },
                    build.errors.to_string(),
                );
                ui.colored_label(
                    if build.warnings > 0 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::GREEN
                    },
                    build.warnings.to_string(),
                );
                ui.end_row();
            }
        });
}

/// Render success trend visualization
fn render_success_trend(app: &UCCApp, ui: &mut egui::Ui) {
    if app.build_history.is_empty() {
        ui.label("Build history needed for trends");
        return;
    }

    ui.horizontal(|ui| {
        ui.label("Recent builds:");

        // Show mini bars for last 5 builds
        for build in app.build_history.iter().rev().take(5) {
            let color = if build.success {
                egui::Color32::GREEN
            } else {
                egui::Color32::RED
            };
            let height = if build.success { 20.0 } else { 10.0 };

            ui.vertical(|ui| {
                ui.add_space(30.0 - height);
                let (rect, _) = ui.allocate_exact_size(
                    egui::Vec2::new(15.0, height),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(rect, egui::Rounding::same(2.0), color);
            });
        }
    });
}
