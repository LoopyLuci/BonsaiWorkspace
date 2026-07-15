//! UI Widgets CLI - exercises the component builder, theme manager, and widget database

use ui_widgets::{Component, ComponentSize, ComponentType, ComponentVariant, ThemeManager, WidgetDatabase};

fn main() {
    let button = Component::new("primary-cta".to_string(), ComponentType::Button)
        .with_variant(ComponentVariant::Primary)
        .with_size(ComponentSize::Large);
    println!("component: {} ({})", button.name, button.component_type.as_str());

    let themes = ThemeManager::new();
    if let Some(active) = themes.get_active_theme() {
        println!("active theme: {}", active.name);
        println!("{}", active.as_css());
    }

    let db = WidgetDatabase::new();
    println!("widgets in catalog: {}", db.count());
    for category in db.get_categories() {
        println!("  category: {} ({} widgets)", category, db.list_by_category(&category).len());
    }
}
