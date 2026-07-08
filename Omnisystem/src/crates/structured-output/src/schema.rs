use serde::{Serialize, Deserialize};
use crate::JsonSchemaProperty;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    pub schema_type: String,
    pub properties: Option<std::collections::HashMap<String, JsonSchemaProperty>>,
    pub required: Option<Vec<String>>,
    pub items: Option<Box<JsonSchema>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchemaProperty {
    pub prop_type: String,
    pub description: Option<String>,
    pub enum_values: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedOutput {
    pub valid: bool,
    pub parsed: serde_json::Value,
    pub errors: Vec<String>,
}

impl JsonSchema {
    pub fn from_json(value: &serde_json::Value) -> Result<Self, anyhow::Error> {
        Ok(serde_json::from_value(value.clone())?)
    }

    pub fn validate(&self, json_str: &str) -> ValidatedOutput {
        match serde_json::from_str::<serde_json::Value>(json_str) {
            Ok(value) => {
                let errors = self.check_value(&value, "");
                ValidatedOutput {
                    valid: errors.is_empty(),
                    parsed: value,
                    errors,
                }
            }
            Err(e) => ValidatedOutput {
                valid: false,
                parsed: serde_json::Value::Null,
                errors: vec![format!("JSON parse error: {}", e)],
            },
        }
    }

    fn check_value(&self, value: &serde_json::Value, path: &str) -> Vec<String> {
        let mut errors = Vec::new();
        match self.schema_type.as_str() {
            "object" => {
                if !value.is_object() {
                    errors.push(format!("{}: expected object", path));
                    return errors;
                }
                let obj = value.as_object().unwrap();
                if let Some(props) = &self.properties {
                    for (key, prop) in props {
                        let child_path = if path.is_empty() { key.clone() } else { format!("{}.{}", path, key) };
                        if let Some(child) = obj.get(key) {
                            errors.extend(prop.check_value(child, &child_path));
                        } else if self.required.as_ref().map_or(false, |r| r.contains(key)) {
                            errors.push(format!("{}: missing required field", child_path));
                        }
                    }
                }
            }
            "array" => {
                if !value.is_array() {
                    errors.push(format!("{}: expected array", path));
                    return errors;
                }
                if let Some(item_schema) = &self.items {
                    for (i, item) in value.as_array().unwrap().iter().enumerate() {
                        let child_path = format!("{}[{}]", path, i);
                        errors.extend(item_schema.check_value(item, &child_path));
                    }
                }
            }
            "string" => if !value.is_string() { errors.push(format!("{}: expected string", path)); },
            "number" => if !value.is_number() { errors.push(format!("{}: expected number", path)); },
            "boolean" => if !value.is_boolean() { errors.push(format!("{}: expected boolean", path)); },
            _ => {}
        }
        errors
    }
}

impl JsonSchemaProperty {
    fn check_value(&self, value: &serde_json::Value, path: &str) -> Vec<String> {
        let mut errors = Vec::new();
        match self.prop_type.as_str() {
            "string" => if !value.is_string() { errors.push(format!("{}: expected string", path)); },
            "number" => if !value.is_number() { errors.push(format!("{}: expected number", path)); },
            "boolean" => if !value.is_boolean() { errors.push(format!("{}: expected boolean", path)); },
            "array" => if !value.is_array() { errors.push(format!("{}: expected array", path)); },
            "object" => if !value.is_object() { errors.push(format!("{}: expected object", path)); },
            _ => {}
        }
        if let Some(enum_vals) = &self.enum_values {
            if let Some(s) = value.as_str() {
                if !enum_vals.iter().any(|v| v == s) {
                    errors.push(format!("{}: value '{}' not in enum {:?}", path, s, enum_vals));
                }
            }
        }
        errors
    }
}
