//! CLI demo: update and toggle a settings-configuration UI panel.

use settings_configuration_ui::UI;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = UI::new();
    ui.update("theme=dark".to_string())?;
    println!("{}", ui.render());
    ui.toggle();
    println!("After toggle: {:?}", ui.render());
    Ok(())
}
