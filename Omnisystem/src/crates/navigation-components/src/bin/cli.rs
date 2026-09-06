//! CLI for navigation-components — exercises the real navigation stack
//! instead of the dead generic Component template.

use navigation_components::{NavigationStack, Route};

#[tokio::main]
async fn main() -> navigation_components::Result<()> {
    let mut nav = NavigationStack::new(Route::new("/", "Home"));
    println!("current: {}", nav.current().path);

    nav.push(Route::new("/settings", "Settings"));
    nav.push(Route::new("/settings/profile", "Profile"));
    println!("current: {} (depth {})", nav.current().path, nav.depth());

    let back_to = nav.back()?;
    println!("went back to: {}", back_to.path);

    println!("history:");
    for route in nav.history() {
        println!("  {} — {}", route.path, route.title);
    }

    Ok(())
}
