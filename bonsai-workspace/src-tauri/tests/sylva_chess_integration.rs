//! Integration test: Use Sylva to call the `play_move` tool in a chess game.
//! Requires the Tauri app to be built with the `test` feature.

#![cfg(feature = "test")]

use tauri::Manager;
use serde_json::json;

#[tokio::test]
async fn test_sylva_play_move_in_chess() -> Result<(), Box<dyn std::error::Error>> {
    // Create a mock Tauri app with necessary commands registered.
    // In practice, use the real app builder that registers your commands.
    let app = tauri::test::mock_builder()
        .invoke_handler(tauri::generate_handler![
            // Wire up the real commands here when running inside the app
        ])
        .build(tauri::test::mock_context())?;

    // Illustrative only: create a game and run a Sylva script that calls play_move.
    // The test runner should replace below with the actual command names.

    Ok(())
}
