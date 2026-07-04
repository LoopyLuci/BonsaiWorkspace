//! Input injection (keyboard, mouse, touch, gestures).
//!
//! Handles remote input delivery including keyboard events, mouse movements,
//! clicks, touch, and multi-touch gestures.

use crate::SessionId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during input operations.
#[derive(Debug, Error)]
pub enum InputError {
    #[error("Session not found: {session_id}")]
    SessionNotFound { session_id: String },

    #[error("Invalid input: {reason}")]
    InvalidInput { reason: String },

    #[error("Input injection failed: {reason}")]
    InjectionFailed { reason: String },

    #[error("Permission denied")]
    PermissionDenied,
}

/// Input event types.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum InputType {
    /// Keyboard key event.
    Keyboard,
    /// Mouse movement event.
    MouseMove,
    /// Mouse button event.
    MouseButton,
    /// Mouse scroll wheel.
    MouseScroll,
    /// Touch screen event.
    Touch,
    /// Gesture (pinch, rotate, etc.).
    Gesture,
    /// Text input (for input method support).
    TextInput,
}

impl InputType {
    pub fn as_str(&self) -> &'static str {
        match self {
            InputType::Keyboard => "keyboard",
            InputType::MouseMove => "mouse_move",
            InputType::MouseButton => "mouse_button",
            InputType::MouseScroll => "mouse_scroll",
            InputType::Touch => "touch",
            InputType::Gesture => "gesture",
            InputType::TextInput => "text_input",
        }
    }
}

/// Keyboard key event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardEvent {
    /// Key code (platform-specific).
    pub key_code: u32,
    /// Key name (e.g., "a", "Return", "F1").
    pub key_name: String,
    /// True if key is pressed, false if released.
    pub pressed: bool,
    /// Shift key held.
    pub shift: bool,
    /// Control key held.
    pub ctrl: bool,
    /// Alt key held.
    pub alt: bool,
    /// Super/Win key held.
    pub super_key: bool,
}

/// Mouse button types.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

/// Mouse button event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseButtonEvent {
    /// X coordinate.
    pub x: i32,
    /// Y coordinate.
    pub y: i32,
    /// Mouse button.
    pub button: MouseButton,
    /// True if pressed, false if released.
    pub pressed: bool,
    /// Number of clicks (1=single, 2=double, etc.).
    pub clicks: u8,
}

/// Mouse movement event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseMoveEvent {
    /// X coordinate.
    pub x: i32,
    /// Y coordinate.
    pub y: i32,
    /// Relative movement (for raw input mode).
    pub relative: Option<(i32, i32)>,
}

/// Mouse scroll event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseScrollEvent {
    /// X coordinate.
    pub x: i32,
    /// Y coordinate.
    pub y: i32,
    /// Vertical scroll amount (negative=down, positive=up).
    pub vertical: i32,
    /// Horizontal scroll amount (negative=left, positive=right).
    pub horizontal: i32,
}

/// Touch event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouchEvent {
    /// Touch points (ID, X, Y).
    pub touches: Vec<(u32, i32, i32)>,
    /// Touch phase (0=begin, 1=move, 2=end, 3=cancel).
    pub phase: u8,
}

/// Gesture event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GestureEvent {
    /// Gesture type ("pinch", "rotate", "swipe").
    pub gesture_type: String,
    /// Scale factor (for pinch).
    pub scale: Option<f64>,
    /// Rotation angle in radians (for rotate).
    pub rotation: Option<f64>,
    /// Velocity for swipe.
    pub velocity: Option<f64>,
}

/// Text input event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextInputEvent {
    /// Text to insert.
    pub text: String,
    /// Replace existing selection (if any).
    pub replace_selection: bool,
}

/// Input service for injecting remote input.
pub struct InputService {
    /// Delivery queue (for testing).
    delivered_inputs: Arc<tokio::sync::Mutex<Vec<InputEvent>>>,
}

/// Combined input event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEvent {
    Keyboard(KeyboardEvent),
    MouseMove(MouseMoveEvent),
    MouseButton(MouseButtonEvent),
    MouseScroll(MouseScrollEvent),
    Touch(TouchEvent),
    Gesture(GestureEvent),
    TextInput(TextInputEvent),
}

impl InputService {
    /// Create a new InputService.
    pub fn new() -> Self {
        InputService {
            delivered_inputs: Arc::new(tokio::sync::Mutex::new(vec![])),
        }
    }

    /// Inject a keyboard event.
    pub async fn inject_keyboard(
        &self,
        session_id: SessionId,
        event: KeyboardEvent,
    ) -> Result<(), InputError> {
        // In production: deliver to remote system via appropriate method
        // - Windows: SendInput API
        // - macOS: CGEventCreateKeyboardEvent
        // - Linux: xdotool or uinput

        self.delivered_inputs
            .lock()
            .await
            .push(InputEvent::Keyboard(event));

        tracing::debug!("Injected keyboard event for session {}", session_id);
        Ok(())
    }

    /// Inject a mouse move event.
    pub async fn inject_mouse_move(
        &self,
        session_id: SessionId,
        event: MouseMoveEvent,
    ) -> Result<(), InputError> {
        self.delivered_inputs
            .lock()
            .await
            .push(InputEvent::MouseMove(event));

        tracing::debug!("Injected mouse move for session {}", session_id);
        Ok(())
    }

