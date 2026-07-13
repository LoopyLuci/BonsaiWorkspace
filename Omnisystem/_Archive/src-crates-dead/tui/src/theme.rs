use ratatui::style::Color;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub warning: Color,
    pub error: Color,
    pub success: Color,
    pub border: Color,
    pub selection: Color,
    pub muted: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Theme {
            bg: Color::Rgb(26, 27, 38),
            fg: Color::Rgb(192, 202, 245),
            accent: Color::Rgb(122, 162, 247),
            warning: Color::Rgb(224, 175, 104),
            error: Color::Rgb(247, 118, 142),
            success: Color::Rgb(158, 206, 106),
            border: Color::Rgb(86, 95, 137),
            selection: Color::Rgb(41, 46, 66),
            muted: Color::Rgb(86, 95, 137),
        }
    }

    pub fn light() -> Self {
        Theme {
            bg: Color::Rgb(255, 255, 255),
            fg: Color::Rgb(52, 59, 88),
            accent: Color::Rgb(52, 108, 203),
            warning: Color::Rgb(180, 120, 40),
            error: Color::Rgb(200, 40, 70),
            success: Color::Rgb(60, 150, 40),
            border: Color::Rgb(180, 185, 210),
            selection: Color::Rgb(220, 225, 245),
            muted: Color::Rgb(140, 148, 175),
        }
    }

    pub fn from_toml(_path: &Path) -> Option<Self> {
        // Stub: TOML theme loading not yet implemented
        None
    }
}
