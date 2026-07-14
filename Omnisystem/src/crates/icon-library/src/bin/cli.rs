//! CLI demo: render an icon component with custom props.

use icon_library::{Component, Props};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut component = Component::new(Props {
        id: "icon-search".to_string(),
        class: "icon--large".to_string(),
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
