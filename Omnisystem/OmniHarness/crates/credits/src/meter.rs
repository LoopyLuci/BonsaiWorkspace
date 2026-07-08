use chrono::{DateTime, Utc};
use sysinfo::System;
use tokio::sync::{mpsc, oneshot, watch};

/// A point-in-time resource usage sample.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UsageSample {
    pub timestamp: DateTime<Utc>,
    /// 0.0–1.0
    pub cpu_utilization: f64,
    /// 0.0–1.0
    pub ram_utilization: f64,
    /// 0.0–1.0
    pub gpu_utilization: f64,
    /// Actual network throughput in Mbps
    pub net_mbps_actual: f64,
}

impl Default for UsageSample {
    fn default() -> Self {
        UsageSample {
            timestamp: Utc::now(),
            cpu_utilization: 0.0,
            ram_utilization: 0.0,
            gpu_utilization: 0.0,
            net_mbps_actual: 0.0,
        }
    }
}

/// Commands that can be sent to the `ResourceMeter` actor.
pub enum MeterCmd {
    GetSample(oneshot::Sender<UsageSample>),
    Stop,
}

/// Handle for interacting with a running `ResourceMeter`.
#[derive(Clone)]
pub struct MeterHandle {
    tx: mpsc::Sender<MeterCmd>,
}

impl MeterHandle {
    pub async fn latest_sample(&self) -> UsageSample {
        let (resp_tx, resp_rx) = oneshot::channel();
        let _ = self.tx.send(MeterCmd::GetSample(resp_tx)).await;
        resp_rx.await.unwrap_or_default()
    }

    pub async fn stop(&self) {
        let _ = self.tx.send(MeterCmd::Stop).await;
    }
}

/// Tokio actor that polls sysinfo every 10 seconds and broadcasts the latest sample.
pub struct ResourceMeter;

impl ResourceMeter {
    /// Spawn the meter task. Returns a handle and a watch receiver for the latest sample.
    pub fn spawn() -> (MeterHandle, watch::Receiver<UsageSample>) {
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<MeterCmd>(32);
        let (watch_tx, watch_rx) = watch::channel(UsageSample::default());

        tokio::spawn(async move {
            let mut sys = System::new_all();
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        sys.refresh_all();
                        let sample = build_sample(&sys);
                        let _ = watch_tx.send(sample);
                    }
                    Some(cmd) = cmd_rx.recv() => {
                        match cmd {
                            MeterCmd::GetSample(resp) => {
                                sys.refresh_all();
                                let sample = build_sample(&sys);
                                let _ = watch_tx.send(sample.clone());
                                let _ = resp.send(sample);
                            }
                            MeterCmd::Stop => break,
                        }
                    }
                }
            }
        });

        (MeterHandle { tx: cmd_tx }, watch_rx)
    }
}

fn build_sample(sys: &System) -> UsageSample {
    let cpus = sys.cpus();
    let cpu_util = if cpus.is_empty() {
        0.0
    } else {
        cpus.iter().map(|c| c.cpu_usage() as f64).sum::<f64>() / cpus.len() as f64 / 100.0
    };

    let total_mem = sys.total_memory();
    let used_mem = sys.used_memory();
    let ram_util = if total_mem > 0 {
        used_mem as f64 / total_mem as f64
    } else {
        0.0
    };

    UsageSample {
        timestamp: Utc::now(),
        cpu_utilization: cpu_util,
        ram_utilization: ram_util,
        gpu_utilization: 0.0,   // GPU detection deferred
        net_mbps_actual: 0.0,   // Network sampling deferred
    }
}
