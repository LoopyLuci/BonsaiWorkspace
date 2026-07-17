//! CLI demo: update and toggle a volume-management UI panel.

use volume_management_ui::UI;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = UI::new();
    ui.update("volume=data-01".to_string())?;
    println!("{}", ui.render());
    ui.toggle();
    println!("After toggle: {:?}", ui.render());
    Ok(())
}
