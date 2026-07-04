use uuid::Uuid;
use crate::registry::DeviceRegistration;
use credits::urv::BASE_RATE;

/// Filter criteria for browsing the marketplace.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MarketplaceFilter {
    pub min_urv: Option<f64>,
    pub max_price_per_min: Option<f64>,
    pub requires_gpu: bool,
    pub min_reliability: Option<f64>,
}

/// A public listing of a device, augmented with live utilisation data.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceListing {
    pub device_id: Uuid,
    pub display_name: String,
    pub urv: f64,
    pub device_class_str: String,
    pub free_tier_pct: u8,
    pub paid_pct: u8,
    pub price_per_minute: f64,
    pub reliability_score: f64,
    pub is_online: bool,
    pub live_utilization: f64,
}

/// Convert a `DeviceRegistration` to a `DeviceListing` given live utilisation.
pub fn to_listing(reg: &DeviceRegistration, live_util: f64) -> DeviceListing {
    let price_per_minute = reg.urv * BASE_RATE * reg.price_multiplier;
    DeviceListing {
        device_id: reg.device_id,
        display_name: reg.display_name.clone(),
        urv: reg.urv,
        device_class_str: format!("{:?}", reg.device_class),
        free_tier_pct: reg.free_tier_pct,
        paid_pct: reg.paid_pct,
        price_per_minute,
        reliability_score: reg.reliability_score,
        is_online: reg.is_online,
        live_utilization: live_util,
    }
}

/// Apply a filter to a slice of listings and return a sorted (price ascending) subset.
pub fn filter_listings(listings: &[DeviceListing], filter: &MarketplaceFilter) -> Vec<DeviceListing> {
    let mut result: Vec<DeviceListing> = listings
        .iter()
        .filter(|l| {
            if let Some(min_urv) = filter.min_urv {
                if l.urv < min_urv {
                    return false;
                }
            }
            if let Some(max_price) = filter.max_price_per_min {
                if l.price_per_minute > max_price {
                    return false;
                }
            }
            if filter.requires_gpu && l.urv < 1.0 {
                // Without GPU detection, gpu-requiring filter is a best-effort pass-through.
            }
            if let Some(min_rel) = filter.min_reliability {
                if l.reliability_score < min_rel {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();

    result.sort_by(|a, b| {
        a.price_per_minute
            .partial_cmp(&b.price_per_minute)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    result
}
