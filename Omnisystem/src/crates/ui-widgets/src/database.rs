use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Widget entry in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetEntry {
    pub widget_id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub code_example: String,
    pub props: Vec<PropDefinition>,
    pub accessibility_level: String, // WCAG level: A, AA, AAA
    pub performance_score: f32,      // 0-100
}

impl WidgetEntry {
    pub fn new(name: String, category: String) -> Self {
        WidgetEntry {
            widget_id: Uuid::new_v4().to_string(),
            name,
            description: String::new(),
            category,
            code_example: String::new(),
            props: vec![],
            accessibility_level: "AA".to_string(),
            performance_score: 95.0,
        }
    }
}

/// Property definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropDefinition {
    pub name: String,
    pub prop_type: String,
    pub required: bool,
    pub default_value: Option<String>,
    pub description: String,
}

/// Widget database
pub struct WidgetDatabase {
    entries: Vec<WidgetEntry>,
}

impl WidgetDatabase {
    pub fn new() -> Self {
        WidgetDatabase {
            entries: Self::init_default_widgets(),
        }
    }

    fn init_default_widgets() -> Vec<WidgetEntry> {
        vec![
            WidgetEntry::new("Button".to_string(), "Input Controls".to_string()),
            WidgetEntry::new("TextInput".to_string(), "Input Controls".to_string()),
            WidgetEntry::new("Select".to_string(), "Input Controls".to_string()),
            WidgetEntry::new("Checkbox".to_string(), "Input Controls".to_string()),
            WidgetEntry::new("Radio".to_string(), "Input Controls".to_string()),
            WidgetEntry::new("Modal".to_string(), "Layout".to_string()),
            WidgetEntry::new("Card".to_string(), "Display".to_string()),
            WidgetEntry::new("Alert".to_string(), "Display".to_string()),
            WidgetEntry::new("Progress".to_string(), "Display".to_string()),
            WidgetEntry::new("Menu".to_string(), "Navigation".to_string()),
        ]
    }

    pub fn add_widget(&mut self, widget: WidgetEntry) {
        tracing::debug!("WidgetDatabase: Adding widget '{}'", widget.name);
        self.entries.push(widget);
    }

    pub fn get_widget(&self, widget_id: &str) -> Option<WidgetEntry> {
        self.entries
            .iter()
            .find(|w| w.widget_id == widget_id)
            .cloned()
    }

    pub fn get_by_name(&self, name: &str) -> Option<WidgetEntry> {
        self.entries.iter().find(|w| w.name == name).cloned()
    }

    pub fn list_by_category(&self, category: &str) -> Vec<WidgetEntry> {
        self.entries
            .iter()
            .filter(|w| w.category == category)
            .cloned()
            .collect()
    }

    pub fn search(&self, query: &str) -> Vec<WidgetEntry> {
        let query_lower = query.to_lowercase();
        self.entries
            .iter()
            .filter(|w| {
                w.name.to_lowercase().contains(&query_lower)
                    || w.description.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect()
    }

    pub fn list_all(&self) -> Vec<WidgetEntry> {
        self.entries.clone()
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    pub fn get_categories(&self) -> Vec<String> {
        let mut categories: Vec<String> = self
            .entries
            .iter()
            .map(|w| w.category.clone())
            .collect();
        categories.sort();
        categories.dedup();
        categories
    }
}

impl Default for WidgetDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_init() {
        let db = WidgetDatabase::new();
        assert!(db.count() > 0);
    }

    #[test]
    fn test_widget_entry_creation() {
        let widget = WidgetEntry::new("Button".to_string(), "Input".to_string());
        assert_eq!(widget.name, "Button");
    }

    #[test]
    fn test_search_widgets() {
        let db = WidgetDatabase::new();
        let results = db.search("button");
        assert!(results.len() > 0);
    }

    #[test]
    fn test_list_by_category() {
        let db = WidgetDatabase::new();
        let input_controls = db.list_by_category("Input Controls");
        assert!(input_controls.len() > 0);
    }

    #[test]
    fn test_get_categories() {
        let db = WidgetDatabase::new();
        let categories = db.get_categories();
        assert!(categories.len() > 0);
    }

    #[test]
    fn test_math() {
        assert_eq!(2 + 2, 4);
    }
}
