use serde::{Deserialize, Serialize};

/// Advanced widgets - Phase 4

/// Data table component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTable {
    pub table_id: String,
    pub columns: Vec<ColumnDef>,
    pub rows: Vec<Vec<String>>,
    pub sortable: bool,
    pub filterable: bool,
    pub page_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: String,
    pub sortable: bool,
    pub width: Option<String>,
}

impl DataTable {
    pub fn new() -> Self {
        DataTable {
            table_id: uuid::Uuid::new_v4().to_string(),
            columns: vec![],
            rows: vec![],
            sortable: true,
            filterable: true,
            page_size: 25,
        }
    }

    pub fn add_column(&mut self, column: ColumnDef) {
        self.columns.push(column);
    }

    pub fn add_row(&mut self, row: Vec<String>) {
        self.rows.push(row);
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
}

impl Default for DataTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Chart component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chart {
    pub chart_id: String,
    pub chart_type: ChartType,
    pub title: String,
    pub data: ChartData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChartType {
    Line,
    Bar,
    Pie,
    Area,
    Scatter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartData {
    pub labels: Vec<String>,
    pub datasets: Vec<ChartDataset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartDataset {
    pub label: String,
    pub data: Vec<f32>,
    pub color: String,
}

impl Chart {
    pub fn new(chart_type: ChartType, title: String) -> Self {
        Chart {
            chart_id: uuid::Uuid::new_v4().to_string(),
            chart_type,
            title,
            data: ChartData {
                labels: vec![],
                datasets: vec![],
            },
        }
    }
}

/// Notification component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub notification_id: String,
    pub title: String,
    pub message: String,
    pub notification_type: NotificationType,
    pub duration_ms: u32,
    pub action_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotificationType {
    Info,
    Success,
    Warning,
    Error,
}

impl Notification {
    pub fn new(title: String, message: String, notification_type: NotificationType) -> Self {
        Notification {
            notification_id: uuid::Uuid::new_v4().to_string(),
            title,
            message,
            notification_type,
            duration_ms: 5000,
            action_url: None,
        }
    }
}

/// Rich text editor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RichTextEditor {
    pub editor_id: String,
    pub content: String,
    pub toolbar: bool,
    pub read_only: bool,
}

impl RichTextEditor {
    pub fn new() -> Self {
        RichTextEditor {
            editor_id: uuid::Uuid::new_v4().to_string(),
            content: String::new(),
            toolbar: true,
            read_only: false,
        }
    }
}

impl Default for RichTextEditor {
    fn default() -> Self {
        Self::new()
    }
}

/// File picker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePicker {
    pub picker_id: String,
    pub multiple: bool,
    pub accept: Vec<String>,
    pub selected_files: Vec<FileSelection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSelection {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
}

impl FilePicker {
    pub fn new() -> Self {
        FilePicker {
            picker_id: uuid::Uuid::new_v4().to_string(),
            multiple: false,
            accept: vec![],
            selected_files: vec![],
        }
    }
}

impl Default for FilePicker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_table() {
        let table = DataTable::new();
        assert_eq!(table.row_count(), 0);
    }

    #[test]
    fn test_chart() {
        let chart = Chart::new(ChartType::Line, "CPU Usage".to_string());
        assert_eq!(chart.chart_type, ChartType::Line);
    }

    #[test]
    fn test_notification() {
        let notif = Notification::new(
            "Success".to_string(),
            "Operation completed".to_string(),
            NotificationType::Success,
        );
        assert_eq!(notif.notification_type, NotificationType::Success);
    }

    #[test]
    fn test_rich_text_editor() {
        let editor = RichTextEditor::new();
        assert!(!editor.read_only);
    }

    #[test]
    fn test_file_picker() {
        let picker = FilePicker::new();
        assert!(!picker.multiple);
    }

    #[test]
    fn test_math() {
        assert_eq!(2 + 2, 4);
    }
}
