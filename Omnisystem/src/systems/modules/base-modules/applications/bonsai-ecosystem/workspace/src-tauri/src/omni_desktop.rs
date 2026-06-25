//! Workstream D — OmniDesktop: GPU-accelerated compositor
//!
//! Manages a virtual window compositor on top of the existing GpuLayer Vulkan
//! backend.  Windows, panels, wallpaper and overlays are tracked as logical
//! objects; rendering metadata (damage rects, frame requests) is forwarded to
//! the GPU layer.  The physical draw calls are performed by the Tauri WebView
//! compositing pipeline; OmniDesktop provides the *layout engine* and
//! *window state* that drives it.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

use crate::gpu_layer::GpuLayer;
use crate::predictive_engine::PredictedAction;

// ─────────────────────────────────────────────────────────────────────────────
// § 1 — Core geometry types
// ─────────────────────────────────────────────────────────────────────────────

pub type WindowId = String;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.w as i32
            && self.x + self.w as i32 > other.x
            && self.y < other.y + other.h as i32
            && self.y + self.h as i32 > other.y
    }

    pub fn area(&self) -> u64 {
        self.w as u64 * self.h as u64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PanelPosition {
    Top,
    Bottom,
    Left,
    Right,
}

// ─────────────────────────────────────────────────────────────────────────────
// § 2 — Window types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowDecorations {
    pub has_titlebar: bool,
    pub has_borders: bool,
    pub border_radius: u32,
    pub shadow_radius: u32,
    pub accent_color: String,
}

