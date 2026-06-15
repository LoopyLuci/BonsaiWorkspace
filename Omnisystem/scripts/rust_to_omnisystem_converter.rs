// rust_to_omnisystem_converter.rs
// Core Rust → Omnisystem Language Converter
// Handles conversion from Rust to Titan, Aether, Sylva, and Axiom

use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct RustType {
    name: String,
    fields: Vec<(String, String)>,
    is_struct: bool,
}

#[derive(Debug, Clone)]
pub struct RustFunction {
    name: String,
    params: Vec<(String, String)>,
    return_type: String,
    body: String,
}

pub trait LanguageConverter {
    fn convert_type(&self, rust_type: &RustType) -> String;
    fn convert_function(&self, rust_fn: &RustFunction) -> String;
    fn convert_imports(&self, imports: &[String]) -> String;
    fn file_extension(&self) -> &str;
}

pub struct TitanConverter {
    type_map: HashMap<String, String>,
}

pub struct AetherConverter {
    type_map: HashMap<String, String>,
}

pub struct SylvaConverter {
    type_map: HashMap<String, String>,
}

pub struct AxiomConverter {
    type_map: HashMap<String, String>,
}

impl TitanConverter {
    pub fn new() -> Self {
        let mut type_map = HashMap::new();
        type_map.insert("i32".to_string(), "i32".to_string());
        type_map.insert("i64".to_string(), "i64".to_string());
        type_map.insert("f64".to_string(), "f64".to_string());
        type_map.insert("bool".to_string(), "bool".to_string());
        type_map.insert("String".to_string(), "String".to_string());
        type_map.insert("Vec<T>".to_string(), "Vec<T>".to_string());

        TitanConverter { type_map }
    }
}

impl LanguageConverter for TitanConverter {
    fn convert_type(&self, rust_type: &RustType) -> String {
        if rust_type.is_struct {
            let mut output = format!("pub struct {} {{\n", rust_type.name);
            for (field_name, field_type) in &rust_type.fields {
                let converted_type = self.type_map.get(field_type)
                    .cloned()
                    .unwrap_or_else(|| field_type.clone());
                output.push_str(&format!("    {}: {},\n", field_name, converted_type));
            }
            output.push_str("}\n");
            output
        } else {
            format!("pub enum {} {{}}\n", rust_type.name)
        }
    }

    fn convert_function(&self, rust_fn: &RustFunction) -> String {
        let mut params_str = String::new();
        for (i, (param_name, param_type)) in rust_fn.params.iter().enumerate() {
            let converted_type = self.type_map.get(param_type)
                .cloned()
                .unwrap_or_else(|| param_type.clone());
            if i > 0 {
                params_str.push_str(", ");
            }
            params_str.push_str(&format!("{}: {}", param_name, converted_type));
        }

        let return_type = self.type_map.get(&rust_fn.return_type)
            .cloned()
            .unwrap_or_else(|| rust_fn.return_type.clone());

        format!(
            "pub fn {}({}) -> {} {{\n    {}\n}}\n",
            rust_fn.name, params_str, return_type, rust_fn.body
        )
    }

    fn convert_imports(&self, imports: &[String]) -> String {
        let mut output = String::new();
        for import in imports {
            output.push_str(&format!("use {};\n", import));
        }
        output
    }

    fn file_extension(&self) -> &str {
        "ti"
    }
}

impl LanguageConverter for AetherConverter {
    fn convert_type(&self, rust_type: &RustType) -> String {
        if rust_type.is_struct {
            let mut output = format!("pub actor {} {{\n", rust_type.name);
            for (field_name, field_type) in &rust_type.fields {
                output.push_str(&format!("    {}: {},\n", field_name, field_type));
            }
            output.push_str("}\n");
            output
        } else {
            format!("pub actor {} {{}}\n", rust_type.name)
        }
    }

    fn convert_function(&self, rust_fn: &RustFunction) -> String {
        format!(
            "pub fn {}({}) -> {} {{\n    {}\n}}\n",
            rust_fn.name,
            rust_fn.params.iter()
                .map(|(n, t)| format!("{}: {}", n, t))
                .collect::<Vec<_>>()
                .join(", "),
            rust_fn.return_type,
            rust_fn.body
        )
    }

