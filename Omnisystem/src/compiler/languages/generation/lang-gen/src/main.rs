use anyhow::{Result, Context};
use serde::{Deserialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct Language {
    name: String,
    extensions: Vec<String>,
    parser: String,
}

#[derive(Debug, Deserialize)]
struct Config {
    languages: Vec<Language>,
}

fn sanitize_crate_name(name: &str) -> String {
    name.to_lowercase()
        .replace(" ", "-")
        .replace("+", "plus")
        .replace("#", "sharp")
        .replace("*", "star")
        .replace("/", "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect()
}

fn generate_crate(lang: &Language, base_path: &Path) -> Result<()> {
    let crate_name = format!("omnisystem-{}", sanitize_crate_name(&lang.name));
    let crate_path = base_path.join(&crate_name);
    fs::create_dir_all(&crate_path)?;

    let src_path = crate_path.join("src");
    fs::create_dir_all(&src_path)?;

    let uses_lsp = !lang.parser.is_empty() && lang.parser != "regex-fallback";
    let extensions_str = lang.extensions.iter()
        .map(|e| format!("\"{}\"", e))
        .collect::<Vec<_>>()
        .join(", ");

    let cargo_toml = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
core-ir = {{ path = "../core-ir" }}
language-system = {{ path = "../language-system" }}
async-trait = "0.1"
tracing = "0.1"
"#,
        crate_name
    );

    let lib_rs = if uses_lsp {
        format!(
            r#"use language_system::LanguageFrontend;
use frontend::{}Frontend;

mod frontend;

inventory::submit! {{
    language_system::LanguageRegistration {{
        name: "{}",
        factory: || Box::new({}Frontend::new()),
    }}
}}
"#,
            lang.name.replace(" ", ""),
            lang.name,
            lang.name.replace(" ", "")
        )
    } else {
        format!(
            r#"use language_system::LanguageFrontend;
use regex_frontend::RegexFrontend;

inventory::submit! {{
    language_system::LanguageRegistration {{
        name: "{}",
        factory: || Box::new(RegexFrontend::new("{}", vec![{}])),
    }}
}}
"#,
            lang.name,
            lang.name,
            extensions_str
        )
    };

    let frontend_rs = if uses_lsp {
        format!(
            r#"use language_system::LanguageFrontend;
use core_ir::*;
use std::path::Path;
use anyhow::Result;
use async_trait::async_trait;

pub struct {}Frontend;

impl {}Frontend {{
    pub fn new() -> Self {{ Self }}
}}

#[async_trait]
impl LanguageFrontend for {}Frontend {{
    fn language_name(&self) -> &str {{ "{}" }}
    fn file_extensions(&self) -> &[&str] {{ &{:?} }}

    async fn parse(&self, _source: &str, _path: &Path) -> Result<LairModule> {{
        Ok(LairModule {{
            name: "{}".into(),
            functions: vec![],
            types: vec![],
            constants: vec![],
            metadata: core_ir::ModuleMetadata {{
                imports: vec![],
                exports: vec![],
                source_language: Some("{}".into()),
            }},
        }})
    }}
}}
"#,
            lang.name.replace(" ", ""),
            lang.name.replace(" ", ""),
            lang.name.replace(" ", ""),
            lang.name,
            lang.extensions,
            format!("{}_module", lang.name.to_lowercase()),
            lang.name
        )
    } else {
        format!(r#"// {} uses regex fallback via bonsai-regex-frontend
"#, lang.name)
    };

    fs::write(crate_path.join("Cargo.toml"), cargo_toml)?;
    fs::write(src_path.join("lib.rs"), lib_rs)?;

    if uses_lsp {
        fs::write(src_path.join("frontend.rs"), frontend_rs)?;
    }

    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: bonsai-lang-gen <languages.yaml> [--all|--subset N]");
        eprintln!("Examples:");
        eprintln!("  bonsai-lang-gen languages.yaml --all");
        eprintln!("  bonsai-lang-gen languages.yaml --subset 50");
        std::process::exit(1);
    }

    let config_path = &args[1];
    let config_yaml = fs::read_to_string(config_path)
        .context(format!("Failed to read {}", config_path))?;
    let config: Config = serde_yaml::from_str(&config_yaml)
        .context("Failed to parse YAML")?;

    let subset_size = if args.len() > 2 && args[2] == "--all" {
        config.languages.len()
    } else if args.len() > 2 && args[2] == "--subset" && args.len() > 3 {
        args[3].parse::<usize>().unwrap_or(50)
    } else {
        50
    };

    let languages_to_gen = config.languages.iter().take(subset_size);
    let crate_base = Path::new("crates");

    for lang in languages_to_gen {
        generate_crate(lang, crate_base)?;
        println!("✓ Generated: omnisystem-{}", sanitize_crate_name(&lang.name));
    }

    println!("\n✅ Generated {} language crates", subset_size);
    println!("📦 Total languages configured: {}", config.languages.len());
    Ok(())
}
