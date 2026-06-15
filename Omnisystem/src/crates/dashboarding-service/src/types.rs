use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dashboard {
    pub dashboard_id: Uuid,
    pub name: String,
    pub owner: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_public: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Widget {
    pub widget_id: Uuid,
    pub dashboard_id: Uuid,
    pub widget_type: WidgetType,
    pub title: String,
    pub position_x: u32,
    pub position_y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum WidgetType {
    TimeSeries,
    BarChart,
    PieChart,
    Table,
    Gauge,
    Heatmap,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WidgetData {
    pub data_id: Uuid,
    pub widget_id: Uuid,
    pub data_source: String,
    pub refresh_interval_sec: u32,
    pub query: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RealTimeUpdate {
    pub update_id: Uuid,
    pub widget_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub new_value: f64,
}
