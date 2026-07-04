use sysinfo::System;

/// Unified Resource Value — a normalised measure of a device's compute capacity.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceUrv {
    pub cpu_cores: u32,
    pub cpu_freq_ghz: f64,
    /// 1.0 = modern x86, 0.75 = mobile ARM, 1.15 = Zen4+
    pub cpu_ipc_factor: f64,
    pub ram_gb: f64,
    /// 1.0 = DDR4-3200, 1.3 = DDR5, 0.7 = LPDDR4
    pub ram_bandwidth_factor: f64,
    /// 0.0 if no discrete GPU
    pub gpu_tflops: f64,
    pub gpu_vram_gb: f64,
    pub net_mbps: f64,
    pub power_watts: f64,
}

/// Credits earned per URV per minute.
pub const BASE_RATE: f64 = 0.001;

impl DeviceUrv {
    /// Compute the scalar URV score for this device.
    pub fn score(&self) -> f64 {
        (self.cpu_cores as f64 * self.cpu_freq_ghz * self.cpu_ipc_factor * 3.5
            + self.ram_gb * self.ram_bandwidth_factor * 1.2
            + self.gpu_tflops * 80.0
            + self.gpu_vram_gb * 4.0
            + self.net_mbps * 0.05)
            / 10.0
    }

    /// Auto-detect hardware via `sysinfo`. GPU detection is deferred — defaults to 0.
    pub fn from_sysinfo() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_cores = sys.cpus().len() as u32;
        // sysinfo reports frequency in MHz
        let cpu_freq_ghz = if cpu_cores > 0 {
            sys.cpus()[0].frequency() as f64 / 1000.0
        } else {
            2.5
        };
        // total_memory returns bytes
        let ram_gb = sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0);

        DeviceUrv {
            cpu_cores,
            cpu_freq_ghz,
            cpu_ipc_factor: 1.0,
            ram_gb,
            ram_bandwidth_factor: 1.0,
            gpu_tflops: 0.0,
            gpu_vram_gb: 0.0,
            net_mbps: 0.0,
            power_watts: 0.0,
        }
    }
}

/// Broad device classification based on URV score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DeviceClass {
    Phone,
    Tablet,
    Laptop,
    Desktop,
    HighEndDesktop,
    Server,
    Supercomputer,
}

impl DeviceClass {
    pub fn from_urv(urv: f64) -> DeviceClass {
        if urv < 25.0 {
            DeviceClass::Phone
        } else if urv < 50.0 {
            DeviceClass::Tablet
        } else if urv < 90.0 {
            DeviceClass::Laptop
        } else if urv < 200.0 {
            DeviceClass::Desktop
        } else if urv < 600.0 {
            DeviceClass::HighEndDesktop
        } else if urv < 3000.0 {
            DeviceClass::Server
        } else {
            DeviceClass::Supercomputer
        }
    }
}

/// Bonus multiplier for paid contributors, based on free-tier offer percentage.
///
/// `free_tier_pct` is the percentage of capacity offered to the free pool (0–15).
pub fn paid_bonus_multiplier(free_tier_pct: u8, urv: f64) -> f64 {
    let base = match free_tier_pct {
        0 => 1.00,
        1..=4 => 1.00,
        5..=9 => 1.08,
        10..=14 => 1.15,
        _ => 1.20,
    };
    let class_bonus = if urv >= 2000.0 {
        0.10
    } else if urv >= 500.0 {
        0.05
    } else {
        0.0
    };
    base + class_bonus
}

/// Credits earned per minute given URV score, offered fraction (0.0–1.0),
/// and actual utilisation fraction (0.0–1.0).
pub fn credits_per_minute(urv: f64, offered_pct: f64, utilization: f64) -> f64 {
    urv * offered_pct * utilization * BASE_RATE
}
