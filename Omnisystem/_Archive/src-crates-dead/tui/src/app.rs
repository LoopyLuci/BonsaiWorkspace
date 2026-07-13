use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[derive(Parser, Debug, Clone)]
#[command(name = "bti", about = "Bonsai TUI — terminal interface for the Bonsai Ecosystem")]
pub struct CliArgs {
    /// Connect to remote daemon (default: 127.0.0.1:11370)
    #[arg(long, value_name = "HOST:PORT")]
    pub connect: Option<String>,

    /// Auth token (also reads from ~/.bonsai/vscode_token)
    #[arg(long, value_name = "TOKEN")]
    pub token: Option<String>,

    /// Start with this panel active
    #[arg(long, value_name = "NAME")]
    pub panel: Option<String>,

    /// Run one command and print result (non-interactive)
    #[arg(long, value_name = "CMD")]
    pub exec: Option<String>,

    /// List daemons on LAN
    #[arg(long)]
    pub discover: bool,

    /// Headless mode (not yet implemented)
    #[arg(long)]
    pub headless: bool,

    /// Write tracing logs to file instead of stderr
    #[arg(long, value_name = "PATH")]
    pub log_file: Option<PathBuf>,
}

use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers};
use futures_util::StreamExt;
use ratatui::layout::{Constraint, Layout};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::{
    client::DaemonClient,
    mode::Mode,
    panel::{Panel, PanelMeta},
    panels::{
        chat_panel::ChatPanel,
        collaboration_panel::CollaborationPanel,
        compute_panel::ComputePanel,
        credits_panel::CreditsPanel,
        estimate_panel::EstimatePanel,
        files_panel::FilesPanel,
        health_panel::HealthPanel,
        logs_panel::LogsPanel,
        marketplace_panel::MarketplacePanel,
        settings_panel::SettingsPanel,
        terminal_panel::TerminalPanel,
        trainer_panel::TrainerPanel,
    },
    theme::Theme,
    widgets::{command_line, sidebar, status_bar, tab_bar},
};

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum PanelId {
    Chat,
    Files,
    Trainer,
    Terminal,
    Health,
    Collaboration,
    Marketplace,
    Credits,
    Estimate,
    Compute,
    Settings,
    Logs,
}

pub struct App {
    pub client: Arc<Mutex<DaemonClient>>,
    pub active_panel: PanelId,
    pub panels: Vec<Box<dyn Panel>>,
    pub mode: Mode,
    pub command_buf: String,
    pub status_msg: String,
    pub theme: Theme,
    pub show_sidebar: bool,
    pub connected: bool,
    pub balance: f64,
}

impl App {
    pub fn panel_metas(&self) -> Vec<PanelMeta> {
        self.panels
            .iter()
            .map(|p| PanelMeta {
                id: p.id(),
                name: p.name().to_string(),
                icon: p.icon().to_string(),
            })
            .collect()
    }

    fn active_panel_name(&self) -> &str {
        self.panels
            .iter()
            .find(|p| p.id() == self.active_panel)
            .map(|p| p.name())
            .unwrap_or("unknown")
    }

    fn active_panel_idx(&self) -> Option<usize> {
        self.panels.iter().position(|p| p.id() == self.active_panel)
    }

    fn set_active_by_index(&mut self, idx: usize) {
        if let Some(p) = self.panels.get(idx) {
            self.active_panel = p.id();
        }
    }

