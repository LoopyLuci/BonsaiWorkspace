//! CLI

use network_management_ui::UI;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = UI::new();
    ui.update("hello".to_string())?;
    println!("{}", ui.render());
    ui.toggle();
    println!("Visible after toggle: {}", ui.render().is_empty());
    Ok(())
}
