use anyhow::{anyhow, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
use tokio::time::timeout;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name:        String,
    pub description: String,
    pub schema:      String,
    pub handler_url: Option<String>,
    pub builtin:     bool,
}

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub result:     String,
    pub success:    bool,
    pub latency_ms: f64,
}

pub struct ToolRegistry {
    tools: DashMap<String, ToolDef>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: DashMap::new() }
    }

    pub fn register_builtins(&self) {
        let builtins = vec![
            ToolDef {
                name: "read_file".to_string(),
                description: "Read the contents of a file at the given path".to_string(),
                schema: r#"{"type":"object","properties":{"path":{"type":"string","description":"File path to read"}},"required":["path"]}"#.to_string(),
                handler_url: None,
                builtin: true,
            },
            ToolDef {
                name: "write_file".to_string(),
                description: "Write content to a file at the given path".to_string(),
                schema: r#"{"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}"#.to_string(),
                handler_url: None,
                builtin: true,
            },
            ToolDef {
                name: "list_dir".to_string(),
                description: "List files and directories in a directory".to_string(),
                schema: r#"{"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}"#.to_string(),
                handler_url: None,
                builtin: true,
            },
            ToolDef {
                name: "http_get".to_string(),
                description: "Make an HTTP GET request to a URL".to_string(),
                schema: r#"{"type":"object","properties":{"url":{"type":"string"},"headers":{"type":"object"}},"required":["url"]}"#.to_string(),
                handler_url: None,
                builtin: true,
            },
            ToolDef {
                name: "http_post".to_string(),
                description: "Make an HTTP POST request to a URL with a JSON body".to_string(),
                schema: r#"{"type":"object","properties":{"url":{"type":"string"},"body":{"type":"object"},"headers":{"type":"object"}},"required":["url","body"]}"#.to_string(),
                handler_url: None,
                builtin: true,
            },
            ToolDef {
                name: "calculator".to_string(),
                description: "Evaluate a mathematical expression (supports +,-,*,/,^,sqrt,sin,cos,log)".to_string(),
                schema: r#"{"type":"object","properties":{"expression":{"type":"string"}},"required":["expression"]}"#.to_string(),
                handler_url: None,
                builtin: true,
            },
        ];
        for t in builtins {
            info!("[Tools] Registered builtin: {}", t.name);
            self.tools.insert(t.name.clone(), t);
        }
    }

    pub fn register(&self, def: ToolDef) {
        self.tools.insert(def.name.clone(), def);
    }

    pub fn unregister(&self, name: &str) -> bool {
        self.tools.remove(name).is_some()
    }

    pub fn list(&self, builtins_only: bool) -> Vec<ToolDef> {
        self.tools.iter()
            .filter(|t| !builtins_only || t.builtin)
            .map(|t| t.clone())
            .collect()
    }

    pub fn len(&self) -> usize { self.tools.len() }

    pub async fn execute(
        &self, name: &str, args_json: &str, timeout_ms: u32,
    ) -> Result<ToolResult> {
        let def = self.tools.get(name)
            .ok_or_else(|| anyhow!("Tool '{}' not registered", name))?
            .clone();

        let args: Value = serde_json::from_str(args_json)
            .unwrap_or(Value::Object(Default::default()));

        let t0     = Instant::now();
        let dur    = Duration::from_millis(if timeout_ms == 0 { 30_000 } else { timeout_ms as u64 });

        let result = timeout(dur, self.run_tool(&def, &args)).await
            .map_err(|_| anyhow!("Tool '{}' timed out after {}ms", name, timeout_ms))??;

        Ok(ToolResult {
            success:    true,
            result,
            latency_ms: t0.elapsed().as_secs_f64() * 1000.0,
        })
    }

    async fn run_tool(&self, def: &ToolDef, args: &Value) -> Result<String> {
        // Remote HTTP tool
        if let Some(url) = &def.handler_url {
            let client = reqwest::Client::new();
            let resp = client.post(url).json(args).send().await?;
            return Ok(resp.text().await?);
        }

        // Builtin tools
        match def.name.as_str() {
            "read_file" => {
                let path = args["path"].as_str()
                    .ok_or_else(|| anyhow!("read_file: missing 'path'"))?;
                let content = tokio::fs::read_to_string(path).await
                    .map_err(|e| anyhow!("read_file '{}': {}", path, e))?;
                Ok(content)
            }

            "write_file" => {
                let path    = args["path"].as_str().ok_or_else(|| anyhow!("missing 'path'"))?;
                let content = args["content"].as_str().ok_or_else(|| anyhow!("missing 'content'"))?;
                tokio::fs::write(path, content).await?;
                Ok(format!("Written {} bytes to '{}'", content.len(), path))
            }

            "list_dir" => {
                let path = args["path"].as_str().ok_or_else(|| anyhow!("missing 'path'"))?;
                let mut rd = tokio::fs::read_dir(path).await?;
                let mut entries = Vec::new();
                while let Some(entry) = rd.next_entry().await? {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let meta = entry.metadata().await?;
                    let kind = if meta.is_dir() { "dir" } else { "file" };
                    entries.push(format!("{} ({})", name, kind));
                }
                Ok(entries.join("\n"))
            }

            "http_get" => {
                let url  = args["url"].as_str().ok_or_else(|| anyhow!("missing 'url'"))?;
                let client = reqwest::Client::new();
                let mut req = client.get(url);
                if let Some(headers) = args["headers"].as_object() {
                    for (k, v) in headers {
                        if let Some(s) = v.as_str() {
                            req = req.header(k.as_str(), s);
                        }
                    }
                }
                let body = req.send().await?.text().await?;
                Ok(body)
            }

            "http_post" => {
                let url  = args["url"].as_str().ok_or_else(|| anyhow!("missing 'url'"))?;
                let body = args.get("body").cloned().unwrap_or(Value::Object(Default::default()));
                let client = reqwest::Client::new();
                let mut req = client.post(url).json(&body);
                if let Some(headers) = args["headers"].as_object() {
                    for (k, v) in headers {
                        if let Some(s) = v.as_str() {
                            req = req.header(k.as_str(), s);
                        }
                    }
                }
                let resp = req.send().await?.text().await?;
                Ok(resp)
            }

            "calculator" => {
                let expr = args["expression"].as_str()
                    .ok_or_else(|| anyhow!("missing 'expression'"))?;
                Ok(eval_math(expr)?.to_string())
            }

            other => Err(anyhow!("Unknown builtin tool: {}", other)),
        }
    }
}

