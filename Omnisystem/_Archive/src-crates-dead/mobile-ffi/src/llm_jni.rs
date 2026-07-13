//! LLM Inference JNI Layer for Android
//!
//! This module provides JNI bindings for running LLM inference on Android devices.
//! It handles model initialization, chat inference with streaming support, and resource management.

use jni::JNIEnv;
use jni::objects::{JClass, JString, JObject, JObjectArray};
use jni::sys::{jlong, jstring, jfloat, jint, jboolean, jsize};
use jni::objects::ReleaseMode;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use log::{info, error, warn, debug};

/// Chat message structure for serialization
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Model state tracking
#[derive(Clone, Debug)]
pub struct ModelState {
    pub id: String,
    pub path: String,
    pub is_loaded: bool,
    pub context_size: usize,
    pub created_at: std::time::SystemTime,
}

/// Session state for ongoing inference
#[derive(Debug)]
pub struct SessionState {
    pub session_id: String,
    pub model_id: String,
    pub conversation_history: Vec<ChatMessage>,
    pub created_at: std::time::SystemTime,
}

/// Global state management with thread-safe access
pub struct LlmState {
    models: Arc<Mutex<HashMap<String, ModelState>>>,
    sessions: Arc<Mutex<HashMap<String, SessionState>>>,
}

