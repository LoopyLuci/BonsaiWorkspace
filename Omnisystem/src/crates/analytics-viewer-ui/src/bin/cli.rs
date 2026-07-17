//! CLI demo: update and toggle an analytics-viewer UI panel.

use analytics_viewer_ui::UI;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = UI::new();
    ui.update("chart=revenue-by-quarter".to_string())?;
    println!("{}", ui.render());
    ui.toggle();
    println!("After toggle: {:?}", ui.render());
    Ok(())
}
