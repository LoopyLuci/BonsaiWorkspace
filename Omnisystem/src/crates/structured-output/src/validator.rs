use crate::JsonSchema;
use anyhow::Result;

pub fn enforce_schema(text: &str, schema: &JsonSchema) -> Result<String> {
    let cleaned = text.trim().to_string();
    let validation = schema.validate(&cleaned);
    if validation.valid {
        Ok(cleaned)
    } else {
        anyhow::bail!("Schema validation failed: {:?}", validation.errors)
    }
}