/// Simple recursive descent math evaluator (no `eval`).
fn eval_math(expr: &str) -> Result<f64> {
    let s = expr.trim().replace(" ", "");
    parse_expr(&s, &mut 0)
}

fn parse_expr(s: &str, pos: &mut usize) -> Result<f64> {
    let mut lhs = parse_term(s, pos)?;
    while *pos < s.len() {
        match s.as_bytes().get(*pos) {
            Some(b'+') => { *pos += 1; lhs += parse_term(s, pos)?; }
            Some(b'-') => { *pos += 1; lhs -= parse_term(s, pos)?; }
            _ => break,
        }
    }
    Ok(lhs)
}

fn parse_term(s: &str, pos: &mut usize) -> Result<f64> {
    let mut lhs = parse_power(s, pos)?;
    while *pos < s.len() {
        match s.as_bytes().get(*pos) {
            Some(b'*') => { *pos += 1; lhs *= parse_power(s, pos)?; }
            Some(b'/') => {
                *pos += 1;
                let rhs = parse_power(s, pos)?;
                if rhs == 0.0 { return Err(anyhow!("Division by zero")); }
                lhs /= rhs;
            }
            _ => break,
        }
    }
    Ok(lhs)
}

fn parse_power(s: &str, pos: &mut usize) -> Result<f64> {
    let base = parse_unary(s, pos)?;
    if *pos < s.len() && s.as_bytes()[*pos] == b'^' {
        *pos += 1;
        let exp = parse_unary(s, pos)?;
        return Ok(base.powf(exp));
    }
    Ok(base)
}

fn parse_unary(s: &str, pos: &mut usize) -> Result<f64> {
    if *pos < s.len() && s.as_bytes()[*pos] == b'-' {
        *pos += 1;
        return Ok(-parse_primary(s, pos)?);
    }
    parse_primary(s, pos)
}

fn parse_primary(s: &str, pos: &mut usize) -> Result<f64> {
    let bytes = s.as_bytes();
    // Parentheses
    if *pos < s.len() && bytes[*pos] == b'(' {
        *pos += 1;
        let v = parse_expr(s, pos)?;
        if *pos < s.len() && bytes[*pos] == b')' { *pos += 1; }
        return Ok(v);
    }
    // Named functions
    for (name, f) in [
        ("sqrt", f64::sqrt as fn(f64) -> f64),
        ("sin",  f64::sin),
        ("cos",  f64::cos),
        ("tan",  f64::tan),
        ("log",  f64::ln),
        ("abs",  f64::abs),
        ("ceil", f64::ceil),
        ("floor",f64::floor),
    ] {
        if s[*pos..].starts_with(name) {
            *pos += name.len();
            if *pos < s.len() && bytes[*pos] == b'(' { *pos += 1; }
            let arg = parse_expr(s, pos)?;
            if *pos < s.len() && bytes[*pos] == b')' { *pos += 1; }
            return Ok(f(arg));
        }
    }
    // "pi" and "e"
    if s[*pos..].starts_with("pi") { *pos += 2; return Ok(std::f64::consts::PI); }
    if *pos < s.len() && bytes[*pos] == b'e' && (*pos + 1 >= s.len() || !bytes[*pos+1].is_ascii_alphanumeric()) {
        *pos += 1; return Ok(std::f64::consts::E);
    }
    // Number
    let start = *pos;
    while *pos < s.len() && (bytes[*pos].is_ascii_digit() || bytes[*pos] == b'.') {
        *pos += 1;
    }
    if *pos == start { return Err(anyhow!("Unexpected char at pos {}: {:?}", start, s.get(start..))); }
    s[start..*pos].parse::<f64>().map_err(|e| anyhow!("Parse float: {}", e))
}
