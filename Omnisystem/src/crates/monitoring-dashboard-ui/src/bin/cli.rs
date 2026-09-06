//! CLI for monitoring-dashboard-ui — exercises the crate's real UI widget API.

use monitoring_dashboard_ui::UI;

fn main() -> monitoring_dashboard_ui::Result<()> {
    let mut ui = UI::new();
    println!("initial render: {}", ui.render());

    let content = std::env::args().nth(1).unwrap_or_else(|| "hello from the CLI".to_string());
    ui.update(content)?;
    println!("after update:   {}", ui.render());

    ui.toggle();
    println!("after toggle:   {:?}", ui.render());

    Ok(())
}