    fn handle_command(&mut self, input: &str) {
        let input = input.trim().to_string();
        if input.is_empty() {
            return;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        let cmd = parts[0];
        let args = &parts[1..];

        match cmd {
            "q" | "quit" | "exit" => {
                // Signal quit via a side-channel — handled in run()
                self.status_msg = "__QUIT__".into();
            }
            "theme" => {
                if args.first() == Some(&"light") {
                    self.theme = Theme::light();
                    self.status_msg = "Theme set to light".into();
                } else {
                    self.theme = Theme::dark();
                    self.status_msg = "Theme set to dark".into();
                }
            }
            _ => {
                // Delegate to active panel
                let idx = self.active_panel_idx();
                if let Some(i) = idx {
                    let result = self.panels[i].run_command(cmd, args);
                    self.status_msg = result.unwrap_or_else(|| format!("Unknown command: {}", cmd));
                }
            }
        }
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> bool {
        // Global Ctrl+C / Ctrl+Q always quit
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('q') => return true, // quit
                KeyCode::Char('b') => {
                    self.show_sidebar = !self.show_sidebar;
                    return false;
                }
                _ => {}
            }
        }

        match &self.mode.clone() {
            Mode::Normal => {
                match key.code {
                    KeyCode::Char('q') => return true,
                    KeyCode::Char('?') => {
                        self.status_msg = "Keys: 1-9 panels | Tab cycle | j/k nav | : cmd | Ctrl+B sidebar | q quit".into();
                    }
                    KeyCode::Char(':') => {
                        self.mode = Mode::Command;
                        self.command_buf.clear();
                    }
                    KeyCode::Esc => {
                        self.status_msg.clear();
                    }
                    KeyCode::Tab => {
                        if let Some(idx) = self.active_panel_idx() {
                            self.set_active_by_index((idx + 1) % self.panels.len());
                        }
                    }
                    KeyCode::BackTab => {
                        if let Some(idx) = self.active_panel_idx() {
                            let new_idx = if idx == 0 { self.panels.len() - 1 } else { idx - 1 };
                            self.set_active_by_index(new_idx);
                        }
                    }
                    KeyCode::Char(c @ '1'..='9') => {
                        let idx = (c as u8 - b'1') as usize;
                        self.set_active_by_index(idx);
                    }
                    _ => {
                        // Delegate to active panel
                        let idx = self.active_panel_idx();
                        if let Some(i) = idx {
                            self.panels[i].handle_key(key, &mut self.mode);
                        }
                    }
                }
            }
            Mode::Command => {
                match key.code {
                    KeyCode::Esc => {
                        self.mode = Mode::Normal;
                        self.command_buf.clear();
                    }
                    KeyCode::Enter => {
                        let cmd = self.command_buf.clone();
                        self.command_buf.clear();
                        self.mode = Mode::Normal;
                        self.handle_command(&cmd);
                    }
                    KeyCode::Backspace => {
                        self.command_buf.pop();
                    }
                    KeyCode::Char(c) => {
                        self.command_buf.push(c);
                    }
                    KeyCode::Tab => {
                        // Stub completion
                    }
                    _ => {}
                }
            }
            Mode::Insert => {
                // Delegate entirely to active panel
                let idx = self.active_panel_idx();
                if let Some(i) = idx {
                    self.panels[i].handle_key(key, &mut self.mode);
                }
            }
        }

        false
    }

    fn broadcast_event(&mut self, event: Value) {
        for panel in self.panels.iter_mut() {
            panel.handle_daemon_event(&event);
        }
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();

        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

        let status_area = chunks[0];
        let main_area = chunks[1];
        let tab_area = chunks[2];

        // Status bar
        status_bar::render_status_bar(
            frame,
            status_area,
            &self.theme,
            self.connected,
            self.balance,
            self.active_panel_name(),
            &self.mode,
        );

        // Tab bar
        let metas = self.panel_metas();
        tab_bar::render_tab_bar(frame, tab_area, &self.theme, &metas, self.active_panel);

        // Main: optional sidebar + content
        let content_area = if self.show_sidebar {
            let main_chunks = Layout::horizontal([
                Constraint::Percentage(20),
                Constraint::Percentage(80),
            ])
            .split(main_area);

            sidebar::render_sidebar(frame, main_chunks[0], &self.theme, &metas, self.active_panel);
            main_chunks[1]
        } else {
            main_area
        };

        // Render active panel
        let active_id = self.active_panel;
        let theme = self.theme.clone();
        if let Some(panel) = self.panels.iter_mut().find(|p| p.id() == active_id) {
            if self.mode == Mode::Command {
                // Render panel then overlay command line
                let panel_area = Rect {
                    x: content_area.x,
                    y: content_area.y,
                    width: content_area.width,
                    height: content_area.height,
                };
                panel.render(frame, panel_area, &theme);
                command_line::render_command_line(frame, panel_area, &theme, &self.command_buf);
            } else {
                panel.render(frame, content_area, &theme);
            }
        }

        // Show status message if not shown via command line
        if !self.status_msg.is_empty() && self.mode != Mode::Command {
            let _ = status_area;
        }
    }
}

