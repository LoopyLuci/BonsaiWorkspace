use crate::{Dashboard, Widget, WidgetType, WidgetData, RealTimeUpdate, DashboardError, DashboardResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct DashboardingService {
    dashboards: Arc<DashMap<Uuid, Dashboard>>,
    widgets: Arc<DashMap<Uuid, Widget>>,
    widget_data: Arc<DashMap<Uuid, WidgetData>>,
    realtime_updates: Arc<DashMap<Uuid, RealTimeUpdate>>,
}

impl DashboardingService {
    pub fn new() -> Self {
        Self {
            dashboards: Arc::new(DashMap::new()),
            widgets: Arc::new(DashMap::new()),
            widget_data: Arc::new(DashMap::new()),
            realtime_updates: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_dashboard(&self, name: &str, owner: &str) -> DashboardResult<Dashboard> {
        let dashboard = Dashboard {
            dashboard_id: Uuid::new_v4(),
            name: name.to_string(),
            owner: owner.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_public: false,
        };

        self.dashboards.insert(dashboard.dashboard_id, dashboard.clone());
        Ok(dashboard)
    }

    pub async fn add_widget(&self, dashboard_id: Uuid, title: &str, widget_type: WidgetType) -> DashboardResult<Widget> {
        if self.dashboards.get(&dashboard_id).is_none() {
            return Err(DashboardError::DashboardNotFound);
        }

        let widget = Widget {
            widget_id: Uuid::new_v4(),
            dashboard_id,
            widget_type,
            title: title.to_string(),
            position_x: 0,
            position_y: 0,
            width: 400,
            height: 300,
        };

        self.widgets.insert(widget.widget_id, widget.clone());
        Ok(widget)
    }

    pub async fn configure_widget_data(&self, widget_id: Uuid, data_source: &str, query: &str) -> DashboardResult<WidgetData> {
        if self.widgets.get(&widget_id).is_none() {
            return Err(DashboardError::WidgetNotFound);
        }

        let widget_data = WidgetData {
            data_id: Uuid::new_v4(),
            widget_id,
            data_source: data_source.to_string(),
            refresh_interval_sec: 30,
            query: query.to_string(),
        };

        self.widget_data.insert(widget_data.data_id, widget_data.clone());
        Ok(widget_data)
    }

    pub async fn publish_update(&self, widget_id: Uuid, new_value: f64) -> DashboardResult<RealTimeUpdate> {
        let update = RealTimeUpdate {
            update_id: Uuid::new_v4(),
            widget_id,
            timestamp: Utc::now(),
            new_value,
        };

        self.realtime_updates.insert(update.update_id, update.clone());
        Ok(update)
    }

    pub async fn get_dashboard(&self, dashboard_id: Uuid) -> DashboardResult<Dashboard> {
        self.dashboards
            .get(&dashboard_id)
            .map(|d| d.clone())
            .ok_or(DashboardError::DashboardNotFound)
    }

    pub async fn list_dashboard_widgets(&self, dashboard_id: Uuid) -> DashboardResult<Vec<Widget>> {
        let mut widgets = Vec::new();

        for entry in self.widgets.iter() {
            if entry.value().dashboard_id == dashboard_id {
                widgets.push(entry.value().clone());
            }
        }

        Ok(widgets)
    }

    pub fn dashboard_count(&self) -> usize {
        self.dashboards.len()
    }
}

impl Default for DashboardingService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_dashboard() {
        let service = DashboardingService::new();
        let dashboard = service.create_dashboard("Sales", "john").await.unwrap();

        assert_eq!(dashboard.name, "Sales");
        assert_eq!(dashboard.owner, "john");
        assert_eq!(service.dashboard_count(), 1);
    }

    #[tokio::test]
    async fn test_add_widget() {
        let service = DashboardingService::new();
        let dashboard = service.create_dashboard("Analytics", "jane").await.unwrap();

        let widget = service
            .add_widget(dashboard.dashboard_id, "Revenue Trend", WidgetType::TimeSeries)
            .await
            .unwrap();

        assert_eq!(widget.widget_type, WidgetType::TimeSeries);
    }

    #[tokio::test]
    async fn test_configure_widget_data() {
        let service = DashboardingService::new();
        let dashboard = service.create_dashboard("KPI", "admin").await.unwrap();

        let widget = service
            .add_widget(dashboard.dashboard_id, "Orders", WidgetType::Gauge)
            .await
            .unwrap();

        let data = service
            .configure_widget_data(widget.widget_id, "database", "SELECT COUNT(*) FROM orders")
            .await
            .unwrap();

        assert_eq!(data.data_source, "database");
    }

    #[tokio::test]
    async fn test_publish_update() {
        let service = DashboardingService::new();
        let dashboard = service.create_dashboard("Live", "ops").await.unwrap();

        let widget = service
            .add_widget(dashboard.dashboard_id, "CPU", WidgetType::Gauge)
            .await
            .unwrap();

        let update = service.publish_update(widget.widget_id, 65.5).await.unwrap();

        assert_eq!(update.new_value, 65.5);
    }
}
