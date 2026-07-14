//! CLI demo: update and toggle an alerting-configuration UI panel.

use alerting_configuration_ui::UI;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = UI::new();
    ui.update("threshold=95pct".to_string())?;
    println!("{}", ui.render());
    ui.toggle();
    println!("After toggle: {:?}", ui.render());
    Ok(())
}
