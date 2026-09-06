//! CLI for modal-component-library — exercises the real modal stack
//! manager instead of the dead generic Component template.

use modal_component_library::ModalStack;

#[tokio::main]
async fn main() -> modal_component_library::Result<()> {
    let mut modals = ModalStack::new();

    modals.open("settings")?;
    println!("opened 'settings', top: {:?}", modals.top());

    modals.open("confirm-discard")?;
    println!("opened 'confirm-discard', top: {:?}", modals.top());

    println!("stack (bottom to top):");
    for modal in modals.stack() {
        println!("  {} (z-index {})", modal.id, modal.z_index);
    }

    let closed = modals.close_top();
    println!("closed top: {closed:?}, new top: {:?}", modals.top());

    Ok(())
}
