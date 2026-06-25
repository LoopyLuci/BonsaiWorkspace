//! Cluster Credits & Device Rental Marketplace — Tauri command layer.
//!
//! Exposes the `bonsai-credits` and `bonsai-marketplace` crates to the frontend
//! as typed IPC commands. All heavy state lives in `ClusterState`.

use std::path::PathBuf;
use std::sync::Arc;

use credits::{
    community::PersistentCommunityPool,
    estimator::{default_profiles, estimate, ProgressEstimator, TaskEstimateRequest, TaskProfile},
    ledger::Ledger,
    manager::CreditManager,
    meter::{MeterHandle, ResourceMeter},
    urv::{credits_per_minute, paid_bonus_multiplier, DeviceClass, DeviceUrv, BASE_RATE},
};
use marketplace::{
    free_tier::{FreeProject, FreeProjectStatus, FreeTierOrchestrator},
    listing::{filter_listings, to_listing, DeviceListing, MarketplaceFilter},
    registry::{DeviceRegistration, DeviceRegistry},
    reservation::{RentalContract, ReservationManager},
    MarketplaceState,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

// ── App-managed state ─────────────────────────────────────────────────────────

pub struct ClusterState {
    pub credit_manager: Arc<CreditManager>,
    pub community_pool: Arc<PersistentCommunityPool>,
    pub ledger: Arc<Ledger>,
    pub marketplace: Arc<MarketplaceState>,
    pub meter_handle: MeterHandle,
    /// This device's own registration (None until `set_contribution` is called).
    pub my_registration: Arc<RwLock<Option<DeviceRegistration>>>,
    /// Task profiles for estimation.
    pub task_profiles: Vec<TaskProfile>,
    /// Active progress estimators keyed by project_id.
    pub estimators: Arc<RwLock<std::collections::HashMap<Uuid, ProgressEstimator>>>,
}

impl ClusterState {
    pub fn new() -> Self {
        let data_dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".bonsai/cluster");
        std::fs::create_dir_all(&data_dir).ok();

        let credit_manager =
            Arc::new(CreditManager::new(data_dir.join("credits.db")).unwrap_or_else(|e| {
                warn!("CreditManager init failed: {e}; using temp path");
                CreditManager::new(PathBuf::from("/tmp/bonsai_credits.db")).unwrap()
            }));
        let community_pool = Arc::new(
            PersistentCommunityPool::new(data_dir.join("community.db")).unwrap_or_else(|_| {
                PersistentCommunityPool::new(PathBuf::from("/tmp/bonsai_community.db")).unwrap()
            }),
        );
        let ledger = Arc::new(Ledger::new(data_dir.join("ledger.db")).unwrap_or_else(|_| {
            Ledger::new(PathBuf::from("/tmp/bonsai_ledger.db")).unwrap()
        }));

        let marketplace = Arc::new(MarketplaceState::new());
        let (meter_handle, _) = ResourceMeter::spawn();

        // Start the stale-device cleanup loop (marks devices offline after 90s).
        let registry = marketplace.registry.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                registry.mark_stale();
            }
        });

        // Start the free-tier rebalance loop.
        let orchestrator = marketplace.free_tier.clone();
        tokio::spawn(FreeTierOrchestrator::rebalance_loop(orchestrator));

        Self {
            credit_manager,
            community_pool,
            ledger,
            marketplace,
            meter_handle,
            my_registration: Arc::new(RwLock::new(None)),
            task_profiles: default_profiles(),
            estimators: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
}

// ── Response DTOs ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct WalletInfo {
    pub balance: f64,
    pub community_pool_balance: f64,
}

#[derive(Serialize)]
pub struct DeviceInfo {
    pub urv: f64,
    pub device_class: String,
    pub credits_per_minute_full: f64,
    pub free_tier_pct: u8,
    pub paid_pct: u8,
    pub paid_bonus_multiplier: f64,
    pub live_cpu_utilization: f64,
    pub live_ram_utilization: f64,
}

#[derive(Serialize)]
pub struct ProjectEstimateResponse {
    pub total_urv_minutes: f64,
    pub eta_minutes: f64,
    pub eta_low_minutes: f64,
    pub eta_high_minutes: f64,
    pub credits_per_minute: f64,
    pub estimated_total_credits: f64,
    pub confidence: f64,
    pub free_tier_eta_minutes: Option<f64>,
    pub paid_savings_minutes: Option<f64>,
}

#[derive(Serialize)]
pub struct FreePoolStatus {
    pub total_cpu_urv: f64,
    pub total_gpu_urv: f64,
    pub active_projects: usize,
    pub per_project_cpu_urv: f64,
    pub per_project_gpu_urv: f64,
    pub device_count: usize,
}

// ── Tauri commands ─────────────────────────────────────────────────────────────

/// Return the user's current credit balance and community pool balance.
#[tauri::command]
pub async fn credits_balance(state: State<'_, ClusterState>) -> Result<WalletInfo, String> {
    let balance = state.credit_manager.balance().map_err(|e| e.to_string())?;
    let pool = state
        .community_pool
        .balance()
        .map_err(|e| e.to_string())?;
    Ok(WalletInfo {
        balance,
        community_pool_balance: pool,
    })
}