pub async fn run(args: CliArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let host_port = args.connect.clone().unwrap_or_else(|| "127.0.0.1:11370".into());
    let (host, port) = if let Some(colon) = host_port.rfind(':') {
        let h = host_port[..colon].to_string();
        let p: u16 = host_port[colon + 1..].parse().unwrap_or(11370);
        (h, p)
    } else {
        (host_port, 11370u16)
    };

    let token = args.token.clone().unwrap_or_else(|| {
        dirs::home_dir()
            .map(|h| h.join(".bonsai").join("vscode_token"))
            .and_then(|p| std::fs::read_to_string(p).ok())
            .unwrap_or_default()
            .trim()
            .to_string()
    });

    let (client, mut event_rx) = DaemonClient::connect(host, port, token).await?;
    let connected = client.is_connected();
    let client = Arc::new(Mutex::new(client));

    let panels: Vec<Box<dyn Panel>> = vec![
        Box::new(ChatPanel::new()),
        Box::new(FilesPanel::new()),
        Box::new(TrainerPanel::new()),
        Box::new(TerminalPanel::new()),
        Box::new(HealthPanel::new()),
        Box::new(CollaborationPanel::new()),
        Box::new(MarketplacePanel::new()),
        Box::new(CreditsPanel::new()),
        Box::new(EstimatePanel::new()),
        Box::new(ComputePanel::new()),
        Box::new(SettingsPanel::new()),
        Box::new(LogsPanel::new()),
    ];

    let initial_panel = match args.panel.as_deref() {
        Some("chat") => PanelId::Chat,
        Some("trainer") => PanelId::Trainer,
        Some("marketplace") => PanelId::Marketplace,
        Some("credits") => PanelId::Credits,
        Some("health") => PanelId::Health,
        Some("logs") => PanelId::Logs,
        Some("terminal") => PanelId::Terminal,
        _ => PanelId::Chat,
    };

    let mut app = App {
        client,
        active_panel: initial_panel,
        panels,
        mode: Mode::Normal,
        command_buf: String::new(),
        status_msg: String::new(),
        theme: Theme::dark(),
        show_sidebar: true,
        connected,
        balance: 42.0,
    };

    // Init ratatui
    let mut terminal = ratatui::init();
    let mut event_stream = EventStream::new();
    let tick_rate = Duration::from_millis(16);

    let result: Result<(), Box<dyn std::error::Error + Send + Sync>> = loop {
        // Draw
        terminal.draw(|frame| app.draw(frame))?;

        // Select on events
        let tick = tokio::time::sleep(tick_rate);
        tokio::select! {
            maybe_event = event_stream.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) => {
                        let quit = app.handle_key(key);
                        if quit || app.status_msg == "__QUIT__" {
                            break Ok(());
                        }
                    }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        tracing::error!("Event error: {e}");
                    }
                    None => break Ok(()),
                }
            }
            maybe_msg = event_rx.recv() => {
                if let Some(msg) = maybe_msg {
                    app.broadcast_event(msg);
                }
            }
            _ = tick => {}
        }
    };

    ratatui::restore();
    result
}

// Need Rect in scope for draw method
use ratatui::layout::Rect;
