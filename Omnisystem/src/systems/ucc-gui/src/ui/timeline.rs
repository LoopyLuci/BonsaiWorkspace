//! Compilation timeline - Gantt chart and scheduling visualization

use eframe::egui;
use crate::app::UCCApp;

/// Task for Gantt chart visualization
#[derive(Clone, Debug)]
struct GanttTask {
    name: String,
    start_ms: f32,
    duration_ms: f32,
    core: usize,
}

/// Render Gantt chart and compilation timeline
pub fn render(app: &UCCApp, ui: &mut egui::Ui) {
    ui.heading("⏱️ Compilation Timeline");

    if app.project_path.is_none() {
        ui.label("No project loaded. Build a project to see the compilation timeline.");
        return;
    }

    ui.separator();

    // Timeline controls
    ui.horizontal(|ui| {
        ui.label("Timeline View:");
        if ui.button("⬅️ Zoom Out").clicked() {
            // Zoom out functionality
        }
        if ui.button("➡️ Zoom In").clicked() {
            // Zoom in functionality
        }
        if ui.button("🔄 Reset View").clicked() {
            // Reset view
        }
    });

    ui.separator();

    // Sample Gantt data
    let sequential_time = 1300.0;
    let parallel_cores = 4;
    let parallel_time = 650.0;
    let speedup = sequential_time / parallel_time;

    // Create Gantt chart
    let tasks = vec![
        GanttTask {
            name: "core".to_string(),
            start_ms: 0.0,
            duration_ms: 500.0,
            core: 0,
        },
        GanttTask {
            name: "lib".to_string(),
            start_ms: 500.0,
            duration_ms: 450.0,
            core: 1,
        },
        GanttTask {
            name: "main".to_string(),
            start_ms: 650.0,
            duration_ms: 350.0,
            core: 2,
        },
    ];

    // Render Gantt chart
    ui.heading("Gantt Schedule (Parallel Execution)");
    render_gantt_chart(ui, &tasks, parallel_time);

    ui.separator();

    // Performance comparison
    ui.heading("Performance Metrics");
    ui.group(|ui| {
        ui.vertical(|ui| {
            ui.label("Sequential Execution:");
            ui.horizontal(|ui| {
                ui.label("Total time:");
                ui.label(format!("{}ms", sequential_time as u128));
            });

            ui.separator();

            ui.label(&format!("Parallel Execution ({} cores):", parallel_cores));
            ui.horizontal(|ui| {
                ui.label("Total time:");
                ui.label(format!("{}ms", parallel_time as u128));
            });

            ui.separator();

            ui.label("Performance Improvement:");
            ui.horizontal(|ui| {
                ui.label("Speedup:");
                ui.label(format!("{:.2}x", speedup));
                ui.separator();
                ui.label("Efficiency:");
                let efficiency = (speedup / parallel_cores as f32) * 100.0;
                ui.label(format!("{:.0}%", efficiency));
            });
        });
    });

    ui.separator();

    // Critical path highlighting
    ui.heading("Critical Path");
    ui.label("core (0-500ms) → lib (500-950ms) → main (950-1300ms)");
    ui.horizontal(|ui| {
        ui.add(egui::ProgressBar::new(500.0 / 1300.0).text("core"));
        ui.add(egui::ProgressBar::new(450.0 / 1300.0).text("lib"));
        ui.add(egui::ProgressBar::new(350.0 / 1300.0).text("main"));
    });

    ui.separator();

    // Parallel efficiency analysis
    ui.heading("Resource Utilization");
    ui.columns(parallel_cores, |cols| {
        for i in 0..parallel_cores {
            cols[i].vertical(|ui| {
                ui.label(format!("Core {}", i + 1));
                let usage = match i {
                    0 => 500.0 / parallel_time,
                    1 => 450.0 / parallel_time,
                    2 => 350.0 / parallel_time,
                    _ => 0.0,
                };
                ui.add(egui::ProgressBar::new(usage).show_percentage());
            });
        }
    });
}

/// Render Gantt chart visualization
fn render_gantt_chart(ui: &mut egui::Ui, tasks: &[GanttTask], total_time: f32) {
    let max_cores = tasks.iter().map(|t| t.core).max().unwrap_or(0) + 1;

    ui.group(|ui| {
        ui.vertical(|ui| {
            // Time axis
            ui.horizontal(|ui| {
                ui.label("Time:");
                for ms in (0..=total_time as i32).step_by(200) {
                    ui.label(format!("{}ms", ms));
                }
            });

            ui.separator();

            // Gantt bars
            for core in 0..max_cores {
                ui.horizontal(|ui| {
                    ui.label(format!("Core {}:", core + 1));

                    let core_tasks: Vec<_> = tasks.iter().filter(|t| t.core == core).collect();

                    for task in core_tasks {
                        let start_ratio = task.start_ms / total_time;
                        let width_ratio = task.duration_ms / total_time;

                        // Spacing before task
                        ui.horizontal(|ui| {
                            let available = ui.available_width();
                            let spacing = available * start_ratio;
                            ui.add_space(spacing);

                            // Task bar
                            let bar_width = available * width_ratio;
                            let (rect, _) = ui.allocate_exact_size(
                                egui::Vec2::new(bar_width.max(20.0), 20.0),
                                egui::Sense::hover(),
                            );

                            ui.painter().rect(
                                rect,
                                egui::Rounding::same(4.0),
                                egui::Color32::from_rgb(100, 150, 255),
                                egui::Stroke::new(1.0, egui::Color32::WHITE),
                            );

                            let task_label = format!("{}\n{}ms", &task.name, task.duration_ms as u128);
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                task_label,
                                egui::FontId::default(),
                                egui::Color32::WHITE,
                            );
                        });
                    }
                });
            }
        });
    });
}
