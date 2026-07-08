pub fn operation(param: String) -> Result<String, String> {
    if param.is_empty() {
        Err("Parameter required".to_string())
    } else {
        Ok(format!("Processed: {}", param))
    }
}
