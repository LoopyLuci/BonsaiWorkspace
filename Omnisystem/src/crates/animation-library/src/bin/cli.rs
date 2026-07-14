//! CLI demo: render an animation component with custom props.

use animation_library::{Component, Props};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut component = Component::new(Props {
        id: "fade-transition".to_string(),
        class: "anim--fade".to_string(),
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
