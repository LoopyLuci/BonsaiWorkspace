//! CLI: create a dashboard, add a widget, configure its data source, and
//! publish a real-time update.

use dashboarding_service::{DashboardingService, WidgetType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = DashboardingService::new();

    let dashboard = service.create_dashboard("Ops Overview", "admin").await?;
    println!("created dashboard '{}' (owner: {})", dashboard.name, dashboard.owner);

    let widget = service
        .add_widget(dashboard.dashboard_id, "CPU Utilization", WidgetType::Gauge)
        .await?;
    println!("added widget '{}' ({:?})", widget.title, widget.widget_type);

    let data = service
        .configure_widget_data(widget.widget_id, "prometheus", "avg(cpu_usage_percent)")
        .await?;
    println!("configured data source '{}' with query '{}'", data.data_source, data.query);

    let update = service.publish_update(widget.widget_id, 72.3).await?;
    println!("published update: {} = {}", update.widget_id, update.new_value);

    let widgets = service.list_dashboard_widgets(dashboard.dashboard_id).await?;
    println!("dashboard now has {} widget(s)", widgets.len());
    println!("total dashboards: {}", service.dashboard_count());

    Ok(())
}