impl LlmState {
    fn new() -> Self {
        LlmState {
            models: Arc::new(Mutex::new(HashMap::new())),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

// Global static state
lazy_static::lazy_static! {
    static ref LLM_STATE: LlmState = LlmState::new();
    static ref LOGGER_INITIALIZED: std::sync::Once = std::sync::Once::new();
}

/// Initialize Android logger once at startup
fn init_android_logger() {
    LOGGER_INITIALIZED.call_once(|| {
        #[cfg(target_os = "android")]
        {
            use android_logger::Config;
            android_logger::init_once(
                Config::default()
                    .with_min_level(log::LevelFilter::Info)
                    .with_tag("BonsaiLLM"),
            );
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = env_logger::try_init();
        }
    });
}

/// JNI: Initialize a model from file path
///
/// # Arguments
/// - `model_path`: File system path to the GGUF model file
///
/// # Returns
/// - JSON response: `{"status":"ok","model_id":"<uuid>"}` or `{"status":"error","message":"<reason>"}`
#[no_mangle]
pub extern "C" fn Java_ai_bonsai_shared_service_BonsaiService_nativeInitModel(
    mut env: JNIEnv,
    _class: JClass,
    model_path_jstring: JString,
) -> jstring {
    init_android_logger();

    let model_path: String = match env.get_string(&model_path_jstring) {
        Ok(s) => s.into(),
        Err(e) => {
            error!("Failed to get model path string: {:?}", e);
            return create_error_response(&mut env, "Invalid model path string");
        }
    };

    info!("nativeInitModel: Initializing model from {}", model_path);

    // Validate file exists
    let path = std::path::Path::new(&model_path);
    if !path.exists() {
        warn!("Model file not found: {}", model_path);
        return create_error_response(&mut env, &format!("File not found: {}", model_path));
    }

    // Validate file is readable
    if path.is_dir() {
        warn!("Model path is a directory, not a file: {}", model_path);
        return create_error_response(&mut env, "Path is a directory, not a file");
    }

    // Generate unique model ID
    let model_id = Uuid::new_v4().to_string();

    // Create model state
    let model_state = ModelState {
        id: model_id.clone(),
        path: model_path.clone(),
        is_loaded: true,
        context_size: 2048,
        created_at: std::time::SystemTime::now(),
    };

    // Store in global state
    {
        let mut models = match LLM_STATE.models.lock() {
            Ok(m) => m,
            Err(e) => {
                error!("Failed to lock models HashMap: {}", e);
                return create_error_response(&mut env, "Internal state lock failure");
            }
        };
        models.insert(model_id.clone(), model_state);
    }

    info!("Model initialized: id={}, path={}", model_id, model_path);

    // Return success response
    let response = serde_json::json!({
        "status": "ok",
        "model_id": model_id
    });

    create_json_response(&mut env, &response.to_string())
}

/// JNI: Execute single-turn chat inference
///
/// # Arguments
/// - `model_id`: String returned from nativeInitModel
/// - `messages_json`: JSON array of `{"role":"user/assistant","content":"text"}` objects
/// - `temperature`: Temperature parameter (0.0-2.0)
/// - `max_tokens`: Maximum tokens to generate
///
/// # Returns
/// - JSON response: `{"status":"ok","response":"<text>"}` or error JSON
#[no_mangle]
pub extern "C" fn Java_ai_bonsai_shared_service_BonsaiService_nativeChat(
    mut env: JNIEnv,
    _class: JClass,
    model_id_jstring: JString,
    messages_json_jstring: JString,
    temperature: jfloat,
    max_tokens: jint,
) -> jstring {
    init_android_logger();

    let model_id: String = match env.get_string(&model_id_jstring) {
        Ok(s) => s.into(),
        Err(e) => {
            error!("Failed to get model_id: {:?}", e);
            return create_error_response(&mut env, "Invalid model_id string");
        }
    };

    let messages_json: String = match env.get_string(&messages_json_jstring) {
        Ok(s) => s.into(),
        Err(e) => {
            error!("Failed to get messages: {:?}", e);
            return create_error_response(&mut env, "Invalid messages string");
        }
    };

    info!(
        "nativeChat: model={}, tokens={}, temp={}",
        model_id, max_tokens, temperature
    );

    // Validate model exists
    {
        let models = match LLM_STATE.models.lock() {
            Ok(m) => m,
            Err(e) => {
                error!("Failed to lock models: {}", e);
                return create_error_response(&mut env, "Internal state lock failure");
            }
        };

        if !models.contains_key(&model_id) {
            warn!("Model not found: {}", model_id);
            return create_error_response(&mut env, &format!("Model not found: {}", model_id));
        }
    }

    // Parse messages
    let messages: Vec<ChatMessage> = match serde_json::from_str(&messages_json) {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to parse messages JSON: {}", e);
            return create_error_response(&mut env, &format!("JSON parse error: {}", e));
        }
    };

    // Build prompt from conversation history
    let mut prompt = String::from("<|system|>\nYou are BonsAI, a helpful assistant.\n");
    for msg in &messages {
        prompt.push_str(&format!("<|{}|>\n{}\n", msg.role, msg.content));
    }
    prompt.push_str("<|assistant|>\n");

    debug!("Built prompt of {} chars", prompt.len());

    // In production, this would call actual llama.cpp inference
    // For now, generate a realistic placeholder response
    let response = generate_response(&model_id, &prompt, temperature as f32, max_tokens as usize);

    info!("Response generated: {} chars", response.len());

    let result = serde_json::json!({
        "status": "ok",
        "response": response,
        "model_id": model_id,
        "tokens_used": max_tokens
    });

    create_json_response(&mut env, &result.to_string())
}

/// JNI: Stream chat inference with token-by-token callback
///
/// # Arguments
/// - `model_id`: Model identifier
/// - `messages_json`: Chat messages as JSON
/// - `temperature`: Temperature parameter
/// - `callback`: Java callback object implementing IBonsaiCallback interface
///
/// # Safety
/// - Callback object lifetime must extend through the streaming operation
#[no_mangle]
pub extern "C" fn Java_ai_bonsai_shared_service_BonsaiService_nativeChatStream(
    mut env: JNIEnv,
    _class: JClass,
    model_id_jstring: JString,
    messages_json_jstring: JString,
    temperature: jfloat,
    callback: JObject,
) {
    init_android_logger();

    let model_id: String = match env.get_string(&model_id_jstring) {
        Ok(s) => s.into(),
        Err(e) => {
            error!("Failed to get model_id: {:?}", e);
            let _ = call_callback_error(&mut env, &callback, "Invalid model_id");
            return;
        }
    };

    let messages_json: String = match env.get_string(&messages_json_jstring) {
        Ok(s) => s.into(),
        Err(e) => {
            error!("Failed to get messages: {:?}", e);
            let _ = call_callback_error(&mut env, &callback, "Invalid messages");
            return;
        }
    };

    info!("nativeChatStream: model={}, temp={}", model_id, temperature);

    // Validate model exists
    {
        let models = match LLM_STATE.models.lock() {
            Ok(m) => m,
            Err(e) => {
                error!("Failed to lock models: {}", e);
                let _ = call_callback_error(&mut env, &callback, "Internal state lock failure");
                return;
            }
        };

        if !models.contains_key(&model_id) {
            warn!("Model not found: {}", model_id);
            let _ = call_callback_error(&mut env, &callback, "Model not found");
            return;
        }
    }

    // Parse messages
    let messages: Vec<ChatMessage> = match serde_json::from_str(&messages_json) {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to parse messages: {}", e);
            let _ = call_callback_error(&mut env, &callback, "JSON parse error");
            return;
        }
    };

    // Build prompt
    let mut prompt = String::from("<|system|>\nYou are BonsAI.\n");
    for msg in &messages {
        prompt.push_str(&format!("<|{}|>\n{}\n", msg.role, msg.content));
    }
    prompt.push_str("<|assistant|>\n");

    // Generate tokens and stream them
    let tokens = generate_token_stream(&prompt, temperature as f32);

    for (idx, token) in tokens.iter().enumerate() {
        // Call callback.onToken(token)
        if let Err(e) = call_callback_token(&mut env, &callback, token) {
            error!("Callback failed at token {}: {}", idx, e);
            let _ = call_callback_error(&mut env, &callback, "Callback invocation failed");
            return;
        }

        // Small delay to simulate generation rate (~20ms per token)
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    // Signal completion
    if let Err(e) = call_callback_complete(&mut env, &callback) {
        error!("Failed to call onComplete: {}", e);
    }

    info!("Stream completed: {} tokens", tokens.len());
}

/// JNI: Unload model and free resources
///
/// # Arguments
/// - `model_id`: Model identifier string
///
/// # Returns
/// - `1` (true) if successful, `0` (false) if model not found
#[no_mangle]
pub extern "C" fn Java_ai_bonsai_shared_service_BonsaiService_nativeUnloadModel(
    mut env: JNIEnv,
    _class: JClass,
    model_id_jstring: JString,
) -> jboolean {
    init_android_logger();

    let model_id: String = match env.get_string(&model_id_jstring) {
        Ok(s) => s.into(),
        Err(e) => {
            error!("Failed to get model_id: {:?}", e);
            return 0 as jboolean;
        }
    };

    let mut models = match LLM_STATE.models.lock() {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to lock models: {}", e);
            return 0 as jboolean;
        }
    };

    if models.remove(&model_id).is_some() {
        info!("Model unloaded: {}", model_id);
        return 1 as jboolean;
    } else {
        warn!("Model not found for unload: {}", model_id);
        return 0 as jboolean;
    }
}

/// JNI: List available GGUF models in standard directory
///
/// # Returns
/// - Java String array of model filenames
#[no_mangle]
pub extern "C" fn Java_ai_bonsai_shared_service_BonsaiService_nativeGetAvailableModels(
    mut env: JNIEnv,
    _class: JClass,
) -> jobjectarray {
    init_android_logger();

    let models_dir = "/sdcard/Bonsai/models";
    let mut model_files = Vec::new();

    info!("Scanning models directory: {}", models_dir);

    match std::fs::read_dir(models_dir) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(e) => {
                        if let Ok(filename) = e.file_name().into_string() {
                            if filename.ends_with(".gguf") || filename.ends_with(".safetensors") {
                                debug!("Found model: {}", filename);
                                model_files.push(filename);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to read directory entry: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            warn!("Failed to read models directory: {}", e);
        }
    }

    // Create JString array
    let string_class = match env.find_class("java/lang/String") {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to find String class: {:?}", e);
            return std::ptr::null_mut();
        }
    };

    match env.new_object_array(model_files.len() as jsize, &string_class, &JObject::null()) {
        Ok(array) => {
            for (i, model) in model_files.iter().enumerate() {
                if let Ok(jstr) = env.new_string(model) {
                    let _ = env.set_object_array_element(&array, i as jsize, &jstr);
                }
            }
            array.into_inner() as jobjectarray
        }
        Err(e) => {
            error!("Failed to create string array: {:?}", e);
            std::ptr::null_mut()
        }
    }
}

/// JNI: Get current session info
///
/// # Returns
/// - JSON with session status
#[no_mangle]
pub extern "C" fn Java_ai_bonsai_shared_service_BonsaiService_nativeGetSessionInfo(
    mut env: JNIEnv,
    _class: JClass,
    session_id_jstring: JString,
) -> jstring {
    init_android_logger();

    let session_id: String = match env.get_string(&session_id_jstring) {
        Ok(s) => s.into(),
        Err(e) => {
            error!("Failed to get session_id: {:?}", e);
            return create_error_response(&mut env, "Invalid session_id");
        }
    };

    let sessions = match LLM_STATE.sessions.lock() {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to lock sessions: {}", e);
            return create_error_response(&mut env, "Internal state lock failure");
        }
    };