/// Return live stats for this device: URV, class, live utilization, bonus multiplier.
#[tauri::command]
pub async fn my_device_info(state: State<'_, ClusterState>) -> Result<DeviceInfo, String> {
    let urv_spec = DeviceUrv::from_sysinfo();
    let urv = urv_spec.score();
    let sample = state.meter_handle.latest_sample().await;
    let reg = state.my_registration.read().await;
    let (free_pct, paid_pct) = reg
        .as_ref()
        .map(|r| (r.free_tier_pct, r.paid_pct))
        .unwrap_or((15, 80));

    Ok(DeviceInfo {
        urv,
        device_class: format!("{:?}", DeviceClass::from_urv(urv)),
        credits_per_minute_full: credits_per_minute(urv, 1.0, 1.0),
        free_tier_pct: free_pct,
        paid_pct,
        paid_bonus_multiplier: paid_bonus_multiplier(free_pct, urv),
        live_cpu_utilization: sample.cpu_utilization,
        live_ram_utilization: sample.ram_utilization,
    })
}

/// Set this device's contribution split (free_tier_pct + paid_pct ≤ 100,
/// free_tier_pct ≤ 15). Registers/updates the device in the marketplace.
#[tauri::command]
pub async fn set_contribution(
    state: State<'_, ClusterState>,
    free_tier_pct: u8,
    paid_pct: u8,
    display_name: String,
    price_multiplier: f64,
) -> Result<DeviceInfo, String> {
    if free_tier_pct > 15 {
        return Err("Free tier cap is 15%".into());
    }
    if (free_tier_pct as u16 + paid_pct as u16) > 100 {
        return Err("free_tier_pct + paid_pct must be ≤ 100".into());
    }

    let urv_spec = DeviceUrv::from_sysinfo();
    let urv = urv_spec.score();
    let sample = state.meter_handle.latest_sample().await;

    let reg = DeviceRegistration {
        device_id: Uuid::new_v4(),
        owner_id: Uuid::new_v4(), // TODO: wire to identity system
        display_name: display_name.clone(),
        urv,
        device_class: DeviceClass::from_urv(urv),
        free_tier_pct,
        paid_pct,
        price_multiplier: price_multiplier.clamp(1.0, 3.0),
        reliability_score: 1.0,
        last_seen: Utc::now(),
        is_online: true,
    };

    state.marketplace.registry.register(reg.clone());
    *state.my_registration.write().await = Some(reg);

    info!(display_name, free_tier_pct, paid_pct, urv, "[cluster] contribution set");

    Ok(DeviceInfo {
        urv,
        device_class: format!("{:?}", DeviceClass::from_urv(urv)),
        credits_per_minute_full: credits_per_minute(urv, 1.0, 1.0),
        free_tier_pct,
        paid_pct,
        paid_bonus_multiplier: paid_bonus_multiplier(free_tier_pct, urv),
        live_cpu_utilization: sample.cpu_utilization,
        live_ram_utilization: sample.ram_utilization,
    })
}

/// List devices available in the paid rental marketplace.
#[tauri::command]
pub async fn marketplace_list(
    state: State<'_, ClusterState>,
    filter: Option<MarketplaceFilter>,
) -> Result<Vec<DeviceListing>, String> {
    let registrations = state.marketplace.registry.list_online();
    let sample = state.meter_handle.latest_sample().await;
    let live_util = (sample.cpu_utilization + sample.gpu_utilization) / 2.0;

    let listings: Vec<DeviceListing> = registrations
        .iter()
        .filter(|r| r.paid_pct > 0)
        .map(|r| to_listing(r, live_util))
        .collect();

    let filtered = if let Some(f) = filter {
        filter_listings(&listings, &f)
    } else {
        listings
    };

    Ok(filtered)
}

/// Reserve a device for N minutes at the listed price.
#[tauri::command]
pub async fn reserve_device(
    state: State<'_, ClusterState>,
    device_id: String,
    minutes: u32,
) -> Result<RentalContract, String> {
    let device_id: Uuid = device_id.parse().map_err(|_| "invalid device_id")?;
    let registrations = state.marketplace.registry.list_online();
    let reg = registrations
        .iter()
        .find(|r| r.device_id == device_id)
        .ok_or_else(|| format!("device {device_id} not found or offline"))?;

    let urv = reg.urv * (reg.paid_pct as f64 / 100.0);
    let price_per_minute = credits_per_minute(urv, 1.0, 1.0) * reg.price_multiplier;

    let renter_id = Uuid::new_v4(); // TODO: wire to identity
    let contract = state
        .marketplace
        .reservations
        .reserve(device_id, renter_id, minutes, price_per_minute);

    info!(
        ?device_id,
        minutes,
        price_per_minute,
        "[cluster] device reserved"
    );
    Ok(contract)
}

