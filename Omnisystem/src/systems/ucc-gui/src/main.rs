//! UCC GUI - Production-Grade Universal Cross-Compiler Interface

mod app;
mod ui;
mod models;

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 1000.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(&[255, 255, 255, 255])
                    .unwrap_or_default(),
            ),
        ..Default::default()
    };

    eframe::run_native(
        "🔨 UCC - Universal Cross-Compiler",
        options,
        Box::new(|cc| {
            let app = app::UCCApp::new(cc);
            Ok(Box::new(app))
        }),
    )
}
