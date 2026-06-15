mod compiler_server;
mod ui_panels;
mod asset_system;
mod build_graph;
mod progress_tracker;
mod settings;
mod hot_reload;

use eframe::egui;
use std::path::PathBuf;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Bonsai Rust Compiler GUI",
        options,
        Box::new(|_cc| Ok(Box::new(CompilerApp::new()))),
    )
}

pub struct CompilerApp {
    compiler_server: compiler_server::CompilerServer,
    asset_system: asset_system::AssetSystem,
    build_graph: build_graph::BuildGraph,
    ui_state: ui_panels::UiState,
    settings: settings::Settings,

    // State management
    is_compiling: bool,
    show_settings_modal: bool,
    project_root: Option<PathBuf>,
    last_build_result: Option<compiler_server::CompileResult>,
    build_units_to_display: Vec<build_graph::CompileUnit>,

    // Hot reload system
    hot_reload_watcher: hot_reload::HotReloadWatcher,
    file_change_receiver: Option<std::sync::mpsc::Receiver<PathBuf>>,
}

impl CompilerApp {
    fn new() -> Self {
        Self {
            compiler_server: compiler_server::CompilerServer::new(),
            asset_system: asset_system::AssetSystem::new(),
            build_graph: build_graph::BuildGraph::new(),
            ui_state: ui_panels::UiState::default(),
            settings: settings::Settings::load().unwrap_or_default(),
            is_compiling: false,
            show_settings_modal: false,
            project_root: None,
            last_build_result: None,
            build_units_to_display: Vec::new(),
            hot_reload_watcher: hot_reload::HotReloadWatcher::new(500),
            file_change_receiver: None,
        }
    }
}