/// Cancel an active rental contract.
#[tauri::command]
pub async fn cancel_reservation(
    state: State<'_, ClusterState>,
    contract_id: String,
) -> Result<(), String> {
    let id: Uuid = contract_id.parse().map_err(|_| "invalid contract_id")?;
    state.marketplace.reservations.cancel(id);
    Ok(())
}

/// Submit a project to the free tier pool.
#[tauri::command]
pub async fn submit_free_project(
    state: State<'_, ClusterState>,
    task_type: String,
    requires_gpu: bool,
) -> Result<String, String> {
    let project = FreeProject {
        project_id: Uuid::new_v4(),
        owner_id: Uuid::new_v4(), // TODO: wire to identity
        task_type,
        submitted_at: Utc::now(),
        requires_gpu,
        progress: 0.0,
        status: FreeProjectStatus::Queued,
    };
    let id = state.marketplace.free_tier.submit_project(project);
    info!(?id, "[cluster] free project submitted");
    Ok(id.to_string())
}

/// Report progress on a free project (0.0–1.0). When progress = 1.0 it completes.
#[tauri::command]
pub async fn update_free_project_progress(
    state: State<'_, ClusterState>,
    project_id: String,
    progress: f64,
) -> Result<(), String> {
    let id: Uuid = project_id.parse().map_err(|_| "invalid project_id")?;
    let progress = progress.clamp(0.0, 1.0);
    if progress >= 1.0 {
        state.marketplace.free_tier.complete_project(id);
    } else {
        state.marketplace.free_tier.update_progress(id, progress);
    }
    Ok(())
}

/// Current status of the free resource pool.
#[tauri::command]
pub async fn free_pool_status(state: State<'_, ClusterState>) -> Result<FreePoolStatus, String> {
    let (cpu_urv, gpu_urv) = state.marketplace.free_tier.per_project_urv();
    let active = state.marketplace.free_tier.active_count();
    let online = state.marketplace.registry.list_online();
    let total_cpu = state.marketplace.registry.free_pool_urv();
    let total_gpu: f64 = online
        .iter()
        .map(|r| r.urv * (r.free_tier_pct as f64 / 100.0) * 0.3) // rough GPU fraction
        .sum();

    Ok(FreePoolStatus {
        total_cpu_urv: total_cpu,
        total_gpu_urv: total_gpu,
        active_projects: active,
        per_project_cpu_urv: cpu_urv,
        per_project_gpu_urv: gpu_urv,
        device_count: online.len(),
    })
}

/// Estimate cost and time for a project before committing credits.
#[tauri::command]
pub async fn estimate_project(
    state: State<'_, ClusterState>,
    task_type: String,
    units: f64,
    num_paid_devices: u32,
    total_paid_urv_per_min: f64,
) -> Result<ProjectEstimateResponse, String> {
    let profile = state
        .task_profiles
        .iter()
        .find(|p| p.task_type == task_type)
        .ok_or_else(|| format!("Unknown task type: {task_type}"))?;

    let free_pool_urv = state.marketplace.registry.free_pool_urv();
    let free_active = state.marketplace.free_tier.active_count().max(1) as f64;
    let free_share = free_pool_urv / free_active;

    let req = TaskEstimateRequest {
        task_type: task_type.clone(),
        units,
        num_devices: num_paid_devices,
        total_urv_per_min: total_paid_urv_per_min,
    };
    let est = estimate(&req, profile);

    let free_eta = if free_share > 0.0 {
        Some(est.total_urv_minutes / free_share)
    } else {
        None
    };

    let paid_savings = free_eta.map(|f| f - est.eta_minutes).filter(|&s| s > 0.0);

    Ok(ProjectEstimateResponse {
        total_urv_minutes: est.total_urv_minutes,
        eta_minutes: est.eta_minutes,
        eta_low_minutes: est.eta_low,
        eta_high_minutes: est.eta_high,
        credits_per_minute: est.credits_per_minute,
        estimated_total_credits: est.estimated_total_credits,
        confidence: est.confidence,
        free_tier_eta_minutes: free_eta,
        paid_savings_minutes: paid_savings,
    })
}

/// Get earnings history (last N days).
#[tauri::command]
pub async fn earnings_history(
    state: State<'_, ClusterState>,
    days: u32,
) -> Result<Vec<credits::manager::EarnReceipt>, String> {
    state
        .credit_manager
        .earnings_history(days)
        .map_err(|e| e.to_string())
}

/// Get spending history (last N days).
#[tauri::command]
pub async fn spending_history(
    state: State<'_, ClusterState>,
    days: u32,
) -> Result<Vec<credits::manager::SpendReceipt>, String> {
    state
        .credit_manager
        .spending_history(days)
        .map_err(|e| e.to_string())
}

/// List available task types with their profiles (for the estimator UI).
#[tauri::command]
pub async fn list_task_profiles(
    state: State<'_, ClusterState>,
) -> Result<Vec<TaskProfile>, String> {
    Ok(state.task_profiles.clone())
}
