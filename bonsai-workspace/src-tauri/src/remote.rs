use std::sync::{Arc, Mutex as StdMutex, atomic::{AtomicBool, Ordering}};
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::io;
use tauri::AppHandle;
use serde::Serialize;
use scrap::{Capturer, Display};
use image::{codecs::png::PngEncoder, ColorType, ImageBuffer, ImageEncoder, RgbaImage};
use enigo::{Enigo, KeyboardControllable, Key, MouseButton, MouseControllable};

use crate::remote_input::RemoteInputEvent;

#[derive(Debug, Clone, Serialize)]
pub struct RemoteSessionInfo {
    pub id: String,
    pub state: String,
}

pub struct RemoteManager {
    active_session: Arc<StdMutex<Option<RemoteSessionInfo>>>,
    is_live: AtomicBool,
}

impl RemoteManager {
    pub fn new(_app_handle: &AppHandle) -> Self {
        Self {
            active_session: Arc::new(StdMutex::new(None)),
            is_live: AtomicBool::new(false),
        }
    }

    pub async fn start_session(&self) -> Result<RemoteSessionInfo, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis();

        let session = RemoteSessionInfo {
            id: format!("remote-{}", now),
            state: "started".into(),
        };

        *self.active_session.lock().map_err(|e| e.to_string())? = Some(session.clone());
        self.is_live.store(true, Ordering::SeqCst);
        Ok(session)
    }

    pub async fn stop_session(&self) -> Result<(), String> {
        *self.active_session.lock().map_err(|e| e.to_string())? = None;
        self.is_live.store(false, Ordering::SeqCst);
        Ok(())
    }

    pub async fn submit_input(&self, event: RemoteInputEvent) -> Result<(), String> {
        if !self.is_live.load(Ordering::SeqCst) {
            return Err("No active remote session".into());
        }

        eprintln!("[remote] queued input event: {:?}", event);

        let mut enigo = Enigo::new();

        match event.event_type.as_str() {
            "click" | "mouse_click" => {
                if let (Some(x), Some(y)) = (event.x, event.y) {
                    enigo.mouse_move_to(x, y);
                    let button = match event.button.as_deref().unwrap_or("left") {
                        "right" => MouseButton::Right,
                        "middle" => MouseButton::Middle,
                        _ => MouseButton::Left,
                    };
                    enigo.mouse_down(button);
                    enigo.mouse_up(button);
                    Ok(())
                } else {
                    Err("Missing x/y coordinates for click event".into())
                }
            }
            "mouse_move" => {
                if let (Some(x), Some(y)) = (event.x, event.y) {
                    enigo.mouse_move_to(x, y);
                    Ok(())
                } else {
                    Err("Missing x/y coordinates for mouse_move event".into())
                }
            }
            "mouse_down" => {
                if let (Some(x), Some(y)) = (event.x, event.y) {
                    enigo.mouse_move_to(x, y);
                    let button = match event.button.as_deref().unwrap_or("left") {
                        "right" => MouseButton::Right,
                        "middle" => MouseButton::Middle,
                        _ => MouseButton::Left,
                    };
                    enigo.mouse_down(button);
                    Ok(())
                } else {
                    Err("Missing x/y coordinates for mouse_down event".into())
                }
            }
            "mouse_up" => {
                if let (Some(x), Some(y)) = (event.x, event.y) {
                    enigo.mouse_move_to(x, y);
                    let button = match event.button.as_deref().unwrap_or("left") {
                        "right" => MouseButton::Right,
                        "middle" => MouseButton::Middle,
                        _ => MouseButton::Left,
                    };
                    enigo.mouse_up(button);
                    Ok(())
                } else {
                    Err("Missing x/y coordinates for mouse_up event".into())
                }
            }
            "key" | "key_press" | "key_click" => {
                if let Some(key) = event.key.as_deref() {
                    Self::perform_key_event(&mut enigo, key, event.modifiers.as_deref(), event.text.as_deref())
                } else {
                    Err("Missing key for key event".into())
                }
            }
            "key_down" => {
                if let Some(key) = event.key.as_deref() {
                    let mapped = Self::map_key(key)?;
                    enigo.key_down(mapped);
                    Ok(())
                } else {
                    Err("Missing key for key_down event".into())
                }
            }
            "key_up" => {
                if let Some(key) = event.key.as_deref() {
                    let mapped = Self::map_key(key)?;
                    enigo.key_up(mapped);
                    Ok(())
                } else {
                    Err("Missing key for key_up event".into())
                }
            }
            "text" | "text_input" | "type_text" => {
                if let Some(text) = event.text.as_deref() {
                    enigo.key_sequence(text);
                    Ok(())
                } else {
                    Err("Missing text for text_input event".into())
                }
            }
            _ => Err(format!("Unsupported remote input event type: {}", event.event_type)),
        }
    }

    fn perform_key_event(enigo: &mut Enigo, key: &str, modifiers: Option<&[String]>, text: Option<&str>) -> Result<(), String> {
        if let Some(mods) = modifiers {
            for modifier in mods {
                match modifier.to_lowercase().as_str() {
                    "control" | "ctrl" => enigo.key_down(Key::Control),
                    "shift" => enigo.key_down(Key::Shift),
                    "alt" => enigo.key_down(Key::Alt),
                    _ => (),
                }
            }
        }

        if let Some(text) = text {
            enigo.key_sequence(text);
        } else {
            let mapped = Self::map_key(key)?;
            enigo.key_click(mapped);
        }

        if let Some(mods) = modifiers {
            for modifier in mods.iter().rev() {
                match modifier.to_lowercase().as_str() {
                    "control" | "ctrl" => enigo.key_up(Key::Control),
                    "shift" => enigo.key_up(Key::Shift),
                    "alt" => enigo.key_up(Key::Alt),
                    _ => (),
                }
            }
        }

        Ok(())
    }

    fn map_key(key: &str) -> Result<Key, String> {
        let normalized = key.to_lowercase();
        let mapped = match normalized.as_str() {
            "enter" | "return" => Key::Return,
            "tab" => Key::Tab,
            "escape" | "esc" => Key::Escape,
            "backspace" => Key::Backspace,
            "space" | " " => Key::Space,
            "left" | "arrowleft" | "arrow_left" => Key::LeftArrow,
            "right" | "arrowright" | "arrow_right" => Key::RightArrow,
            "up" | "arrowup" | "arrow_up" => Key::UpArrow,
            "down" | "arrowdown" | "arrow_down" => Key::DownArrow,
            "delete" | "del" => Key::Delete,
            "home" => Key::Home,
            "end" => Key::End,
            "pageup" | "page_up" => Key::PageUp,
            "pagedown" | "page_down" => Key::PageDown,
            "capslock" => Key::CapsLock,
            "f1" => Key::F1,
            "f2" => Key::F2,
            "f3" => Key::F3,
            "f4" => Key::F4,
            "f5" => Key::F5,
            "f6" => Key::F6,
            "f7" => Key::F7,
            "f8" => Key::F8,
            "f9" => Key::F9,
            "f10" => Key::F10,
            "f11" => Key::F11,
            "f12" => Key::F12,
            _ => {
                if normalized.len() == 1 {
                    return Ok(Key::Layout(normalized.chars().next().unwrap()));
                }
                return Err(format!("Unsupported key: {}", key));
            }
        };
        Ok(mapped)
    }

    pub fn get_active_session(&self) -> Option<RemoteSessionInfo> {
        self.active_session.lock().ok().and_then(|session| session.clone())
    }

    pub fn capture_png(&self) -> Result<Vec<u8>, String> {
        if !self.is_live.load(Ordering::SeqCst) {
            return Err("No active remote session".into());
        }

        let display = Display::primary().map_err(|e| e.to_string())?;
        let mut capturer = Capturer::new(display).map_err(|e| e.to_string())?;
        let width = capturer.width();
        let height = capturer.height();
        let stride = width * 4;

        let frame = loop {
            match capturer.frame() {
                Ok(buffer) => break buffer.to_vec(),
                Err(ref error) if error.kind() == io::ErrorKind::WouldBlock => {
                    sleep(Duration::from_millis(16))
                }
                Err(e) => return Err(e.to_string()),
            }
        };

        let mut rgba = Vec::with_capacity(width * height * 4);
        for row in frame.chunks_exact(stride) {
            for pixel in row.chunks_exact(4) {
                rgba.push(pixel[2]);
                rgba.push(pixel[1]);
                rgba.push(pixel[0]);
                rgba.push(pixel[3]);
            }
        }

        let image: RgbaImage = ImageBuffer::from_raw(width as u32, height as u32, rgba)
            .ok_or_else(|| "Failed to convert screen capture to image buffer".to_string())?;

        let mut encoded = Vec::new();
        PngEncoder::new(&mut encoded)
            .write_image(
                image.as_raw(),
                width as u32,
                height as u32,
                ColorType::Rgba8.into(),
            )
            .map_err(|e| e.to_string())?;

        Ok(encoded)
    }
}