impl Default for WindowDecorations {
    fn default() -> Self {
        Self {
            has_titlebar: true,
            has_borders: true,
            border_radius: 8,
            shadow_radius: 16,
            accent_color: "#7c3aed".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopWindow {
    pub id: WindowId,
    pub title: String,
    pub app_id: String,
    pub bounds: Rect,
    pub z_order: u32,
    pub is_minimized: bool,
    pub is_maximized: bool,
    pub opacity: f32,
    pub decorations: WindowDecorations,
    pub workspace: u32,
    pub created_at: i64,
    pub last_focused_at: i64,
}

impl DesktopWindow {
    pub fn new(title: &str, app_id: &str, bounds: Rect) -> Self {
        let now = chrono::Utc::now().timestamp_micros();
        Self {
            id: Uuid::new_v4().to_string(),
            title: title.to_string(),
            app_id: app_id.to_string(),
            bounds,
            z_order: 0,
            is_minimized: false,
            is_maximized: false,
            opacity: 1.0,
            decorations: WindowDecorations::default(),
            workspace: 0,
            created_at: now,
            last_focused_at: now,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 3 — Panel & widget types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrayIcon {
    pub app_id: String,
    pub tooltip: String,
    pub icon_data_b64: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum PanelWidget {
    AppLauncher {
        pinned_apps: Vec<String>,
    },
    SystemTray {
        icons: Vec<TrayIcon>,
    },
    Clock {
        format: String,
        show_seconds: bool,
    },
    WorkspaceSwitcher {
        count: u32,
        active: u32,
    },
    AIAssistant {
        session_id: String,
    },
    PredictiveBar {
        suggestions: Vec<PredictedAction>,
    },
    ResourceMonitor {
        show_cpu: bool,
        show_gpu: bool,
        show_ram: bool,
    },
    Separator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopPanel {
    pub id: String,
    pub position: PanelPosition,
    pub widgets: Vec<PanelWidget>,
    pub auto_hide: bool,
    pub height_px: u32,
    pub background_color: String,
    pub opacity: f32,
}

impl DesktopPanel {
    pub fn default_bottom() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            position: PanelPosition::Bottom,
            widgets: vec![
                PanelWidget::AppLauncher {
                    pinned_apps: vec![],
                },
                PanelWidget::Separator,
                PanelWidget::PredictiveBar {
                    suggestions: vec![],
                },
                PanelWidget::Separator,
                PanelWidget::ResourceMonitor {
                    show_cpu: true,
                    show_gpu: true,
                    show_ram: true,
                },
                PanelWidget::SystemTray { icons: vec![] },
                PanelWidget::Clock {
                    format: "%H:%M".into(),
                    show_seconds: false,
                },
            ],
            auto_hide: false,
            height_px: 48,
            background_color: "#1a1a2e".into(),
            opacity: 0.95,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 4 — Wallpaper and layout types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum Wallpaper {
    SolidColor {
        color: String,
    },
    Image {
        path: String,
        scale_mode: ScaleMode,
    },
    Gradient {
        from: String,
        to: String,
        angle_deg: u32,
    },
    AnimatedShader {
        shader_id: String,
        fps: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScaleMode {
    Fill,
    Fit,
    Center,
    Tile,
}

impl Default for Wallpaper {
    fn default() -> Self {
        Self::Gradient {
            from: "#0f0f23".into(),
            to: "#1a1040".into(),
            angle_deg: 135,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WindowLayout {
    Maximized,
    SplitHorizontal,
    SplitVertical,
    Grid { cols: u32 },
    Tiled,
    Float,
}

// ─────────────────────────────────────────────────────────────────────────────
// § 5 — Damage tracker
// ─────────────────────────────────────────────────────────────────────────────

struct DamageTracker {
    dirty_rects: RwLock<Vec<Rect>>,
}

impl DamageTracker {
    fn new() -> Self {
        Self {
            dirty_rects: RwLock::new(Vec::new()),
        }
    }

    async fn mark_dirty(&self, rect: Rect) {
        let mut dirty = self.dirty_rects.write().await;
        // Merge with existing dirty rects if they overlap
        if let Some(existing) = dirty.iter_mut().find(|r| r.intersects(&rect)) {
            let nx = existing.x.min(rect.x);
            let ny = existing.y.min(rect.y);
            let nx2 = (existing.x + existing.w as i32).max(rect.x + rect.w as i32);
            let ny2 = (existing.y + existing.h as i32).max(rect.y + rect.h as i32);
            *existing = Rect::new(nx, ny, (nx2 - nx) as u32, (ny2 - ny) as u32);
        } else {
            dirty.push(rect);
        }
    }

    async fn flush(&self) -> Vec<Rect> {
        let mut dirty = self.dirty_rects.write().await;
        std::mem::take(&mut *dirty)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 6 — Display config
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
    pub refresh_hz: u32,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            scale_factor: 1.0,
            refresh_hz: 60,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 7 — OmniDesktop
// ─────────────────────────────────────────────────────────────────────────────

pub struct OmniDesktop {
    pub gpu: Arc<GpuLayer>,
    pub windows: RwLock<HashMap<WindowId, DesktopWindow>>,
    pub panels: RwLock<Vec<DesktopPanel>>,
    pub wallpaper: RwLock<Wallpaper>,
    pub display: RwLock<DisplayConfig>,
    damage: DamageTracker,
    z_counter: AtomicU32,
    focused_window: RwLock<Option<WindowId>>,
    active_workspace: AtomicU32,
}

impl OmniDesktop {
    pub fn new(gpu: Arc<GpuLayer>) -> Arc<Self> {
        let desktop = Arc::new(Self {
            gpu,
            windows: RwLock::new(HashMap::new()),
            panels: RwLock::new(vec![DesktopPanel::default_bottom()]),
            wallpaper: RwLock::new(Wallpaper::default()),
            display: RwLock::new(DisplayConfig::default()),
            damage: DamageTracker::new(),
            z_counter: AtomicU32::new(1),
            focused_window: RwLock::new(None),
            active_workspace: AtomicU32::new(0),
        });
        info!("[build-desktop] compositor initialised");
        desktop
    }

    // ── Window lifecycle ─────────────────────────────────────────────────────

    pub async fn open_window(&self, title: &str, app_id: &str, bounds: Rect) -> WindowId {
        let mut win = DesktopWindow::new(title, app_id, bounds);
        win.z_order = self.z_counter.fetch_add(1, Ordering::Relaxed);
        let id = win.id.clone();
        self.damage.mark_dirty(bounds).await;
        self.windows.write().await.insert(id.clone(), win);
        *self.focused_window.write().await = Some(id.clone());
        debug!("[build-desktop] opened window {id} ({app_id})");
        id
    }

    pub async fn close_window(&self, id: &str) {
        let removed = self.windows.write().await.remove(id);
        if let Some(win) = removed {
            self.damage.mark_dirty(win.bounds).await;
            let mut focused = self.focused_window.write().await;
            if focused.as_deref() == Some(id) {
                *focused = None;
            }
        }
    }

    pub async fn focus_window(&self, id: &str) {
        let bounds = {
            let mut windows = self.windows.write().await;
            if let Some(win) = windows.get_mut(id) {
                win.z_order = self.z_counter.fetch_add(1, Ordering::Relaxed);
                win.last_focused_at = chrono::Utc::now().timestamp_micros();
                Some(win.bounds)
            } else {
                None
            }
        };
        if let Some(b) = bounds {
            self.damage.mark_dirty(b).await;
        }
        *self.focused_window.write().await = Some(id.to_string());
    }

    pub async fn move_window(&self, id: &str, new_bounds: Rect) -> bool {
        let old_bounds = {
            let mut windows = self.windows.write().await;
            if let Some(win) = windows.get_mut(id) {
                let old = win.bounds;
                win.bounds = new_bounds;
                Some(old)
            } else {
                None
            }
        };
        if let Some(old) = old_bounds {
            self.damage.mark_dirty(old).await;
            self.damage.mark_dirty(new_bounds).await;
            true
        } else {
            false
        }
    }

    pub async fn minimize_window(&self, id: &str) -> bool {
        let bounds = {
            let mut windows = self.windows.write().await;
            windows.get_mut(id).map(|w| {
                w.is_minimized = true;
                w.bounds
            })
        };
        if let Some(b) = bounds {
            self.damage.mark_dirty(b).await;
            true
        } else {
            false
        }
    }

    pub async fn maximize_window(&self, id: &str) -> bool {
        let display = self.display.read().await;
        let full = Rect::new(0, 0, display.width, display.height);
        drop(display);
        let ok = {
            let mut windows = self.windows.write().await;
            if let Some(win) = windows.get_mut(id) {
                win.bounds = full;
                win.is_maximized = true;
                win.is_minimized = false;
                true
            } else {
                false
            }
        };
        if ok {
            self.damage.mark_dirty(full).await;
        }
        ok
    }

    pub async fn restore_window(&self, id: &str) -> bool {
        let bounds = {
            let mut windows = self.windows.write().await;
            windows.get_mut(id).map(|w| {
                w.is_minimized = false;
                w.is_maximized = false;
                w.bounds
            })
        };
        if let Some(b) = bounds {
            self.damage.mark_dirty(b).await;
            true
        } else {
            false
        }
    }

    // ── Layout engine ────────────────────────────────────────────────────────

    /// AI-driven auto-layout based on visible window count
    pub async fn auto_layout(&self) -> WindowLayout {
        let workspace = self.active_workspace.load(Ordering::Relaxed);
        let count = self
            .windows
            .read()
            .await
            .values()
            .filter(|w| !w.is_minimized && w.workspace == workspace)
            .count();
        match count {
            0 | 1 => WindowLayout::Maximized,
            2 => WindowLayout::SplitHorizontal,
            3 | 4 => WindowLayout::Grid { cols: 2 },
            _ => WindowLayout::Tiled,
        }
    }

    /// Apply a layout to all visible windows on the current workspace
    pub async fn apply_layout(&self, layout: &WindowLayout) {
        let display = self.display.read().await;
        let workspace = self.active_workspace.load(Ordering::Relaxed);
        let panel_h = {
            let panels = self.panels.read().await;
            panels
                .iter()
                .filter(|p| matches!(p.position, PanelPosition::Bottom))
                .map(|p| p.height_px)
                .sum::<u32>()
        };
        let w = display.width;
        let h = display.height.saturating_sub(panel_h);

        let ids: Vec<WindowId> = self
            .windows
            .read()
            .await
            .values()
            .filter(|win| !win.is_minimized && win.workspace == workspace)
            .map(|win| win.id.clone())
            .collect();

        let n = ids.len() as u32;
        if n == 0 {
            return;
        }

        match layout {
            WindowLayout::Maximized => {
                if let Some(id) = ids.first() {
                    self.move_window(id, Rect::new(0, 0, w, h)).await;
                }
            }
            WindowLayout::SplitHorizontal => {
                for (i, id) in ids.iter().enumerate() {
                    let half = w / 2;
                    self.move_window(id, Rect::new((i as u32 * half) as i32, 0, half, h))
                        .await;
                }
            }
            WindowLayout::SplitVertical => {
                for (i, id) in ids.iter().enumerate() {
                    let half = h / 2;
                    self.move_window(id, Rect::new(0, (i as u32 * half) as i32, w, half))
                        .await;
                }
            }
            WindowLayout::Grid { cols } => {
                let cols = (*cols).max(1);
                let rows = (n + cols - 1) / cols;
                let cw = w / cols;
                let ch = h / rows;
                for (i, id) in ids.iter().enumerate() {
                    let col = i as u32 % cols;
                    let row = i as u32 / cols;
                    self.move_window(id, Rect::new((col * cw) as i32, (row * ch) as i32, cw, ch))
                        .await;
                }
            }
            WindowLayout::Tiled => {
                // Simple left-heavy tiling: first window gets 60%, rest share 40%
                if let Some((first, rest)) = ids.split_first() {
                    let main_w = (w as f32 * 0.6) as u32;
                    self.move_window(first, Rect::new(0, 0, main_w, h)).await;
                    let side_w = w - main_w;
                    let side_h = if rest.is_empty() {
                        h
                    } else {
                        h / rest.len() as u32
                    };
                    for (i, id) in rest.iter().enumerate() {
                        self.move_window(
                            id,
                            Rect::new(main_w as i32, (i as u32 * side_h) as i32, side_w, side_h),
                        )
                        .await;
                    }
                }
            }
            WindowLayout::Float => {} // no-op for floating layout
        }
    }

    // ── Workspace management ─────────────────────────────────────────────────

    pub fn switch_workspace(&self, workspace: u32) {
        self.active_workspace.store(workspace, Ordering::Relaxed);
        info!("[build-desktop] switched to workspace {workspace}");
    }

    // ── Wallpaper ────────────────────────────────────────────────────────────

    pub async fn set_wallpaper(&self, wallpaper: Wallpaper) {
        let display = self.display.read().await;
        *self.wallpaper.write().await = wallpaper;
        self.damage
            .mark_dirty(Rect::new(0, 0, display.width, display.height))
            .await;
    }

    // ── Panel management ─────────────────────────────────────────────────────

    pub async fn add_panel(&self, panel: DesktopPanel) -> String {
        let id = panel.id.clone();
        self.panels.write().await.push(panel);
        id
    }

    pub async fn add_panel_widget(&self, panel_id: &str, widget: PanelWidget) -> bool {
        let mut panels = self.panels.write().await;
        if let Some(panel) = panels.iter_mut().find(|p| p.id == panel_id) {
            panel.widgets.push(widget);
            true
        } else {
            false
        }
    }

    // ── Frame metadata ───────────────────────────────────────────────────────

    pub async fn flush_damage(&self) -> Vec<Rect> {
        self.damage.flush().await
    }

    pub async fn window_list(&self) -> Vec<DesktopWindow> {
        let mut windows: Vec<_> = self.windows.read().await.values().cloned().collect();
        windows.sort_by_key(|w| w.z_order);
        windows
    }

    pub fn active_workspace_id(&self) -> u32 {
        self.active_workspace.load(Ordering::Relaxed)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 8 — Tauri commands
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct OpenWindowRequest {
    pub title: String,
    pub app_id: String,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

#[tauri::command]
pub async fn omni_window_list(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<DesktopWindow>, String> {
    Ok(state.omni_desktop.window_list().await)
}

#[tauri::command]
pub async fn omni_window_open(
    state: tauri::State<'_, crate::AppState>,
    req: OpenWindowRequest,
) -> Result<WindowId, String> {
    Ok(state
        .omni_desktop
        .open_window(
            &req.title,
            &req.app_id,
            Rect::new(req.x, req.y, req.w, req.h),
        )
        .await)
}

#[tauri::command]
pub async fn omni_window_close(
    state: tauri::State<'_, crate::AppState>,
    window_id: String,
) -> Result<(), String> {
    state.omni_desktop.close_window(&window_id).await;
    Ok(())
}

#[tauri::command]
pub async fn omni_window_focus(
    state: tauri::State<'_, crate::AppState>,
    window_id: String,
) -> Result<(), String> {
    state.omni_desktop.focus_window(&window_id).await;
    Ok(())
}

#[tauri::command]
pub async fn omni_window_move(
    state: tauri::State<'_, crate::AppState>,
    window_id: String,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
) -> Result<bool, String> {
    Ok(state
        .omni_desktop
        .move_window(&window_id, Rect::new(x, y, w, h))
        .await)
}

#[tauri::command]
pub async fn omni_window_minimize(
    state: tauri::State<'_, crate::AppState>,
    window_id: String,
) -> Result<bool, String> {
    Ok(state.omni_desktop.minimize_window(&window_id).await)
}

#[tauri::command]
pub async fn omni_window_maximize(
    state: tauri::State<'_, crate::AppState>,
    window_id: String,
) -> Result<bool, String> {
    Ok(state.omni_desktop.maximize_window(&window_id).await)
}

#[tauri::command]
pub async fn omni_window_restore(
    state: tauri::State<'_, crate::AppState>,
    window_id: String,
) -> Result<bool, String> {
    Ok(state.omni_desktop.restore_window(&window_id).await)
}

#[tauri::command]
pub async fn omni_desktop_layout(
    state: tauri::State<'_, crate::AppState>,
    layout: Option<WindowLayout>,
) -> Result<WindowLayout, String> {
    let layout = match layout {
        Some(l) => l,
        None => state.omni_desktop.auto_layout().await,
    };
    state.omni_desktop.apply_layout(&layout).await;
    Ok(layout)
}

#[tauri::command]
pub async fn omni_desktop_wallpaper_set(
    state: tauri::State<'_, crate::AppState>,
    wallpaper: Wallpaper,
) -> Result<(), String> {
    state.omni_desktop.set_wallpaper(wallpaper).await;
    Ok(())
}

#[tauri::command]
pub async fn omni_panel_list(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<DesktopPanel>, String> {
    Ok(state.omni_desktop.panels.read().await.clone())
}

#[tauri::command]
pub async fn omni_panel_add(
    state: tauri::State<'_, crate::AppState>,
    panel: DesktopPanel,
) -> Result<String, String> {
    Ok(state.omni_desktop.add_panel(panel).await)
}

#[tauri::command]
pub async fn omni_panel_widget_add(
    state: tauri::State<'_, crate::AppState>,
    panel_id: String,
    widget: PanelWidget,
) -> Result<bool, String> {
    Ok(state.omni_desktop.add_panel_widget(&panel_id, widget).await)
}

#[tauri::command]
pub async fn omni_desktop_damage(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<Rect>, String> {
    Ok(state.omni_desktop.flush_damage().await)
}

#[tauri::command]
pub async fn omni_workspace_switch(
    state: tauri::State<'_, crate::AppState>,
    workspace: u32,
) -> Result<(), String> {
    state.omni_desktop.switch_workspace(workspace);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_gpu() -> Arc<GpuLayer> {
        Arc::new(GpuLayer::new(&crate::gpu_layer::GpuInfo {
            has_vulkan: false,
            has_directml: false,
        }))
    }

    #[tokio::test]
    async fn open_and_close_window() {
        let desk = OmniDesktop::new(make_gpu());
        let id = desk
            .open_window("Test", "test-app", Rect::new(0, 0, 800, 600))
            .await;
        assert!(desk.windows.read().await.contains_key(&id));
        desk.close_window(&id).await;
        assert!(!desk.windows.read().await.contains_key(&id));
    }

    #[tokio::test]
    async fn auto_layout_single() {
        let desk = OmniDesktop::new(make_gpu());
        desk.open_window("A", "app-a", Rect::new(0, 0, 400, 300))
            .await;
        assert_eq!(desk.auto_layout().await, WindowLayout::Maximized);
    }

    #[tokio::test]
    async fn auto_layout_two() {
        let desk = OmniDesktop::new(make_gpu());
        desk.open_window("A", "app-a", Rect::new(0, 0, 400, 300))
            .await;
        desk.open_window("B", "app-b", Rect::new(400, 0, 400, 300))
            .await;
        assert_eq!(desk.auto_layout().await, WindowLayout::SplitHorizontal);
    }

    #[tokio::test]
    async fn damage_rect_merge() {
        let tracker = DamageTracker::new();
        tracker.mark_dirty(Rect::new(0, 0, 100, 100)).await;
        tracker.mark_dirty(Rect::new(50, 50, 100, 100)).await;
        let rects = tracker.flush().await;
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0], Rect::new(0, 0, 150, 150));
    }
}
