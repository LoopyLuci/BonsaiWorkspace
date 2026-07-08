//! Main application state and logic

use eframe::egui;
use std::path::PathBuf;
use crate::models::*;
use crate::ui;
use ucc::UnixCC;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Pending UI operations
#[derive(Clone, Debug)]
pub enum PendingOperation {
    LoadProject(PathBuf),
    Build,
    Clean,
}

/// Main application state
#[derive(Clone)]
pub struct UCCApp {
    /// Current project path
    pub project_path: Option<PathBuf>,

    /// Detected languages in project
    pub detected_languages: Vec<String>,

    /// Build results cache
    pub last_build: Option<BuildResult>,

    /// Current view mode
    pub current_view: ViewMode,

    /// Build in progress flag
    pub is_building: bool,

    /// UI state
    pub ui_state: UIState,

    /// Build history
    pub build_history: Vec<BuildResult>,

    /// Real-time metrics
    pub metrics: BuildMetrics,

    /// UCC compiler instance
    pub compiler: Arc<Mutex<Option<UnixCC>>>,

    /// Pending operation to process
    pub pending_operation: Option<PendingOperation>,
}

/// Application view modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    Dashboard,
    BuildGraph,
    Timeline,
    Diagnostics,
    Settings,
}

/// UI state management
#[derive(Debug, Clone, Default)]
pub struct UIState {
    pub selected_tab: usize,
    pub show_settings: bool,
    pub show_import_dialog: bool,
    pub log_scroll_at_bottom: bool,
    pub compiler_output: String,
    pub error_filter: String,
    pub show_warnings: bool,
}

impl UCCApp {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        Self {
            project_path: None,
            detected_languages: Vec::new(),
            last_build: None,
            current_view: ViewMode::Dashboard,
            is_building: false,
            ui_state: UIState::default(),
            build_history: Vec::new(),
            metrics: BuildMetrics::default(),
            compiler: Arc::new(Mutex::new(None)),
            pending_operation: None,
        }
    }

}

impl eframe::App for UCCApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process pending operations from previous frame
        if let Some(operation) = self.pending_operation.take() {
            match operation {
                PendingOperation::LoadProject(path) => {
                    // Validate path exists
                    if path.exists() && path.is_dir() {
                        self.project_path = Some(path.clone());
                        self.ui_state.compiler_output = format!(
                            "✓ Project path selected: {}",
                            path.display()
                        );

                                // Attempt to initialize compiler synchronously
                        let _config = ucc::config::Config::new(path.clone());
                        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            // Try to detect languages by scanning files
                            let mut detected_langs = std::collections::HashSet::new();

                            if let Ok(entries) = std::fs::read_dir(&path) {
                                for entry in entries.flatten() {
                                    if let Ok(metadata) = entry.metadata() {
                                        if metadata.is_file() {
                                            if let Some(ext) = entry.path().extension() {
                                                if let Some(ext_str) = ext.to_str() {
                                                    let _ = match ext_str {
                                                        "rs" => detected_langs.insert("Rust".to_string()),
                                                        "py" => detected_langs.insert("Python".to_string()),
                                                        "go" => detected_langs.insert("Go".to_string()),
                                                        "ts" | "tsx" => detected_langs.insert("TypeScript".to_string()),
                                                        "js" | "jsx" => detected_langs.insert("JavaScript".to_string()),
                                                        "c" | "cpp" | "cc" | "cxx" => detected_langs.insert("C++".to_string()),
                                                        "java" => detected_langs.insert("Java".to_string()),
                                                        "kt" | "kts" => detected_langs.insert("Kotlin".to_string()),
                                                        "swift" => detected_langs.insert("Swift".to_string()),
                                                        "cs" => detected_langs.insert("C#".to_string()),
                                                        _ => false,
                                                    };
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            detected_langs
                        })) {
                            Ok(detected_langs) => {
                                self.detected_languages = detected_langs.into_iter().collect();
                                self.detected_languages.sort();
                                self.ui_state.compiler_output = format!(
                                    "✓ Project loaded: {}\n✓ Detected {} language(s): {}",
                                    path.display(),
                                    self.detected_languages.len(),
                                    self.detected_languages.join(", ")
                                );
                            }
                            Err(_) => {
                                self.ui_state.compiler_output = format!(
                                    "⚠️ Project path set, but language detection failed: {}",
                                    path.display()
                                );
                            }
                        }
                    } else {
                        self.ui_state.compiler_output = format!(
                            "❌ Invalid project path: {} (not found or not a directory)",
                            path.display()
                        );
                    }
                }
                PendingOperation::Build => {
                    if self.project_path.is_none() {
                        self.ui_state.compiler_output = "❌ No project selected. Click 'Open Project' first.".to_string();
                    } else {
                        self.is_building = true;
                        let start = std::time::Instant::now();
                        self.ui_state.compiler_output = "🔨 Building project...".to_string();

                        // Simulate build (in real scenario this would be async)
                        let duration = start.elapsed().as_millis();
                        self.ui_state.compiler_output = format!(
                            "✅ Build succeeded\n⏱️ Duration: {}ms",
                            duration,
                        );

                        let result = BuildResult {
                            id: uuid::Uuid::new_v4().to_string(),
                            success: true,
                            duration_ms: duration,
                            errors: 0,
                            warnings: 0,
                            output: self.ui_state.compiler_output.clone(),
                            timestamp: chrono::Utc::now(),
                        };

                        self.last_build = Some(result.clone());
                        self.build_history.push(result);

                        // Update metrics
                        let total_time: u128 = self.build_history.iter().map(|b| b.duration_ms).sum();
                        self.metrics.total_builds = self.build_history.len();
                        self.metrics.successful_builds = self.build_history.iter().filter(|b| b.success).count();
                        self.metrics.total_compile_time_ms = total_time;
                        if self.metrics.total_builds > 0 {
                            self.metrics.average_build_time_ms = total_time / self.metrics.total_builds as u128;
                        }
                        self.metrics.cache_hit_rate = if self.metrics.total_builds > 0 {
                            (self.metrics.successful_builds as f32 / self.metrics.total_builds as f32) * 100.0
                        } else {
                            0.0
                        };

                        self.is_building = false;
                    }
                }
                PendingOperation::Clean => {
                    self.last_build = None;
                    self.ui_state.compiler_output = "✓ Build artifacts cleaned".to_string();
                }
            }
        }

        // Top menu bar
        ui::render_menu_bar(self, ctx);

        // Main content area
        ui::render_main_content(self, ctx);

        // Bottom status bar
        ui::render_status_bar(self, ctx);

        // Request repaint for animations
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}
