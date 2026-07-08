//! User input simulation and replay

use crate::errors::ApplicationStressResult;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Input event types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum InputEventType {
    KeyPress { key: char },
    KeyRelease { key: char },
    MouseMove { x: i32, y: i32 },
    MouseClick { button: String },
    MouseScroll { delta: i32 },
    Paste { text: String },
}

/// User input event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputEvent {
    pub event_type: InputEventType,
    pub timestamp: DateTime<Utc>,
    pub window_id: String,
}

impl InputEvent {
    /// Create a new input event
    pub fn new(event_type: InputEventType, window_id: impl Into<String>) -> Self {
        Self {
            event_type,
            timestamp: Utc::now(),
            window_id: window_id.into(),
        }
    }

    /// Create a key press event
    pub fn key_press(key: char, window_id: impl Into<String>) -> Self {
        Self::new(InputEventType::KeyPress { key }, window_id)
    }

    /// Create a mouse click event
    pub fn mouse_click(button: impl Into<String>, window_id: impl Into<String>) -> Self {
        Self::new(InputEventType::MouseClick { button: button.into() }, window_id)
    }

    /// Create a paste event
    pub fn paste(text: impl Into<String>, window_id: impl Into<String>) -> Self {
        Self::new(InputEventType::Paste { text: text.into() }, window_id)
    }
}

/// Deterministic input simulator
pub struct InputSimulator {
    events: Arc<RwLock<Vec<InputEvent>>>,
    replay_index: Arc<RwLock<usize>>,
}

impl InputSimulator {
    /// Create a new input simulator
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            replay_index: Arc::new(RwLock::new(0)),
        }
    }

    /// Record an input event
    pub async fn record_event(&self, event: InputEvent) -> ApplicationStressResult<()> {
        self.events.write().await.push(event);
        Ok(())
    }

    /// Record multiple events
    pub async fn record_events(&self, events: Vec<InputEvent>) -> ApplicationStressResult<()> {
        self.events.write().await.extend(events);
        Ok(())
    }

    /// Get recorded events
    pub async fn get_events(&self) -> Vec<InputEvent> {
        self.events.read().await.clone()
    }

    /// Replay recorded events
    pub async fn replay(&self) -> ApplicationStressResult<usize> {
        let events = self.events.read().await.clone();
        let mut index = self.replay_index.write().await;

        let mut count = 0;
        for _event in events {
            // Simulate event processing
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            count += 1;
            *index += 1;
        }

        Ok(count)
    }

    /// Clear events
    pub async fn clear(&self) -> ApplicationStressResult<()> {
        self.events.write().await.clear();
        *self.replay_index.write().await = 0;
        Ok(())
    }

    /// Get replay progress
    pub async fn progress(&self) -> (usize, usize) {
        let total = self.events.read().await.len();
        let index = *self.replay_index.read().await;
        (index, total)
    }
}

impl Default for InputSimulator {
    fn default() -> Self {
        Self::new()
    }
}

/// User simulation session
#[derive(Debug, Clone)]
pub struct UserSimulation {
    pub user_id: String,
    pub session_duration_secs: u64,
    pub activity_rate_hz: f64,
    pub think_time_ms: u64,
}

impl UserSimulation {
    /// Create a new user simulation
    pub fn new(
        user_id: impl Into<String>,
        session_duration_secs: u64,
        activity_rate_hz: f64,
    ) -> Self {
        Self {
            user_id: user_id.into(),
            session_duration_secs,
            activity_rate_hz,
            think_time_ms: (1000.0 / activity_rate_hz) as u64,
        }
    }

    /// Simulate typing activity
    pub async fn simulate_typing(&self, words: usize) -> ApplicationStressResult<Vec<InputEvent>> {
        let mut events = Vec::new();

        for _ in 0..words {
            // Simulate word (5 characters average)
            for _ in 0..5 {
                events.push(InputEvent::key_press('a', &self.user_id));
            }

            // Space between words
            events.push(InputEvent::key_press(' ', &self.user_id));

            // Think time
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        Ok(events)
    }

    /// Simulate clicking activity
    pub async fn simulate_clicks(&self, count: usize) -> ApplicationStressResult<Vec<InputEvent>> {
        let mut events = Vec::new();

        for _ in 0..count {
            events.push(InputEvent::mouse_click("left", &self.user_id));

            // Time between clicks
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Ok(events)
    }

    /// Simulate scrolling
    pub async fn simulate_scrolling(
        &self,
        scroll_count: usize,
    ) -> ApplicationStressResult<Vec<InputEvent>> {
        let mut events = Vec::new();

        for i in 0..scroll_count {
            events.push(InputEvent::new(
                InputEventType::MouseScroll {
                    delta: if i % 2 == 0 { 5 } else { -5 },
                },
                &self.user_id,
            ));

            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        Ok(events)
    }

    /// Simulate a complete user session
    pub async fn simulate_session(
        &self,
        simulator: &InputSimulator,
    ) -> ApplicationStressResult<usize> {
        // Simulate realistic user behavior
        let typing_events = self.simulate_typing(100).await?;
        simulator.record_events(typing_events).await?;

        let click_events = self.simulate_clicks(50).await?;
        simulator.record_events(click_events).await?;

        let scroll_events = self.simulate_scrolling(30).await?;
        simulator.record_events(scroll_events).await?;

        // Replay all events
        simulator.replay().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_event_creation() {
        let event = InputEvent::key_press('a', "window-1");
        assert_eq!(event.window_id, "window-1");
    }

    #[tokio::test]
    async fn test_input_simulator_recording() {
        let simulator = InputSimulator::new();
        let event = InputEvent::key_press('a', "window-1");
        simulator.record_event(event).await.unwrap();

        let events = simulator.get_events().await;
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn test_input_simulator_replay() {
        let simulator = InputSimulator::new();
        simulator
            .record_event(InputEvent::key_press('a', "window-1"))
            .await
            .unwrap();
        simulator
            .record_event(InputEvent::key_press('b', "window-1"))
            .await
            .unwrap();

        let count = simulator.replay().await.unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_user_simulation_creation() {
        let sim = UserSimulation::new("user-1", 60, 10.0);
        assert_eq!(sim.user_id, "user-1");
        assert_eq!(sim.think_time_ms, 100);
    }

    #[tokio::test]
    async fn test_user_typing_simulation() {
        let sim = UserSimulation::new("user-1", 60, 10.0);
        let events = sim.simulate_typing(5).await.unwrap();
        assert!(events.len() >= 5); // At least one event per word
    }

    #[tokio::test]
    async fn test_user_click_simulation() {
        let sim = UserSimulation::new("user-1", 60, 10.0);
        let events = sim.simulate_clicks(3).await.unwrap();
        assert_eq!(events.len(), 3);
    }
}