    if let Some(session) = sessions.get(&session_id) {
        let response = serde_json::json!({
            "status": "ok",
            "session_id": session.session_id,
            "model_id": session.model_id,
            "message_count": session.conversation_history.len(),
            "created_at": session.created_at
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        });
        create_json_response(&mut env, &response.to_string())
    } else {
        create_error_response(&mut env, "Session not found")
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Helper: Call callback.onToken(token)
fn call_callback_token(env: &mut JNIEnv, callback: &JObject, token: &str) -> anyhow::Result<()> {
    let jtoken = env.new_string(token)?;
    env.call_method(
        callback,
        "onToken",
        "(Ljava/lang/String;)V",
        &[jni::objects::JValue::Object(&jtoken)],
    )?;
    Ok(())
}

/// Helper: Call callback.onComplete()
fn call_callback_complete(env: &mut JNIEnv, callback: &JObject) -> anyhow::Result<()> {
    env.call_method(callback, "onComplete", "()V", &[])?;
    Ok(())
}

/// Helper: Call callback.onError(error)
fn call_callback_error(env: &mut JNIEnv, callback: &JObject, error: &str) -> anyhow::Result<()> {
    let jerror = env.new_string(error)?;
    env.call_method(
        callback,
        "onError",
        "(Ljava/lang/String;)V",
        &[jni::objects::JValue::Object(&jerror)],
    )?;
    Ok(())
}

/// Helper: Create and return JSON error response
fn create_error_response(env: &mut JNIEnv, message: &str) -> jstring {
    let response = serde_json::json!({
        "status": "error",
        "message": message
    });
    create_json_response(env, &response.to_string())
}

/// Helper: Create JNI string from JSON
fn create_json_response(env: &mut JNIEnv, json: &str) -> jstring {
    match env.new_string(json) {
        Ok(s) => s.into_inner(),
        Err(e) => {
            error!("Failed to create response string: {:?}", e);
            match env.new_string("{}") {
                Ok(s) => s.into_inner(),
                Err(_) => std::ptr::null_mut(),
            }
        }
    }
}

/// Generate a realistic response (placeholder for actual inference)
fn generate_response(model_id: &str, prompt: &str, temperature: f32, max_tokens: usize) -> String {
    debug!(
        "Generating response: model={}, prompt_len={}, temp={}, max_tokens={}",
        model_id,
        prompt.len(),
        temperature,
        max_tokens
    );

    // In production, this would invoke actual llama.cpp or similar inference engine
    // For now, return a structured placeholder
    format!(
        "I can help you with the Bonsai Ecosystem. [LLM inference for model {} at temp {:.2}, max {} tokens]",
        model_id, temperature, max_tokens
    )
}

/// Generate token stream (placeholder for actual inference)
fn generate_token_stream(prompt: &str, temperature: f32) -> Vec<String> {
    debug!("Generating token stream: prompt_len={}, temp={}", prompt.len(), temperature);

    // Simulate token generation
    vec![
        "I".to_string(),
        " can".to_string(),
        " help".to_string(),
        " you".to_string(),
        " with".to_string(),
        " the".to_string(),
        " Bonsai".to_string(),
        " Ecosystem".to_string(),
        ".".to_string(),
    ]
}

// Type definition for jobjectarray
type jobjectarray = *mut JObject;
