use crate::{HealthResult, HealthStatus};
use observability_core::{SpanId, TraceId};
use service_mesh_core::{EndpointId, ServiceId};
use std::collections::HashMap;

pub struct HealthObservabilityBridge {
    span_count: u64,
    trace_count: u64,
}

impl HealthObservabilityBridge {
    pub fn new() -> Self {
        Self {
            span_count: 0,
            trace_count: 0,
        }
    }

    pub fn record_health_event(
        &mut self,
        service_id: &ServiceId,
        endpoint_id: &EndpointId,
        status: HealthStatus,
        duration_ms: u32,
        error: Option<String>,
    ) -> HealthResult<()> {
        self.span_count += 1;

        let _event_name = match status {
            HealthStatus::Healthy => "health_check_success",
            HealthStatus::Unhealthy => "health_check_failure",
            HealthStatus::Unknown => "health_check_unknown",
        };

        let mut attributes = HashMap::new();
        attributes.insert("service_id".to_string(), service_id.0.clone());
        attributes.insert("endpoint_id".to_string(), endpoint_id.0.clone());
        attributes.insert("duration_ms".to_string(), duration_ms.to_string());
        attributes.insert("status".to_string(), format!("{:?}", status));

        if let Some(err) = error {
            attributes.insert("error".to_string(), err);
        }

        Ok(())
    }

    pub fn create_trace_for_service(&mut self, _service_id: &ServiceId) -> HealthResult<TraceId> {
        self.trace_count += 1;
        let trace_id = TraceId(format!("health-trace-{}", self.trace_count));
        Ok(trace_id)
    }

    pub fn create_span_for_endpoint(
        &mut self,
        _service_id: &ServiceId,
        _endpoint_id: &EndpointId,
    ) -> HealthResult<SpanId> {
        self.span_count += 1;
        let span_id = SpanId(format!("health-span-{}", self.span_count));
        Ok(span_id)
    }

    pub fn get_span_count(&self) -> u64 {
        self.span_count
    }

    pub fn get_trace_count(&self) -> u64 {
        self.trace_count
    }
}

impl Default for HealthObservabilityBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_health_event() {
        let mut bridge = HealthObservabilityBridge::new();
        let service_id = ServiceId("api-service".to_string());
        let endpoint_id = EndpointId("endpoint-1".to_string());

        let result = bridge.record_health_event(&service_id, &endpoint_id, HealthStatus::Healthy, 50, None);
        assert!(result.is_ok());
        assert_eq!(bridge.get_span_count(), 1);
    }

    #[test]
    fn test_record_health_event_with_error() {
        let mut bridge = HealthObservabilityBridge::new();
        let service_id = ServiceId("api-service".to_string());
        let endpoint_id = EndpointId("endpoint-1".to_string());

        let result = bridge.record_health_event(
            &service_id,
            &endpoint_id,
            HealthStatus::Unhealthy,
            100,
            Some("Connection timeout".to_string()),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_trace_for_service() {
        let mut bridge = HealthObservabilityBridge::new();
        let service_id = ServiceId("api-service".to_string());

        let trace1 = bridge.create_trace_for_service(&service_id).unwrap();
        let trace2 = bridge.create_trace_for_service(&service_id).unwrap();

        assert_ne!(trace1.0, trace2.0);
        assert_eq!(bridge.get_trace_count(), 2);
    }

    #[test]
    fn test_create_span_for_endpoint() {
        let mut bridge = HealthObservabilityBridge::new();
        let service_id = ServiceId("api-service".to_string());
        let endpoint_id = EndpointId("endpoint-1".to_string());

        let span1 = bridge.create_span_for_endpoint(&service_id, &endpoint_id).unwrap();
        let span2 = bridge.create_span_for_endpoint(&service_id, &endpoint_id).unwrap();

        assert_ne!(span1.0, span2.0);
        assert_eq!(bridge.get_span_count(), 2);
    }

    #[test]
    fn test_multiple_operations() {
        let mut bridge = HealthObservabilityBridge::new();
        let service_id = ServiceId("api-service".to_string());
        let endpoint_id = EndpointId("endpoint-1".to_string());

        for _ in 0..10 {
            bridge
                .record_health_event(&service_id, &endpoint_id, HealthStatus::Healthy, 50, None)
                .unwrap();
        }

        assert_eq!(bridge.get_span_count(), 10);
    }
}
