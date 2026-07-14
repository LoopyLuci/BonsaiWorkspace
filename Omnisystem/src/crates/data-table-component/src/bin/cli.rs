//! CLI demo: render a data-table component with custom props.

use data_table_component::{Component, Props};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut component = Component::new(Props {
        id: "data-table".to_string(),
        class: "data-table--striped".to_string(),
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
