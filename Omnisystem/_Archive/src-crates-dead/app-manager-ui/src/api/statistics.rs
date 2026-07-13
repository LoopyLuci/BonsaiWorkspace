use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstallationStats {
    pub total_apps: u32,
    pub total_size_mb: u64,
    pub installation_count: u32,
    pub last_installed: Option<String>,
    pub apps_by_category: HashMap<String, u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UsageStats {
    pub total_app_launches: u32,
    pub average_app_rating: f32,
    pub most_used_apps: Vec<(String, u32)>,
    pub most_searched_terms: Vec<(String, u32)>,
}

#[tauri::command]
pub async fn get_installation_stats() -> Result<InstallationStats, String> {
    // Mock data for Phase 4 Week 2
    // In production, this would query the actual app registry
    let mut apps_by_category = HashMap::new();
    apps_by_category.insert("productivity".to_string(), 12);
    apps_by_category.insert("entertainment".to_string(), 8);
    apps_by_category.insert("utilities".to_string(), 15);
    apps_by_category.insert("development".to_string(), 10);
    apps_by_category.insert("social".to_string(), 6);
    apps_by_category.insert("business".to_string(), 9);

    Ok(InstallationStats {
        total_apps: 60,
        total_size_mb: 45000,
        installation_count: 35,
        last_installed: Some("2026-06-12T14:30:00Z".to_string()),
        apps_by_category,
    })
}

#[tauri::command]
pub async fn get_usage_statistics() -> Result<UsageStats, String> {
    // Mock data for Phase 4 Week 2
    // In production, this would track actual app usage
    let most_used_apps = vec![
        ("VS Code".to_string(), 450),
        ("Chrome".to_string(), 380),
        ("Slack".to_string(), 280),
        ("Figma".to_string(), 150),
        ("Notion".to_string(), 120),
    ];

    let most_searched_terms = vec![
        ("productivity".to_string(), 95),
        ("code editor".to_string(), 72),
        ("design".to_string(), 58),
        ("communication".to_string(), 42),
        ("note taking".to_string(), 31),
    ];

    Ok(UsageStats {
        total_app_launches: 1380,
        average_app_rating: 4.2,
        most_used_apps,
        most_searched_terms,
    })
}
