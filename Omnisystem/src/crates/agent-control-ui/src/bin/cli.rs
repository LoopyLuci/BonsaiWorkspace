//! CLI

use agent_control_ui::UI;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = UI::new();
    ui.update("hello".to_string())?;
    println!("{}", ui.render());
    Ok(())
}
