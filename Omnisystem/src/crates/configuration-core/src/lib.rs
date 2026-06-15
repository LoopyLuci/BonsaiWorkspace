use universal_language_layer::bridge::LanguageBridge;

pub fn register_with_ull() -> Result<(), Box<dyn std::error::Error>> {
    let bridge = LanguageBridge::new();
    bridge.register_function("MODNAME::operation", vec![], vec![])?;
    Ok(())
}

pub fn operation(param: String) -> Result<String, String> {
    if param.is_empty() {
        Err("Parameter required".to_string())
    } else {
        Ok(format!("Processed: {}", param))
    }
}
