//! CLI for layout-components — exercises the real grid layout engine
//! instead of the dead generic Component template.

use layout_components::GridLayout;

#[tokio::main]
async fn main() -> layout_components::Result<()> {
    let item_count: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(7);

    let grid = GridLayout::new(320.0, 3, 10.0, 50.0)?;
    println!("cell width: {:.1}px", grid.cell_width());
    println!("total height for {item_count} items: {:.1}px", grid.total_height(item_count));

    for (i, rect) in grid.layout(item_count).into_iter().enumerate() {
        println!(
            "item {i}: x={:.1} y={:.1} w={:.1} h={:.1}",
            rect.x, rect.y, rect.width, rect.height
        );
    }

    Ok(())
}
