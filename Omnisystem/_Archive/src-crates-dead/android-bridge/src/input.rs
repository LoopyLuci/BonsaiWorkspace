use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Input event type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InputEventType {
    /// Touch event
    Touch,
    /// Keyboard event
    Keyboard,
    /// Mouse/pointer event
    Pointer,
}

/// Touch action
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TouchAction {
    Down,
    Move,
    Up,
    Cancel,
}

/// Key action
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum KeyAction {
    Press,
    Release,
}

/// Keyboard modifiers
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

/// Touch input event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouchEvent {
    /// X coordinate
    pub x: f32,
    /// Y coordinate
    pub y: f32,
    /// Touch action
    pub action: TouchAction,
    /// Touch ID (for multi-touch)
    pub pointer_id: u32,
    /// Pressure (0.0 - 1.0)
    pub pressure: f32,
    /// Size (0.0 - 1.0)
    pub size: f32,
}

/// Keyboard input event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardEvent {
    /// Key code (Android KeyEvent.KEYCODE_*)
    pub key_code: u32,
    /// Unicode character (if printable)
    pub char_code: Option<char>,
    /// Key action
    pub action: KeyAction,
    /// Modifiers
    pub modifiers: Modifiers,
    /// Repeat count
    pub repeat_count: u32,
}

/// Pointer/mouse input event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointerEvent {
    /// X coordinate
    pub x: f32,
    /// Y coordinate
    pub y: f32,
    /// Button index (0=left, 1=right, 2=middle)
    pub button: u32,
    /// Button pressed
    pub pressed: bool,
    /// Scroll delta Y
    pub scroll_y: Option<f32>,
}

/// Generic input event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputEvent {
    /// Event timestamp (microseconds)
    pub timestamp_us: u64,
    /// Event ID (for acking)
    pub id: u32,
    /// Event type
    pub event_type: InputEventType,
    /// Touch event data
    pub touch: Option<TouchEvent>,
    /// Keyboard event data
    pub keyboard: Option<KeyboardEvent>,
    /// Pointer event data
    pub pointer: Option<PointerEvent>,
}

impl InputEvent {
    /// Create touch event
    pub fn touch(x: f32, y: f32, action: TouchAction, pointer_id: u32) -> Self {
        Self {
            timestamp_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
            id: 0,
            event_type: InputEventType::Touch,
            touch: Some(TouchEvent {
                x,
                y,
                action,
                pointer_id,
                pressure: 1.0,
                size: 1.0,
            }),
            keyboard: None,
            pointer: None,
        }
    }

    /// Create keyboard event
    pub fn keyboard(key_code: u32, action: KeyAction) -> Self {
        Self {
            timestamp_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
            id: 0,
            event_type: InputEventType::Keyboard,
            touch: None,
            keyboard: Some(KeyboardEvent {
                key_code,
                char_code: None,
                action,
                modifiers: Modifiers::default(),
                repeat_count: 0,
            }),
            pointer: None,
        }
    }

    /// Create pointer event
    pub fn pointer(x: f32, y: f32, button: u32, pressed: bool) -> Self {
        Self {
            timestamp_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
            id: 0,
            event_type: InputEventType::Pointer,
            touch: None,
            keyboard: None,
            pointer: Some(PointerEvent {
                x,
                y,
                button,
                pressed,
                scroll_y: None,
            }),
        }
    }
}

/// Input event injector
pub struct InputInjector {
    /// Event queue
    event_queue: tokio::sync::mpsc::UnboundedSender<InputEvent>,
    /// Event counter
    event_counter: Arc<parking_lot::Mutex<u32>>,
}

impl InputInjector {
    /// Create new input injector
    pub fn new(event_queue: tokio::sync::mpsc::UnboundedSender<InputEvent>) -> Self {
        Self {
            event_queue,
            event_counter: Arc::new(parking_lot::Mutex::new(0)),
        }
    }

