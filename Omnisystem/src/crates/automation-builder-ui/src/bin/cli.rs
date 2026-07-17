//! CLI demo: update and toggle an automation-builder UI panel.

use automation_builder_ui::UI;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = UI::new();
    ui.update("workflow=deploy-pipeline".to_string())?;
    println!("{}", ui.render());
    ui.toggle();
    println!("After toggle: {:?}", ui.render());
    Ok(())
}
