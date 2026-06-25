use eframe::egui;
use egui_dock::DockState;
use std::collections::HashMap;
use crate::{asset_system::AssetSystem, compiler_server::Diagnostic, CompilerApp};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    SourceEditor,
    BuildGraph,
    CompilerLog,
    Timeline,
    AssetBrowser,
    Diagnostics,
}

pub struct UiState {
    pub tree: DockState<Tab>,
    pub source_code: String,
    pub compiler_output: String,
    pub asset_expansion: HashMap<String, bool>,
    pub show_settings: bool,
}

impl Default for UiState {
    fn default() -> Self {
        let mut tree = DockState::new(vec![
            Tab::SourceEditor,
            Tab::BuildGraph,
            Tab::CompilerLog,
            Tab::Timeline,
        ]);

        Self {
            tree,
            source_code: String::new(),
            compiler_output: String::new(),
            asset_expansion: HashMap::new(),
            show_settings: false,
        }
    }
}

impl UiState {
    pub fn draw_menu_bar(&mut self, ui: &mut egui::Ui, app: &mut CompilerApp) {
        ui.horizontal(|ui| {
            if ui.button("📁 Open Project") {
                // TODO: File dialog
            }
            if ui.button("🔨 Build") {
                let compiler = app.compiler_server.clone();
                tokio::spawn(async move {
                    let _ = compiler.write().await.compile_debug().await;
                });
            }
            if ui.button("⚙️ Settings") {
                self.show_settings = !self.show_settings;
            }
            ui.separator();
            if ui.button("❌ Exit") {
                std::process::exit(0);
            }
        });
    }

    pub fn draw_status_bar(&self, ui: &mut egui::Ui, app: &CompilerApp) {
        ui.horizontal(|ui| {
            if let Some(build) = app.compiler_server.blocking_read().get_last_build() {
                let status = if build.success { "✅ Success" } else { "❌ Failed" };
                ui.label(format!("{} | {} errors | {} warnings | {}ms",
                    status, build.errors, build.warnings, build.duration_ms));
            } else {
                ui.label("Ready to compile...");
            }

            ui.separator();
            let asset_stats = app.asset_system.stats();
            ui.label(format!(
                "Assets: {}/{} | {:.1} MB",
                asset_stats.processed,
                asset_stats.total_assets,
                asset_stats.total_size_bytes as f32 / 1_000_000.0
            ));
        });
    }

    pub fn draw_source_editor(&mut self, ui: &mut egui::Ui) {
        ui.label("📝 Source Code Editor");
        ui.text_edit_multiline(&mut self.source_code);
    }

    pub fn draw_build_graph(&self, ui: &mut egui::Ui) {
        ui.label("📊 Build Dependency Graph");

        if let Ok(graph) = &*crate::CompilerApp { }.build_graph.try_read() {
            let stats = graph.stats();
            ui.horizontal(|ui| {
                ui.label(format!("Units: {}", stats.total_units));
                ui.label(format!("✅ {}", stats.completed));
                ui.label(format!("❌ {}", stats.failed));
                ui.label(format!("⚡ {:.2}x parallelization", stats.parallelization_factor));
            });

            // Draw DAG visualization (simplified)
            let mut y = ui.cursor().top;
            for unit in graph.units() {
                let color = match unit.state {
                    crate::build_graph::UnitState::Completed => egui::Color32::GREEN,
                    crate::build_graph::UnitState::InProgress => egui::Color32::YELLOW,
                    crate::build_graph::UnitState::Failed => egui::Color32::RED,
                    crate::build_graph::UnitState::CachedReused => egui::Color32::BLUE,
                    _ => egui::Color32::GRAY,
                };

                ui.allocate_space(egui::Vec2::new(ui.available_width(), 30.0));
                let rect = ui.painter().rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(ui.cursor().left, y),
                        egui::vec2(
                            (unit.duration_ms as f32 / 10.0).min(ui.available_width()),
                            20.0,
                        ),
                    ),
                    5.0,
                    color,
                );

                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &unit.name,
                    egui::FontId::default(),
                    egui::Color32::WHITE,
                );