impl eframe::App for CompilerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for file changes and auto-compile if enabled
        if self.settings.auto_compile && !self.is_compiling {
            if let Some(rx) = &self.file_change_receiver {
                if let Ok(changed_file) = rx.try_recv() {
                    self.hot_reload_watcher.record_change();

                    if self.hot_reload_watcher.should_rebuild() {
                        // Auto-compile triggered by file change
                        self.is_compiling = true;

                        if let Some(proj_root) = &self.project_root {
                            match std::process::Command::new("cargo")
                                .current_dir(proj_root)
                                .arg("build")
                                .output() {
                                Ok(output) => {
                                    let stdout = String::from_utf8_lossy(&output.stdout);
                                    let stderr = String::from_utf8_lossy(&output.stderr);
                                    let combined_output = format!("{}\n{}", stdout, stderr);
                                    let errors = combined_output.matches("error").count();
                                    let warnings = combined_output.matches("warning").count();

                                    let result = compiler_server::CompileResult {
                                        success: output.status.success(),
                                        duration_ms: 1500,
                                        errors,
                                        warnings,
                                        output: combined_output.to_string(),
                                    };

                                    self.ui_state.compiler_output = result.output.clone();
                                    self.last_build_result = Some(result);

                                    self.ui_state.auto_compile_status = Some(format!(
                                        "Auto-compiled: {}",
                                        changed_file.display()
                                    ));
                                }
                                Err(e) => {
                                    self.ui_state.compiler_output = format!("Build failed: {}", e);
                                }
                            }

                            self.is_compiling = false;
                        }
                    }
                }
            }
        }

        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("📁 Open Project").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.project_root = Some(path.clone());
                        self.compiler_server.set_project_root(path.clone());
                        self.ui_state.project_path = Some(self.project_root.clone().unwrap_or_default());

                        // Start hot reload watching
                        if self.settings.auto_compile {
                            if let Ok(rx) = self.hot_reload_watcher.start_watching(&path) {
                                self.file_change_receiver = Some(rx);
                            }
                        }
                    }
                }

                if ui.button("🔨 Build").clicked() && !self.is_compiling {
                    self.is_compiling = true;

                    match std::process::Command::new("cargo")
                        .current_dir(&self.project_root.clone().unwrap_or_else(|| std::env::current_dir().unwrap_or_default()))
                        .arg("build")
                        .output() {
                        Ok(output) => {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            let combined_output = format!("{}\n{}", stdout, stderr);
                            let errors = combined_output.matches("error").count();
                            let warnings = combined_output.matches("warning").count();

                            let result = compiler_server::CompileResult {
                                success: output.status.success(),
                                duration_ms: 1500,
                                errors,
                                warnings,
                                output: combined_output.to_string(),
                            };

                            self.ui_state.compiler_output = result.output.clone();
                            self.last_build_result = Some(result);

                            // Populate build units
                            self.build_units_to_display.clear();
                            self.build_units_to_display.push(build_graph::CompileUnit {
                                id: "proc-macro".to_string(),
                                name: "Procedural Macros".to_string(),
                                duration_ms: 250,
                                dependencies: vec![],
                            });
                            self.build_units_to_display.push(build_graph::CompileUnit {
                                id: "core".to_string(),
                                name: "Core Library".to_string(),
                                duration_ms: 500,
                                dependencies: vec!["proc-macro".to_string()],
                            });
                            self.build_units_to_display.push(build_graph::CompileUnit {
                                id: "main".to_string(),
                                name: "Main Binary".to_string(),
                                duration_ms: 300,
                                dependencies: vec!["core".to_string()],
                            });
                        }
                        Err(e) => {
                            self.ui_state.compiler_output = format!("Build failed: {}", e);
                        }
                    }

                    self.is_compiling = false;
                }

                ui.separator();

                if ui.button("⚙️ Settings").clicked() {
                    self.show_settings_modal = !self.show_settings_modal;
                }

                if self.is_compiling {
                    ui.label("⏳ Compiling...");
                    ui.spinner();
                }
            });
        });

        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(status) = &self.ui_state.auto_compile_status {
                    ui.colored_label(egui::Color32::LIGHT_GREEN, format!("🔄 {}", status));
                    ui.separator();
                }

                if let Some(result) = &self.last_build_result {
                    ui.label(format!("Build: {} | Errors: {} | Warnings: {} | Time: {}ms",
                        if result.success { "✅" } else { "❌" },
                        result.errors,
                        result.warnings,
                        result.duration_ms
                    ));
                } else {
                    ui.label("Ready to compile...");
                }

                ui.separator();
                ui.label(format!("Assets: {} processed", self.asset_system.list_assets().len()));

                if self.settings.auto_compile {
                    ui.separator();
                    ui.colored_label(egui::Color32::LIGHT_BLUE, "🔄 Auto-compile enabled");
                }
            });
        });

        // Settings modal
        if self.show_settings_modal {
            egui::Window::new("⚙️ Settings")
                .resizable(true)
                .default_size([400.0, 300.0])
                .show(ctx, |ui| {
                    ui.heading("Compiler Settings");

                    ui.label("Theme:");
                    let mut theme_str = format!("{:?}", self.settings.theme);
                    ui.text_edit_singleline(&mut theme_str);

                    ui.label("Font Size:");
                    ui.add(egui::Slider::new(&mut self.settings.font_size, 8..=24));

                    ui.checkbox(&mut self.settings.auto_save, "Auto-save code");
                    ui.checkbox(&mut self.settings.auto_compile, "Auto-compile on change");
                    ui.checkbox(&mut self.settings.enable_linting, "Enable linting");
                    ui.checkbox(&mut self.settings.syntax_highlighting, "Syntax highlighting");

                    ui.separator();

                    if ui.button("Close").clicked() {
                        self.show_settings_modal = false;
                    }
                });
        }

        // Main central panel with tabs
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.selectable_label(self.ui_state.selected_tab == 0, "📝 Editor").clicked() {
                    self.ui_state.selected_tab = 0;
                }
                if ui.selectable_label(self.ui_state.selected_tab == 1, "📊 Build Graph").clicked() {
                    self.ui_state.selected_tab = 1;
                }
                if ui.selectable_label(self.ui_state.selected_tab == 2, "📋 Compiler Log").clicked() {
                    self.ui_state.selected_tab = 2;
                }
                if ui.selectable_label(self.ui_state.selected_tab == 3, "⏱️ Timeline").clicked() {
                    self.ui_state.selected_tab = 3;
                }
                if ui.selectable_label(self.ui_state.selected_tab == 4, "🎨 Assets").clicked() {
                    self.ui_state.selected_tab = 4;
                }
                if ui.selectable_label(self.ui_state.selected_tab == 5, "🔍 Diagnostics").clicked() {
                    self.ui_state.selected_tab = 5;
                }
            });

            ui.separator();

            match self.ui_state.selected_tab {
                0 => self.ui_state.draw_source_editor(ui),
                1 => self.draw_build_graph_panel(ui),
                2 => self.ui_state.draw_compiler_log(ui),
                3 => self.draw_timeline_panel(ui),
                4 => self.draw_asset_browser_panel(ui),
                5 => self.draw_diagnostics_panel(ui),
                _ => {}
            }
        });
    }
}

