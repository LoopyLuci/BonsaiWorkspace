use tauri::State;
use crate::AppState;

use candle_core::Device;

#[tauri::command]
pub async fn start_go_self_play(
    _state: State<'_, AppState>,
    model_path: String,
    num_games: usize,
) -> Result<String, String> {
    let device = Device::Cpu;
    // Spawn self-play in background
    tokio::spawn(async move {
        if let Err(e) = bonsai_go_nn::train::self_play_loop(&model_path, device, num_games).await {
            tracing::error!("Self-play failed: {}", e);
        }
    });
    Ok(format!("Started self-play with {} games", num_games))
}

#[tauri::command]
pub async fn stop_go_self_play(_state: State<'_, AppState>) -> Result<(), String> {
    // Cancellation not yet implemented — placeholder
    Ok(())
}

#[tauri::command]
pub async fn start_go_training(
    state: State<'_, AppState>,
    model_path: Option<String>,
    num_games_per_cycle: Option<usize>,
) -> Result<String, String> {
    // Build a simple config from args
    let mut cfg = bonsai_go_nn::training_loop::GoTrainingConfig::default();
    if let Some(n) = num_games_per_cycle { cfg.self_play_games_per_cycle = n; }

    let device = Device::cuda_if_available(0).unwrap_or(Device::Cpu);
    let cas = state.cas_store.clone();

    // Spawn the training loop in the background.
    tokio::spawn(async move {
        match bonsai_go_nn::training_loop::GoTrainingLoop::new(cfg, device, None, cas).await {
            Ok(mut loop_inst) => {
                loop {
                    if let Err(e) = loop_inst.run_cycle().await {
                        tracing::error!("go training cycle failed: {}", e);
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
                }
            }
            Err(e) => tracing::error!("failed to start go training: {}", e),
        }
    });

    Ok("Go training loop started".into())
}