    /// Inject a mouse button event.
    pub async fn inject_mouse_button(
        &self,
        session_id: SessionId,
        event: MouseButtonEvent,
    ) -> Result<(), InputError> {
        self.delivered_inputs
            .lock()
            .await
            .push(InputEvent::MouseButton(event));

        tracing::debug!("Injected mouse button for session {}", session_id);
        Ok(())
    }

    /// Inject a mouse scroll event.
    pub async fn inject_mouse_scroll(
        &self,
        session_id: SessionId,
        event: MouseScrollEvent,
    ) -> Result<(), InputError> {
        self.delivered_inputs
            .lock()
            .await
            .push(InputEvent::MouseScroll(event));

        tracing::debug!("Injected mouse scroll for session {}", session_id);
        Ok(())
    }

    /// Inject a touch event.
    pub async fn inject_touch(
        &self,
        session_id: SessionId,
        event: TouchEvent,
    ) -> Result<(), InputError> {
        self.delivered_inputs
            .lock()
            .await
            .push(InputEvent::Touch(event));

        tracing::debug!("Injected touch event for session {}", session_id);
        Ok(())
    }

    /// Inject a gesture event.
    pub async fn inject_gesture(
        &self,
        session_id: SessionId,
        event: GestureEvent,
    ) -> Result<(), InputError> {
        self.delivered_inputs
            .lock()
            .await
            .push(InputEvent::Gesture(event));

        tracing::debug!("Injected gesture for session {}", session_id);
        Ok(())
    }

    /// Inject text input.
    pub async fn inject_text(
        &self,
        session_id: SessionId,
        event: TextInputEvent,
    ) -> Result<(), InputError> {
        self.delivered_inputs
            .lock()
            .await
            .push(InputEvent::TextInput(event));

        tracing::debug!("Injected text input for session {}", session_id);
        Ok(())
    }

    /// Get delivered inputs (for testing).
    pub async fn get_delivered_inputs(&self) -> Vec<InputEvent> {
        self.delivered_inputs.lock().await.clone()
    }

    /// Clear delivered inputs (for testing).
    pub async fn clear_delivered_inputs(&self) {
        self.delivered_inputs.lock().await.clear();
    }
}

impl Default for InputService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_inject_keyboard() {
        let service = InputService::new();
        let session_id = SessionId::new();

        let event = KeyboardEvent {
            key_code: 65,
            key_name: "a".to_string(),
            pressed: true,
            shift: false,
            ctrl: false,
            alt: false,
            super_key: false,
        };

        service.inject_keyboard(session_id, event).await.unwrap();

        let delivered = service.get_delivered_inputs().await;
        assert_eq!(delivered.len(), 1);
    }

    #[tokio::test]
    async fn test_inject_mouse_move() {
        let service = InputService::new();
        let session_id = SessionId::new();

        let event = MouseMoveEvent {
            x: 100,
            y: 200,
            relative: None,
        };

        service.inject_mouse_move(session_id, event).await.unwrap();

        let delivered = service.get_delivered_inputs().await;
        assert_eq!(delivered.len(), 1);
    }

    #[tokio::test]
    async fn test_inject_mouse_button() {
        let service = InputService::new();
        let session_id = SessionId::new();

        let event = MouseButtonEvent {
            x: 100,
            y: 200,
            button: MouseButton::Left,
            pressed: true,
            clicks: 1,
        };

        service.inject_mouse_button(session_id, event).await.unwrap();

        let delivered = service.get_delivered_inputs().await;
        assert_eq!(delivered.len(), 1);
    }

    #[tokio::test]
    async fn test_inject_text() {
        let service = InputService::new();
        let session_id = SessionId::new();

        let event = TextInputEvent {
            text: "Hello".to_string(),
            replace_selection: false,
        };

        service.inject_text(session_id, event).await.unwrap();

        let delivered = service.get_delivered_inputs().await;
        assert_eq!(delivered.len(), 1);
    }

    #[tokio::test]
    async fn test_multiple_inputs() {
        let service = InputService::new();
        let session_id = SessionId::new();

        let kb_event = KeyboardEvent {
            key_code: 65,
            key_name: "a".to_string(),
            pressed: true,
            shift: false,
            ctrl: false,
            alt: false,
            super_key: false,
        };

        let mouse_event = MouseMoveEvent {
            x: 100,
            y: 200,
            relative: None,
        };

        service.inject_keyboard(session_id, kb_event).await.unwrap();
        service.inject_mouse_move(session_id, mouse_event).await.unwrap();

        let delivered = service.get_delivered_inputs().await;
        assert_eq!(delivered.len(), 2);
    }

    #[tokio::test]
    async fn test_clear_inputs() {
        let service = InputService::new();
        let session_id = SessionId::new();

        let event = KeyboardEvent {
            key_code: 65,
            key_name: "a".to_string(),
            pressed: true,
            shift: false,
            ctrl: false,
            alt: false,
            super_key: false,
        };

        service.inject_keyboard(session_id, event).await.unwrap();
        assert_eq!(service.get_delivered_inputs().await.len(), 1);

        service.clear_delivered_inputs().await;
        assert_eq!(service.get_delivered_inputs().await.len(), 0);
    }
}
