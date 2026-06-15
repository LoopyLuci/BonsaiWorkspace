//! Build graph visualization - Interactive dependency graph with node status

use eframe::egui;
use crate::app::UCCApp;
use crate::models::UnitStatus;

/// Compilation unit node for visualization
#[derive(Clone, Debug)]
struct CompilationNode {
    name: String,
    duration_ms: u128,
    status: UnitStatus,
    dependencies: Vec<String>,
    language: String,
}

/// Render interactive build dependency graph
pub fn render(app: &UCCApp, ui: &mut egui::Ui) {
    ui.heading("🌳 Build Dependency Graph");

    if app.project_path.is_none() {
        ui.label("No project loaded. Open a project to see the dependency graph.");
        return;
    }

    ui.label("Compilation units and their dependencies:");
    ui.separator();

    // Sample compilation units (would be populated from actual build)
    let units = vec![
        CompilationNode {
            name: "core".to_string(),
            duration_ms: 500,
            status: UnitStatus::Success,
            dependencies: vec![],
            language: "Rust".to_string(),
        },
        CompilationNode {
            name: "lib".to_string(),
            duration_ms: 450,
            status: UnitStatus::Success,
            dependencies: vec!["core".to_string()],
            language: "Rust".to_string(),
        },
        CompilationNode {
            name: "main".to_string(),
            duration_ms: 350,
            status: UnitStatus::Success,
            dependencies: vec!["lib".to_string(), "core".to_string()],
            language: "Rust".to_string(),
        },
    ];

    // Render units in visual hierarchy
    ui.group(|ui| {
        ui.vertical(|ui| {
            render_dependency_tree(ui, &units);
        });
    });

    ui.separator();

    // Critical path analysis
    ui.heading("Critical Path Analysis");
    ui.horizontal(|ui| {
        ui.label("Longest dependency chain:");
        ui.label("core → lib → main");
        ui.separator();
        ui.label("Total time: 1300ms");
    });

    ui.separator();

    // Build statistics
    ui.heading("Compilation Statistics");
    ui.columns(4, |cols| {
        cols[0].vertical(|ui| {
            ui.label("Total Units:");
            ui.heading(units.len().to_string());
        });
        cols[1].vertical(|ui| {
            ui.label("Success:");
            ui.heading(units.iter().filter(|u| u.status == UnitStatus::Success).count().to_string());
        });
        cols[2].vertical(|ui| {
            ui.label("Failed:");
            ui.heading(units.iter().filter(|u| u.status == UnitStatus::Failed).count().to_string());
        });
        cols[3].vertical(|ui| {
            ui.label("Cached:");
            ui.heading(units.iter().filter(|u| u.status == UnitStatus::Cached).count().to_string());
        });
    });
}

/// Render tree-style dependency visualization
fn render_dependency_tree(ui: &mut egui::Ui, units: &[CompilationNode]) {
    for unit in units {
        render_unit_node(ui, unit);
    }
}

/// Render individual compilation unit node
fn render_unit_node(ui: &mut egui::Ui, unit: &CompilationNode) {
    ui.horizontal(|ui| {
        // Status icon
        ui.label(unit.status.icon());

        // Unit name and info
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(&unit.name);
                ui.separator();
                ui.small(&unit.language);
                ui.separator();
                ui.small(format!("{}ms", unit.duration_ms));
            });

            // Dependencies
            if !unit.dependencies.is_empty() {
                ui.small(format!("Dependencies: {}", unit.dependencies.join(", ")));
            }
        });

        // Status color background
        let response = ui.interact(
            ui.available_rect_before_wrap(),
            ui.auto_id_with("unit_node"),
            egui::Sense::click(),
        );

        if response.hovered() {
            ui.painter().rect(
                response.rect.expand(2.0),
                egui::Rounding::same(4.0),
                egui::Color32::DARK_GRAY,
                egui::Stroke::new(1.0, unit.status.color()),
            );
        }
    });

    // Render dependency edges as indented children
    if !unit.dependencies.is_empty() {
        ui.indent("deps", |ui| {
            for dep in &unit.dependencies {
                ui.label(format!("↳ {}", dep));
            }
        });
    }

    ui.separator();
}
