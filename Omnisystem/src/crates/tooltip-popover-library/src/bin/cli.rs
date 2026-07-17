//! CLI demo: render a tooltip component with custom props.

use tooltip_popover_library::{Component, Props};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut component = Component::new(Props {
        id: "help-tooltip".to_string(),
        class: "tooltip--top".to_string(),
        disabled: false,
    });
    println!("{}", component.render());

    component.update_props(Props {
        disabled: true,
        ..component.props().clone()
    });
    println!("{}", component.render());
    println!("Disabled: {}", component.props().disabled);

    Ok(())
}