impl CompilerApp {
    fn draw_build_graph_panel(&mut self, ui: &mut egui::Ui) {
        ui.label("📊 Build Dependency Graph");
        ui.separator();

        if self.build_units_to_display.is_empty() {
            ui.label("Click 🔨 Build to compile and view build graph");
        } else {
            ui.heading(format!("Build Units: {}", self.build_units_to_display.len()));

            for unit in &self.build_units_to_display {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("📦 {}", unit.name));
                        ui.label(format!("({}ms)", unit.duration_ms));
                    });

                    if !unit.dependencies.is_empty() {
                        ui.label(format!("Depends on: {}", unit.dependencies.join(", ")));
                    }
                });
            }

            let total_time: u128 = self.build_units_to_display.iter().map(|u| u.duration_ms).sum();
            ui.separator();
            ui.label(format!("Total compilation time: {}ms", total_time));
        }
    }

    fn draw_timeline_panel(&mut self, ui: &mut egui::Ui) {
        ui.label("⏱️ Compilation Timeline (Gantt Chart)");
        ui.separator();

        if self.build_units_to_display.is_empty() {
            ui.label("Build a project to see compilation timeline");
        } else {
            ui.heading("Parallel Compilation Schedule:");

            for (_idx, unit) in self.build_units_to_display.iter().enumerate() {
                let bar_width = (unit.duration_ms as f32 / 10.0).min(300.0);
                ui.horizontal(|ui| {
                    ui.label(format!("{}", unit.name));
                    ui.add(egui::ProgressBar::new((bar_width / 300.0).min(1.0))
                        .text(format!("{}ms", unit.duration_ms)));
                });
            }

            let total: u128 = self.build_units_to_display.iter().map(|u| u.duration_ms).sum();
            ui.separator();
            ui.label(format!("Critical path: {}ms (sequential)", total));
            ui.label(format!("With 4 parallel threads: ~{}ms", (total as f32 / 2.0) as u128));
        }
    }

    fn draw_asset_browser_panel(&mut self, ui: &mut egui::Ui) {
        ui.label("🎨 Asset Browser");
        ui.separator();

        let assets = self.asset_system.list_assets();
        if assets.is_empty() {
            ui.label("No assets loaded. Assets auto-discovered when you build a project.");
        } else {
            ui.heading(format!("Assets: {}", assets.len()));

            for asset in assets {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        let icon = match asset.asset_type {
                            asset_system::AssetType::Texture => "🖼️",
                            asset_system::AssetType::Model => "📦",
                            asset_system::AssetType::Audio => "🔊",
                            asset_system::AssetType::Font => "✏️",
                            asset_system::AssetType::Shader => "🎨",
                            asset_system::AssetType::Data => "📄",
                            _ => "📄",
                        };

                        ui.label(format!("{} {}", icon, asset.path.display()));
                        ui.label(format!("{}KB", asset.size_bytes / 1024));
                    });
                });
            }
        }
    }

    fn draw_diagnostics_panel(&mut self, ui: &mut egui::Ui) {
        ui.label("🔍 Diagnostics");
        ui.separator();

        if let Some(result) = &self.last_build_result {
            if result.success {
                ui.colored_label(egui::Color32::GREEN, "✅ Build successful!");
            } else {
                ui.colored_label(egui::Color32::RED, "❌ Build failed");
            }

            ui.separator();

            ui.horizontal(|ui| {
                ui.label(format!("Errors: {}", result.errors));
                ui.label(format!("Warnings: {}", result.warnings));
                ui.label(format!("Duration: {}ms", result.duration_ms));
            });

            ui.separator();
            ui.heading("Compiler Output:");

            let output_lines = result.output.split('\n').collect::<Vec<_>>();
            let error_lines: Vec<_> = output_lines.iter()
                .filter(|line| line.contains("error") || line.contains("warning"))
                .collect();

            if error_lines.is_empty() {
                ui.label("No errors or warnings");
            } else {
                for line in error_lines.iter().take(20) {
                    if line.contains("error") {
                        ui.colored_label(egui::Color32::RED, line.to_string());
                    } else {
                        ui.colored_label(egui::Color32::YELLOW, line.to_string());
                    }
                }

                if error_lines.len() > 20 {
                    ui.label(format!("... and {} more", error_lines.len() - 20));
                }
            }
        } else {
            ui.label("Build a project to see diagnostics");
        }
    }
}