    fn convert_imports(&self, imports: &[String]) -> String {
        let mut output = String::new();
        for import in imports {
            output.push_str(&format!("use {};\n", import));
        }
        output
    }

    fn file_extension(&self) -> &str {
        "ae"
    }
}

impl LanguageConverter for SylvaConverter {
    fn convert_type(&self, rust_type: &RustType) -> String {
        if rust_type.is_struct {
            let mut output = format!("pub struct {} {{\n", rust_type.name);
            for (field_name, field_type) in &rust_type.fields {
                output.push_str(&format!("    {}: {},\n", field_name, field_type));
            }
            output.push_str("}\n");
            output
        } else {
            format!("pub enum {} {{}}\n", rust_type.name)
        }
    }

    fn convert_function(&self, rust_fn: &RustFunction) -> String {
        format!(
            "pub fn {}({}) -> {} {{\n    {}\n}}\n",
            rust_fn.name,
            rust_fn.params.iter()
                .map(|(n, t)| format!("{}: {}", n, t))
                .collect::<Vec<_>>()
                .join(", "),
            rust_fn.return_type,
            rust_fn.body
        )
    }

    fn convert_imports(&self, imports: &[String]) -> String {
        let mut output = String::new();
        for import in imports {
            output.push_str(&format!("use {};\n", import));
        }
        output
    }

    fn file_extension(&self) -> &str {
        "sy"
    }
}

impl LanguageConverter for AxiomConverter {
    fn convert_type(&self, rust_type: &RustType) -> String {
        if rust_type.is_struct {
            let mut output = format!("pub struct {} {{\n", rust_type.name);
            for (field_name, field_type) in &rust_type.fields {
                output.push_str(&format!("    {}: {},\n", field_name, field_type));
            }
            output.push_str("}\n");
            output
        } else {
            format!("pub enum {} {{}}\n", rust_type.name)
        }
    }

    fn convert_function(&self, rust_fn: &RustFunction) -> String {
        format!(
            "pub fn {}({}) -> {} {{\n    {}\n}}\n",
            rust_fn.name,
            rust_fn.params.iter()
                .map(|(n, t)| format!("{}: {}", n, t))
                .collect::<Vec<_>>()
                .join(", "),
            rust_fn.return_type,
            rust_fn.body
        )
    }

    fn convert_imports(&self, imports: &[String]) -> String {
        let mut output = String::new();
        for import in imports {
            output.push_str(&format!("// use {};\n", import));
        }
        output
    }

    fn file_extension(&self) -> &str {
        "ax"
    }
}

pub fn get_converter(language: &str) -> Box<dyn LanguageConverter> {
    match language {
        "titan" => Box::new(TitanConverter::new()),
        "aether" => Box::new(AetherConverter::new()),
        "sylva" => Box::new(SylvaConverter::new()),
        "axiom" => Box::new(AxiomConverter::new()),
        _ => Box::new(TitanConverter::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_titan_converter() {
        let converter = TitanConverter::new();
        let rust_type = RustType {
            name: "Example".to_string(),
            fields: vec![
                ("value".to_string(), "i32".to_string()),
            ],
            is_struct: true,
        };
        let result = converter.convert_type(&rust_type);
        assert!(result.contains("pub struct Example"));
        assert!(result.contains("value: i32"));
    }

    #[test]
    fn test_function_conversion() {
        let converter = TitanConverter::new();
        let rust_fn = RustFunction {
            name: "create".to_string(),
            params: vec![("value".to_string(), "i32".to_string())],
            return_type: "i32".to_string(),
            body: "return value * 2;".to_string(),
        };
        let result = converter.convert_function(&rust_fn);
        assert!(result.contains("pub fn create"));
        assert!(result.contains("value: i32"));
    }
}

fn main() {
    println!("Rust to Omnisystem Language Converter");
    println!("======================================");
    println!("Supported languages: titan, aether, sylva, axiom");
}