                y += 30.0;
            }
        }
    }

    pub fn draw_compiler_log(&mut self, ui: &mut egui::Ui) {
        ui.label("📋 Compiler Output");
        ui.text_edit_multiline(&mut self.compiler_output);
    }

    pub fn draw_timeline(&self, ui: &mut egui::Ui) {
        ui.label("⏱️ Compilation Timeline (Gantt Chart)");

        // Timeline visualization
        let available_width = ui.available_width();
        let row_height = 25.0;

        if let Ok(graph) = &*crate::CompilerApp { }.build_graph.try_read() {
            let max_time = graph.stats().total_duration_ms.max(1);

            for unit in graph.units() {
                ui.horizontal(|ui| {
                    ui.label(&unit.name);
                    ui.separator();

                    let width = (unit.duration_ms as f32 / max_time as f32) * (available_width - 100.0);
                    let color = match unit.state {
                        crate::build_graph::UnitState::Completed => egui::Color32::GREEN,
                        crate::build_graph::UnitState::InProgress => egui::Color32::YELLOW,
                        _ => egui::Color32::GRAY,
                    };

                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(ui.cursor(), egui::vec2(width, row_height)),
                        3.0,
                        color,
                    );

                    ui.label(format!("{}ms", unit.duration_ms));
                });
            }
        }
    }

    pub fn draw_asset_browser(&mut self, ui: &mut egui::Ui, asset_system: &AssetSystem) {
        ui.label("🎨 Asset Browser");

        let assets = asset_system.list_assets();
        let stats = asset_system.stats();

        ui.horizontal(|ui| {
            ui.label(format!("Total: {} | Processed: {} | Failed: {}",
                stats.total_assets, stats.processed, stats.failed));
        });

        ui.separator();

        for asset in assets {
            let is_expanded = *self.asset_expansion.get(&asset.id).unwrap_or(&false);

            if ui.button(format!("{} {} ({})",
                match asset.asset_type {
                    crate::asset_system::AssetType::Texture => "🖼️",
                    crate::asset_system::AssetType::Model => "📦",
                    crate::asset_system::AssetType::Audio => "🔊",
                    crate::asset_system::AssetType::Video => "🎬",
                    crate::asset_system::AssetType::Font => "✏️",
                    crate::asset_system::AssetType::Shader => "🎨",
                    crate::asset_system::AssetType::Data => "📄",
                    crate::asset_system::AssetType::Other => "❓",
                },
                asset.path.file_name().unwrap_or_default().to_string_lossy(),
                asset.size_bytes / 1024
            )) {
                self.asset_expansion.insert(asset.id.clone(), !is_expanded);
            }

            if is_expanded {
                ui.indent(&asset.id, |ui| {
                    ui.label(format!("Hash: {}", &asset.content_hash[..8]));
                    ui.label(format!("State: {:?}", asset.state));
                    ui.label(format!("Processing: {}ms", asset.processing_duration_ms));
                    ui.label(format!("Compression: {:.2}x", asset.compression_ratio));
                });
            }
        }
    }

    pub fn draw_diagnostics(&self, ui: &mut egui::Ui) {
        ui.label("🔍 Diagnostics");

        if let Some(build) = crate::CompilerApp { }
            .compiler_server
            .blocking_read()
            .get_last_build()
        {
            for diagnostic in build.diagnostics {
                let color = match diagnostic.level {
                    crate::compiler_server::DiagnosticLevel::Error => egui::Color32::RED,
                    crate::compiler_server::DiagnosticLevel::Warning => egui::Color32::YELLOW,
                    crate::compiler_server::DiagnosticLevel::Note => egui::Color32::LIGHT_BLUE,
                    crate::compiler_server::DiagnosticLevel::Help => egui::Color32::GREEN,
                };

                ui.colored_label(color, format!("{:?}: {}", diagnostic.level, diagnostic.message));

                if let Some(file) = &diagnostic.file {
                    ui.label(format!(
                        "  at {}:{}:{}",
                        file.display(),
                        diagnostic.line.unwrap_or(0),
                        diagnostic.column.unwrap_or(0)
                    ));
                }
            }
        }
    }
}
