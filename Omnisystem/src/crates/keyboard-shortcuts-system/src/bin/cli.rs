//! CLI for keyboard-shortcuts-system — exercises the real keybinding
//! registry instead of the dead generic Component template.

use keyboard_shortcuts_system::KeymapRegistry;

#[tokio::main]
async fn main() -> keyboard_shortcuts_system::Result<()> {
    let mut registry = KeymapRegistry::new();

    registry.register("Ctrl+S", "save")?;
    registry.register("Ctrl+Shift+P", "command-palette")?;
    registry.register("Ctrl+Z", "undo")?;

    let query = std::env::args().nth(1).unwrap_or_else(|| "ctrl+s".to_string());
    match registry.lookup(&query) {
        Some(action) => println!("{query} -> {action}"),
        None => println!("{query} is not bound"),
    }

    println!("all bindings:");
    for binding in registry.list() {
        println!("  {} -> {}", binding.combo, binding.action);
    }

    Ok(())
}