    /// Inject input event
    pub async fn inject(&self, mut event: InputEvent) -> Result<()> {
        let mut counter = self.event_counter.lock();
        *counter = counter.wrapping_add(1);
        event.id = *counter;

        self.event_queue.send(event).map_err(|e| {
            crate::error::Error::InputError(e.to_string())
        })
    }

    /// Inject touch down
    pub async fn touch_down(&self, x: f32, y: f32, pointer_id: u32) -> Result<()> {
        self.inject(InputEvent::touch(x, y, TouchAction::Down, pointer_id))
            .await
    }

    /// Inject touch move
    pub async fn touch_move(&self, x: f32, y: f32, pointer_id: u32) -> Result<()> {
        self.inject(InputEvent::touch(x, y, TouchAction::Move, pointer_id))
            .await
    }

    /// Inject touch up
    pub async fn touch_up(&self, x: f32, y: f32, pointer_id: u32) -> Result<()> {
        self.inject(InputEvent::touch(x, y, TouchAction::Up, pointer_id))
            .await
    }

    /// Inject key press
    pub async fn key_press(&self, key_code: u32) -> Result<()> {
        self.inject(InputEvent::keyboard(key_code, KeyAction::Press))
            .await
    }

    /// Inject key release
    pub async fn key_release(&self, key_code: u32) -> Result<()> {
        self.inject(InputEvent::keyboard(key_code, KeyAction::Release))
            .await
    }

    /// Inject text (via keyboard)
    pub async fn inject_text(&self, text: &str) -> Result<()> {
        for ch in text.chars() {
            let mut event = InputEvent {
                timestamp_us: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_micros() as u64,
                id: 0,
                event_type: InputEventType::Keyboard,
                touch: None,
                keyboard: Some(KeyboardEvent {
                    key_code: 0, // Not used for char input
                    char_code: Some(ch),
                    action: KeyAction::Press,
                    modifiers: Modifiers::default(),
                    repeat_count: 0,
                }),
                pointer: None,
            };

            let mut counter = self.event_counter.lock();
            *counter = counter.wrapping_add(1);
            event.id = *counter;

            self.event_queue.send(event).map_err(|e| {
                crate::error::Error::InputError(e.to_string())
            })?;
        }
        Ok(())
    }

    /// Inject click
    pub async fn click(&self, x: f32, y: f32) -> Result<()> {
        self.inject(InputEvent::pointer(x, y, 0, true)).await?;
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        self.inject(InputEvent::pointer(x, y, 0, false)).await
    }

    /// Inject swipe
    pub async fn swipe(&self, x1: f32, y1: f32, x2: f32, y2: f32, duration_ms: u64) -> Result<()> {
        let steps = 10.max(duration_ms / 16); // ~16ms per frame
        let step_x = (x2 - x1) / steps as f32;
        let step_y = (y2 - y1) / steps as f32;

        self.touch_down(x1, y1, 0).await?;

        for i in 1..steps {
            let x = x1 + step_x * i as f32;
            let y = y1 + step_y * i as f32;
            self.touch_move(x, y, 0).await?;
            tokio::time::sleep(std::time::Duration::from_millis(duration_ms / steps))
                .await;
        }

        self.touch_up(x2, y2, 0).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_event_creation() {
        let event = InputEvent::touch(100.0, 200.0, TouchAction::Down, 0);
        assert_eq!(event.event_type, InputEventType::Touch);
        assert!(event.touch.is_some());
    }

    #[test]
    fn test_keyboard_event() {
        let event = InputEvent::keyboard(29, KeyAction::Press);
        assert_eq!(event.event_type, InputEventType::Keyboard);
        assert!(event.keyboard.is_some());
    }

    #[tokio::test]
    async fn test_input_injector() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let injector = InputInjector::new(tx);

        assert!(injector.touch_down(100.0, 200.0, 0).await.is_ok());
    }
}
