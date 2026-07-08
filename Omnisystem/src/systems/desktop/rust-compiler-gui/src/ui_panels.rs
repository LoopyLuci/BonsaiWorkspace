use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;
use crate::asset_system::AssetSystem;

pub struct UiState {
    pub selected_tab: usize,
    pub source_code: String,
    pub compiler_output: String,
    pub asset_expansion: HashMap<String, bool>,
    pub project_path: Option<PathBuf>,
    pub auto_compile_status: Option<String>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            selected_tab: 0,
            source_code: "// Write your Rust code here...\nfn main() {\n    println!(\"Hello, world!\");\n}".to_string(),
            compiler_output: String::new(),
            asset_expansion: HashMap::new(),
            project_path: None,
            auto_compile_status: None,
        }
    }
}

impl UiState {
    pub fn draw_menu_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("📁 Open Project").clicked() {
                // TODO: File dialog integration
            }
            if ui.button("🔨 Build").clicked() {
                // TODO: Trigger cargo build
            }
            ui.separator();
            if ui.button("⚙️ Settings").clicked() {
                // TODO: Settings modal
            }
        });
    }

    pub fn draw_status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Ready to compile...");
            ui.separator();
            ui.label("Assets: 0 processed");
        });
    }

    pub fn draw_source_editor(&mut self, ui: &mut egui::Ui) {
        ui.label("📝 Source Code Editor");
        ui.text_edit_multiline(&mut self.source_code);
    }

    pub fn draw_build_graph(&self, ui: &mut egui::Ui) {
        ui.label("📊 Build Dependency Graph");
        ui.label("(Interactive DAG visualization will display here)");
        ui.label("Click 🔨 Build to see compilation units");
    }

    pub fn draw_compiler_log(&mut self, ui: &mut egui::Ui) {
        ui.label("📋 Compiler Output");
        ui.text_edit_multiline(&mut self.compiler_output);
    }

    pub fn draw_timeline(&self, ui: &mut egui::Ui) {
        ui.label("⏱️ Compilation Timeline (Gantt Chart)");
        ui.label("(Real-time compilation timeline will display here)");
    }

    pub fn draw_asset_browser(&mut self, ui: &mut egui::Ui, _asset_system: &AssetSystem) {
        ui.label("🎨 Asset Browser");
        ui.label("Supported asset types:");
        ui.horizontal(|ui| {
            ui.label("🖼️ Textures");
            ui.label("📦 Models");
            ui.label("🔊 Audio");
            ui.label("✏️ Fonts");
            ui.label("🎨 Shaders");
        });
    }

    pub fn draw_diagnostics(&self, ui: &mut egui::Ui) {
        ui.label("🔍 Diagnostics");
        ui.label("(Errors and warnings will appear here after compilation)");
    }
}
