//! CLI demo: render an infinite-scroll component with custom props.

use infinite_scroll_component::{Component, Props};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut component = Component::new(Props {
        id: "feed-scroll".to_string(),
        class: "scroll--vertical".to_string(),
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
