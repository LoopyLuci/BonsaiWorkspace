//! BonsAI Expanded Tool Library — 60+ tools across every category.
//!
//! All tools implement the `tool_registry::Tool` trait and are registered in
//! `ToolRegistryState::new_with_defaults()`. Each tool is self-contained with
//! zero mandatory network calls (100% offline).

use crate::tool_registry::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value};

// ── Web / HTTP ────────────────────────────────────────────────────────────────

pub struct WebFetchTool;
#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }
    fn description(&self) -> &str {
        "Fetch a URL and return the response body (text/JSON). Requires internet."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let url = args["url"].as_str().ok_or("Missing 'url'")?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| e.to_string())?;
        let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
        let status = resp.status().as_u16();
        let body = resp.text().await.map_err(|e| e.to_string())?;
        Ok(ToolResult::json(&json!({ "status": status, "body": body })))
    }
}

pub struct HttpPostTool;
#[async_trait]
impl Tool for HttpPostTool {
    fn name(&self) -> &str {
        "http_post"
    }
    fn description(&self) -> &str {
        "POST JSON to a URL and return the response."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let url = args["url"].as_str().ok_or("Missing 'url'")?;
        let body = args.get("body").cloned().unwrap_or(json!({}));
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| e.to_string())?;
        let resp = client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = resp.status().as_u16();
        let text = resp.text().await.map_err(|e| e.to_string())?;
        Ok(ToolResult::json(&json!({ "status": status, "body": text })))
    }
}

pub struct HttpHeadersTool;
#[async_trait]
impl Tool for HttpHeadersTool {
    fn name(&self) -> &str {
        "http_headers"
    }
    fn description(&self) -> &str {
        "Fetch HTTP response headers for a URL (HEAD request)."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let url = args["url"].as_str().ok_or("Missing 'url'")?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| e.to_string())?;
        let resp = client.head(url).send().await.map_err(|e| e.to_string())?;
        let status = resp.status().as_u16();
        let headers: std::collections::HashMap<String, String> = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        Ok(ToolResult::json(
            &json!({ "status": status, "headers": headers }),
        ))
    }
}

// ── Data: CSV / JSON / XML ───────────────────────────────────────────────────

pub struct CsvParseTool;
#[async_trait]
impl Tool for CsvParseTool {
    fn name(&self) -> &str {
        "csv_parse"
    }
    fn description(&self) -> &str {
        "Parse a CSV string into an array of row objects. Supports quoted fields."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let csv = args["csv"].as_str().ok_or("Missing 'csv'")?;
        let has_header = args["has_header"].as_bool().unwrap_or(true);
        let mut lines = csv.lines();
        let headers: Vec<String> = if has_header {
            lines.next().map(parse_csv_row).unwrap_or_default()
        } else {
            vec![]
        };
        let rows: Vec<Value> = lines
            .map(|line| {
                let cols = parse_csv_row(line);
                if headers.is_empty() {
                    Value::Array(cols.into_iter().map(Value::String).collect())
                } else {
                    let obj: serde_json::Map<String, Value> = headers
                        .iter()
                        .cloned()
                        .zip(cols.into_iter().map(Value::String))
                        .collect();
                    Value::Object(obj)
                }
            })
            .collect();
        Ok(ToolResult::json(
            &json!({ "rows": rows, "count": rows.len() }),
        ))
    }
}

fn parse_csv_row(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    chars.next();
                    current.push('"');
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                fields.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(c),
        }
    }
    fields.push(current.trim().to_string());
    fields
}

pub struct JsonTransformTool;
#[async_trait]
impl Tool for JsonTransformTool {
    fn name(&self) -> &str {
        "json_transform"
    }
    fn description(&self) -> &str {
        "Apply a JMESPath-style dot-path query to JSON data and return the result."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let data = args.get("data").ok_or("Missing 'data'")?;
        let path = args["path"].as_str().ok_or("Missing 'path'")?;
        let result = json_dot_path(data, path);
        Ok(ToolResult::json(&json!({ "result": result, "path": path })))
    }
}

fn json_dot_path<'a>(data: &'a Value, path: &str) -> &'a Value {
    let mut cur = data;
    for part in path.split('.') {
        cur = match cur {
            Value::Object(m) => m.get(part).unwrap_or(&Value::Null),
            Value::Array(a) => part
                .parse::<usize>()
                .ok()
                .and_then(|i| a.get(i))
                .unwrap_or(&Value::Null),
            _ => &Value::Null,
        };
    }
    cur
}

pub struct JsonSchemaTool;
#[async_trait]
impl Tool for JsonSchemaTool {
    fn name(&self) -> &str {
        "json_schema"
    }
    fn description(&self) -> &str {
        "Infer a JSON Schema from a JSON value."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let data = args.get("data").ok_or("Missing 'data'")?;
        Ok(ToolResult::json(&infer_schema(data)))
    }
}

fn infer_schema(v: &Value) -> Value {
    match v {
        Value::Null => json!({"type": "null"}),
        Value::Bool(_) => json!({"type": "boolean"}),
        Value::Number(_) => {
            if v.as_i64().is_some() {
                json!({"type": "integer"})
            } else {
                json!({"type": "number"})
            }
        }
        Value::String(_) => json!({"type": "string"}),
        Value::Array(a) => {
            let items = a.first().map(infer_schema).unwrap_or(json!({}));
            json!({"type": "array", "items": items})
        }
        Value::Object(m) => {
            let props: serde_json::Map<String, Value> = m
                .iter()
                .map(|(k, v)| (k.clone(), infer_schema(v)))
                .collect();
            json!({"type": "object", "properties": props})
        }
    }
}

pub struct XmlParseTool;
#[async_trait]
impl Tool for XmlParseTool {
    fn name(&self) -> &str {
        "xml_parse"
    }
    fn description(&self) -> &str {
        "Parse an XML string and return a simplified JSON representation."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let xml = args["xml"].as_str().ok_or("Missing 'xml'")?;
        // Lightweight: extract text content and tag structure
        let stripped = strip_xml_tags(xml);
        Ok(ToolResult::json(
            &json!({ "text": stripped, "length": xml.len() }),
        ))
    }
}

fn strip_xml_tags(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ── Text Analysis ─────────────────────────────────────────────────────────────

pub struct SentimentTool;
#[async_trait]
impl Tool for SentimentTool {
    fn name(&self) -> &str {
        "sentiment_analysis"
    }
    fn description(&self) -> &str {
        "Lexicon-based sentiment analysis: returns positive/negative/neutral score."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let text = args["text"].as_str().ok_or("Missing 'text'")?;
        let (pos, neg) = sentiment_score(text);
        let label = if pos > neg + 0.1 {
            "positive"
        } else if neg > pos + 0.1 {
            "negative"
        } else {
            "neutral"
        };
        Ok(ToolResult::json(
            &json!({ "label": label, "positive": pos, "negative": neg, "compound": pos - neg }),
        ))
    }
}

fn sentiment_score(text: &str) -> (f64, f64) {
    let lower = text.to_lowercase();
    let pos_words = [
        "good",
        "great",
        "excellent",
        "amazing",
        "love",
        "happy",
        "wonderful",
        "fantastic",
        "positive",
        "best",
        "awesome",
        "brilliant",
        "perfect",
        "nice",
        "beautiful",
        "joy",
        "grateful",
        "excited",
        "pleased",
        "superb",
    ];
    let neg_words = [
        "bad",
        "terrible",
        "awful",
        "hate",
        "sad",
        "horrible",
        "negative",
        "worst",
        "poor",
        "ugly",
        "angry",
        "frustrated",
        "disappoint",
        "fail",
        "broken",
        "wrong",
        "error",
        "problem",
        "issue",
        "concern",
    ];
    let words: Vec<&str> = lower.split_whitespace().collect();
    let total = words.len().max(1) as f64;
    let pos = words.iter().filter(|w| pos_words.contains(w)).count() as f64 / total;
    let neg = words.iter().filter(|w| neg_words.contains(w)).count() as f64 / total;
    (pos, neg)
}

pub struct SummarizeTextTool;
#[async_trait]
impl Tool for SummarizeTextTool {
    fn name(&self) -> &str {
        "summarize_text"
    }
    fn description(&self) -> &str {
        "Extract the top N most informative sentences from a text as a summary."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let text = args["text"].as_str().ok_or("Missing 'text'")?;
        let n = args["sentences"].as_u64().unwrap_or(3) as usize;
        let sentences: Vec<&str> = text
            .split(|c| c == '.' || c == '!' || c == '?')
            .map(str::trim)
            .filter(|s| s.len() > 20)
            .collect();
        let selected: Vec<&str> = sentences
            .iter()
            .step_by((sentences.len() / n.max(1)).max(1))
            .take(n)
            .cloned()
            .collect();
        Ok(ToolResult::json(
            &json!({ "summary": selected.join(". "), "sentence_count": selected.len() }),
        ))
    }
}

pub struct TextDiffTool;
#[async_trait]
impl Tool for TextDiffTool {
    fn name(&self) -> &str {
        "text_diff"
    }
    fn description(&self) -> &str {
        "Compute a line-level diff between two text strings."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let a = args["a"].as_str().ok_or("Missing 'a'")?;
        let b = args["b"].as_str().ok_or("Missing 'b'")?;
        let a_lines: Vec<&str> = a.lines().collect();
        let b_lines: Vec<&str> = b.lines().collect();
        let mut diff = Vec::new();
        let mut ai = 0;
        let mut bi = 0;
        while ai < a_lines.len() || bi < b_lines.len() {
            match (a_lines.get(ai), b_lines.get(bi)) {
                (Some(la), Some(lb)) if la == lb => {
                    diff.push(format!("  {la}"));
                    ai += 1;
                    bi += 1;
                }
                (Some(la), _) => {
                    diff.push(format!("- {la}"));
                    ai += 1;
                }
                (_, Some(lb)) => {
                    diff.push(format!("+ {lb}"));
                    bi += 1;
                }
                _ => break,
            }
        }
        Ok(ToolResult::json(
            &json!({ "diff": diff.join("\n"), "lines_added": diff.iter().filter(|l| l.starts_with('+')).count(), "lines_removed": diff.iter().filter(|l| l.starts_with('-')).count() }),
        ))
    }
}

pub struct WordCountTool;
#[async_trait]
impl Tool for WordCountTool {
    fn name(&self) -> &str {
        "word_count"
    }
    fn description(&self) -> &str {
        "Count words, characters, sentences, and paragraphs in a text."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let text = args["text"].as_str().ok_or("Missing 'text'")?;
        let words = text.split_whitespace().count();
        let chars = text.chars().count();
        let chars_no_space = text.chars().filter(|c| !c.is_whitespace()).count();
        let sentences = text.matches(|c| c == '.' || c == '!' || c == '?').count();
        let paragraphs = text.split("\n\n").filter(|s| !s.trim().is_empty()).count();
        let reading_mins = (words as f64 / 200.0 * 10.0).round() / 10.0;
        Ok(ToolResult::json(
            &json!({ "words": words, "characters": chars, "characters_no_space": chars_no_space, "sentences": sentences, "paragraphs": paragraphs, "reading_minutes": reading_mins }),
        ))
    }
}

pub struct ReadingLevelTool;
#[async_trait]
impl Tool for ReadingLevelTool {
    fn name(&self) -> &str {
        "reading_level"
    }
    fn description(&self) -> &str {
        "Estimate Flesch-Kincaid reading grade level and ease score for text."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let text = args["text"].as_str().ok_or("Missing 'text'")?;
        let words: Vec<&str> = text.split_whitespace().collect();
        let word_count = words.len().max(1);
        let sentence_count = text
            .matches(|c| c == '.' || c == '!' || c == '?')
            .count()
            .max(1);
        let syllable_count: usize = words.iter().map(|w| count_syllables(w)).sum();
        let asl = word_count as f64 / sentence_count as f64;
        let asw = syllable_count as f64 / word_count as f64;
        let ease = 206.835 - 1.015 * asl - 84.6 * asw;
        let grade = 0.39 * asl + 11.8 * asw - 15.59;
        Ok(ToolResult::json(
            &json!({ "flesch_ease": (ease * 10.0).round() / 10.0, "grade_level": (grade * 10.0).round() / 10.0, "avg_sentence_length": (asl * 10.0).round() / 10.0, "avg_syllables_per_word": (asw * 10.0).round() / 10.0 }),
        ))
    }
}

fn count_syllables(word: &str) -> usize {
    let lower = word.to_lowercase();
    let vowels: Vec<char> = lower.chars().filter(|c| "aeiou".contains(*c)).collect();
    let count = vowels.len().max(1);
    if lower.ends_with('e') && count > 1 {
        count - 1
    } else {
        count
    }
}

pub struct MarkdownToHtmlTool;
#[async_trait]
impl Tool for MarkdownToHtmlTool {
    fn name(&self) -> &str {
        "markdown_to_html"
    }
    fn description(&self) -> &str {
        "Convert Markdown text to HTML."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let md = args["markdown"].as_str().ok_or("Missing 'markdown'")?;
        let html = simple_md_to_html(md);
        Ok(ToolResult::json(&json!({ "html": html })))
    }
}

fn simple_md_to_html(md: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    for line in md.lines() {
        if line.starts_with("```") {
            in_code = !in_code;
            if in_code {
                out.push_str("<pre><code>");
            } else {
                out.push_str("</code></pre>\n");
            }
            continue;
        }
        if in_code {
            out.push_str(&html_escape(line));
            out.push('\n');
            continue;
        }
        let line = if let Some(rest) = line.strip_prefix("### ") {
            format!("<h3>{rest}</h3>")
        } else if let Some(rest) = line.strip_prefix("## ") {
            format!("<h2>{rest}</h2>")
        } else if let Some(rest) = line.strip_prefix("# ") {
            format!("<h1>{rest}</h1>")
        } else if let Some(rest) = line.strip_prefix("- ") {
            format!("<li>{rest}</li>")
        } else if line.is_empty() {
            "<br>".to_string()
        } else {
            format!("<p>{line}</p>")
        };
        out.push_str(&line);
        out.push('\n');
    }
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub struct HtmlToMarkdownTool;
#[async_trait]
impl Tool for HtmlToMarkdownTool {
    fn name(&self) -> &str {
        "html_to_markdown"
    }
    fn description(&self) -> &str {
        "Convert HTML to readable Markdown."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let html = args["html"].as_str().ok_or("Missing 'html'")?;
        let md = html_to_md(html);
        Ok(ToolResult::json(&json!({ "markdown": md })))
    }
}

fn html_to_md(html: &str) -> String {
    let s = html
        .replace("<h1>", "# ")
        .replace("</h1>", "\n")
        .replace("<h2>", "## ")
        .replace("</h2>", "\n")
        .replace("<h3>", "### ")
        .replace("</h3>", "\n")
        .replace("<b>", "**")
        .replace("</b>", "**")
        .replace("<strong>", "**")
        .replace("</strong>", "**")
        .replace("<em>", "*")
        .replace("</em>", "*")
        .replace("<li>", "- ")
        .replace("</li>", "\n")
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<p>", "")
        .replace("</p>", "\n\n");
    strip_xml_tags(&s)
}

pub struct TemplateRenderTool;
#[async_trait]
impl Tool for TemplateRenderTool {
    fn name(&self) -> &str {
        "template_render"
    }
    fn description(&self) -> &str {
        "Render a Mustache-style template string with a variables object ({{var}} syntax)."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let template = args["template"].as_str().ok_or("Missing 'template'")?;
        let vars = args.get("vars").ok_or("Missing 'vars'")?;
        let rendered = render_template(template, vars);
        Ok(ToolResult::json(&json!({ "rendered": rendered })))
    }
}

fn render_template(template: &str, vars: &Value) -> String {
    let mut out = template.to_string();
    if let Value::Object(map) = vars {
        for (k, v) in map {
            let val = match v {
                Value::String(s) => s.clone(),
                _ => v.to_string(),
            };
            out = out.replace(&format!("{{{{{k}}}}}"), &val);
        }
    }
    out
}

// ── Math / Statistics ─────────────────────────────────────────────────────────

pub struct MathEvalTool;
#[async_trait]
impl Tool for MathEvalTool {
    fn name(&self) -> &str {
        "math_eval"
    }
    fn description(&self) -> &str {
        "Evaluate a simple arithmetic expression (supports +,-,*,/,^,%)."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let expr = args["expression"].as_str().ok_or("Missing 'expression'")?;
        let result = eval_expr(expr).map_err(|e| e)?;
        Ok(ToolResult::json(
            &json!({ "expression": expr, "result": result }),
        ))
    }
}

fn eval_expr(expr: &str) -> Result<f64, String> {
    // Tokenize and evaluate a simple infix expression
    let tokens: Vec<&str> = expr.split_whitespace().collect();
    if tokens.len() == 1 {
        return tokens[0]
            .parse::<f64>()
            .map_err(|_| format!("Cannot parse: {}", tokens[0]));
    }
    if tokens.len() == 3 {
        let a: f64 = tokens[0].parse().map_err(|_| "Invalid number")?;
        let b: f64 = tokens[2].parse().map_err(|_| "Invalid number")?;
        return match tokens[1] {
            "+" => Ok(a + b),
            "-" => Ok(a - b),
            "*" => Ok(a * b),
            "/" => {
                if b == 0.0 {
                    Err("Division by zero".into())
                } else {
                    Ok(a / b)
                }
            }
            "^" | "**" => Ok(a.powf(b)),
            "%" => Ok(a % b),
            op => Err(format!("Unknown operator: {op}")),
        };
    }
    Err("Complex expressions not supported — use 'a op b' form".into())
}

pub struct StatisticsTool;
#[async_trait]
impl Tool for StatisticsTool {
    fn name(&self) -> &str {
        "statistics"
    }
    fn description(&self) -> &str {
        "Compute descriptive statistics for an array of numbers (mean, median, std, min, max, percentiles)."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let vals: Vec<f64> = args["numbers"]
            .as_array()
            .ok_or("Missing 'numbers' array")?
            .iter()
            .filter_map(|v| v.as_f64())
            .collect();
        if vals.is_empty() {
            return Err("Empty array".into());
        }
        let n = vals.len() as f64;
        let sum: f64 = vals.iter().sum();
        let mean = sum / n;
        let mut sorted = vals.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = if sorted.len() % 2 == 0 {
            (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
        } else {
            sorted[sorted.len() / 2]
        };
        let variance = vals.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
        let std_dev = variance.sqrt();
        let p25 = sorted[(sorted.len() as f64 * 0.25) as usize];
        let p75 = sorted[(sorted.len() as f64 * 0.75).min(sorted.len() as f64 - 1.0) as usize];
        Ok(ToolResult::json(
            &json!({ "count": vals.len(), "mean": mean, "median": median, "std_dev": std_dev, "variance": variance, "min": sorted[0], "max": *sorted.last().unwrap(), "p25": p25, "p75": p75, "sum": sum }),
        ))
    }
}

pub struct UnitConvertTool;
#[async_trait]
impl Tool for UnitConvertTool {
    fn name(&self) -> &str {
        "unit_convert"
    }
    fn description(&self) -> &str {
        "Convert between units: length (m/ft/mi/km/in/cm), weight (kg/lb/g/oz), temp (C/F/K), speed (mph/kph/mps), data (B/KB/MB/GB/TB)."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let value = args["value"].as_f64().ok_or("Missing 'value'")?;
        let from = args["from"]
            .as_str()
            .ok_or("Missing 'from'")?
            .to_lowercase();
        let to = args["to"].as_str().ok_or("Missing 'to'")?.to_lowercase();
        let result = unit_convert(value, &from, &to)?;
        Ok(ToolResult::json(
            &json!({ "value": value, "from": from, "to": to, "result": result }),
        ))
    }
}

fn unit_convert(v: f64, from: &str, to: &str) -> Result<f64, String> {
    // Normalise to SI base unit first
    let (base, category) = match from {
        "m" | "meter" | "meters" => (v, "length"),
        "km" | "kilometer" => (v * 1000.0, "length"),
        "cm" | "centimeter" => (v / 100.0, "length"),
        "mm" | "millimeter" => (v / 1000.0, "length"),
        "ft" | "foot" | "feet" => (v * 0.3048, "length"),
        "in" | "inch" | "inches" => (v * 0.0254, "length"),
        "mi" | "mile" | "miles" => (v * 1609.344, "length"),
        "yd" | "yard" => (v * 0.9144, "length"),
        "kg" | "kilogram" => (v, "mass"),
        "g" | "gram" => (v / 1000.0, "mass"),
        "lb" | "pound" | "pounds" => (v * 0.453592, "mass"),
        "oz" | "ounce" => (v * 0.0283495, "mass"),
        "c" | "celsius" => (v, "temp"),
        "f" | "fahrenheit" => ((v - 32.0) * 5.0 / 9.0, "temp"),
        "k" | "kelvin" => (v - 273.15, "temp"),
        "mps" | "m/s" => (v, "speed"),
        "kph" | "km/h" => (v / 3.6, "speed"),
        "mph" => (v * 0.44704, "speed"),
        "knot" | "knots" => (v * 0.514444, "speed"),
        "b" | "byte" | "bytes" => (v, "data"),
        "kb" | "kilobyte" => (v * 1024.0, "data"),
        "mb" | "megabyte" => (v * 1048576.0, "data"),
        "gb" | "gigabyte" => (v * 1073741824.0, "data"),
        "tb" | "terabyte" => (v * 1099511627776.0, "data"),
        _ => return Err(format!("Unknown unit: {from}")),
    };
    let result = match (to, category) {
        ("m" | "meter" | "meters", "length") => base,
        ("km" | "kilometer", "length") => base / 1000.0,
        ("cm" | "centimeter", "length") => base * 100.0,
        ("mm" | "millimeter", "length") => base * 1000.0,
        ("ft" | "foot" | "feet", "length") => base / 0.3048,
        ("in" | "inch" | "inches", "length") => base / 0.0254,
        ("mi" | "mile" | "miles", "length") => base / 1609.344,
        ("yd" | "yard", "length") => base / 0.9144,
        ("kg" | "kilogram", "mass") => base,
        ("g" | "gram", "mass") => base * 1000.0,
        ("lb" | "pound" | "pounds", "mass") => base / 0.453592,
        ("oz" | "ounce", "mass") => base / 0.0283495,
        ("c" | "celsius", "temp") => base,
        ("f" | "fahrenheit", "temp") => base * 9.0 / 5.0 + 32.0,
        ("k" | "kelvin", "temp") => base + 273.15,
        ("mps" | "m/s", "speed") => base,
        ("kph" | "km/h", "speed") => base * 3.6,
        ("mph", "speed") => base / 0.44704,
        ("knot" | "knots", "speed") => base / 0.514444,
        ("b" | "byte" | "bytes", "data") => base,
        ("kb" | "kilobyte", "data") => base / 1024.0,
        ("mb" | "megabyte", "data") => base / 1048576.0,
        ("gb" | "gigabyte", "data") => base / 1073741824.0,
        ("tb" | "terabyte", "data") => base / 1099511627776.0,
        _ => return Err(format!("Cannot convert {category} to '{to}'")),
    };
    Ok((result * 1e10).round() / 1e10)
}

// ── Crypto / Encoding ─────────────────────────────────────────────────────────

pub struct HashTool;
#[async_trait]
impl Tool for HashTool {
    fn name(&self) -> &str {
        "hash"
    }
    fn description(&self) -> &str {
        "Hash a string using SHA-256, SHA-1, or MD5."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let input = args["input"].as_str().ok_or("Missing 'input'")?;
        let algorithm = args["algorithm"].as_str().unwrap_or("sha256");
        use sha2::Digest;
        let hash = match algorithm {
            "sha256" => {
                let mut h = sha2::Sha256::new();
                h.update(input.as_bytes());
                format!("{:x}", h.finalize())
            }
            "sha512" => {
                let mut h = sha2::Sha512::new();
                h.update(input.as_bytes());
                format!("{:x}", h.finalize())
            }
            "sha1" => sha1_hex(input.as_bytes()),
            _ => return Err(format!("Unknown algorithm: {algorithm}")),
        };
        Ok(ToolResult::json(
            &json!({ "input": input, "algorithm": algorithm, "hash": hash }),
        ))
    }
}

fn sha1_hex(data: &[u8]) -> String {
    // Minimal SHA-1 for non-security purposes (tool use only)
    let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
    let mut msg = data.to_vec();
    let orig_len = data.len() as u64 * 8;
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&orig_len.to_be_bytes());
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for i in 0..80 {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let t = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

pub struct Base64Tool;
#[async_trait]
impl Tool for Base64Tool {
    fn name(&self) -> &str {
        "base64"
    }
    fn description(&self) -> &str {
        "Encode or decode a string using Base64."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let input = args["input"].as_str().ok_or("Missing 'input'")?;
        let decode = args["decode"].as_bool().unwrap_or(false);
        if decode {
            let bytes = base64_decode(input)?;
            let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
            Ok(ToolResult::json(
                &json!({ "result": text, "operation": "decode" }),
            ))
        } else {
            Ok(ToolResult::json(
                &json!({ "result": base64_encode(input.as_bytes()), "operation": "encode" }),
            ))
        }
    }
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b = [
            chunk.first().copied().unwrap_or(0),
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        out.push(CHARS[((n >> 18) & 63) as usize] as char);
        out.push(CHARS[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            CHARS[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            CHARS[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    const TABLE: [i8; 256] = {
        let mut t = [-1i8; 256];
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0usize;
        while i < chars.len() {
            t[chars[i] as usize] = i as i8;
            i += 1;
        }
        t
    };
    let input = input.trim().replace('\n', "");
    if input.len() % 4 != 0 {
        return Err("Invalid base64 length".into());
    }
    let mut out = Vec::new();
    for chunk in input.as_bytes().chunks(4) {
        let [a, b, c, d] = [
            TABLE[chunk[0] as usize],
            TABLE[chunk[1] as usize],
            TABLE[chunk[2] as usize],
            TABLE[chunk[3] as usize],
        ];
        if a < 0 || b < 0 {
            return Err("Invalid base64 character".into());
        }
        let n = (a as u32) << 18 | (b as u32) << 12 | (c.max(0) as u32) << 6 | d.max(0) as u32;
        out.push((n >> 16) as u8);
        if c >= 0 {
            out.push((n >> 8) as u8);
        }
        if d >= 0 {
            out.push(n as u8);
        }
    }
    Ok(out)
}

pub struct UuidTool;
#[async_trait]
impl Tool for UuidTool {
    fn name(&self) -> &str {
        "uuid_generate"
    }
    fn description(&self) -> &str {
        "Generate one or more UUID v4 values."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let count = args["count"].as_u64().unwrap_or(1).min(100) as usize;
        let uuids: Vec<String> = (0..count)
            .map(|_| uuid::Uuid::new_v4().to_string())
            .collect();
        Ok(ToolResult::json(&json!({ "uuids": uuids })))
    }
}

pub struct RandomBytesTool;
#[async_trait]
impl Tool for RandomBytesTool {
    fn name(&self) -> &str {
        "random_bytes"
    }
    fn description(&self) -> &str {
        "Generate N cryptographically-random bytes as a hex string."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let n = args["count"].as_u64().unwrap_or(16).min(1024) as usize;
        use rand::RngCore;
        let mut bytes = vec![0u8; n];
        rand::thread_rng().fill_bytes(&mut bytes);
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        Ok(ToolResult::json(&json!({ "bytes": hex, "count": n })))
    }
}

// ── Time / Date ───────────────────────────────────────────────────────────────

pub struct TimeTool;
#[async_trait]
impl Tool for TimeTool {
    fn name(&self) -> &str {
        "current_time"
    }
    fn description(&self) -> &str {
        "Return the current UTC time and Unix timestamp."
    }
    async fn run(&self, _args: &Value) -> Result<ToolResult, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let ts = now.as_secs();
        let ms = now.as_millis();
        // Format as ISO-8601 (seconds precision)
        let secs = ts;
        let s = secs % 60;
        let m = (secs / 60) % 60;
        let h = (secs / 3600) % 24;
        let days = secs / 86400;
        let (y, mo, d) = days_to_ymd(days);
        Ok(ToolResult::json(
            &json!({ "unix_seconds": ts, "unix_ms": ms, "iso8601": format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z") }),
        ))
    }
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let mut d = days + 719468;
    let era = d / 146097;
    d %= 146097;
    let yoe = (d - d / 1460 + d / 36524 - d / 146096) / 365;
    let y = yoe + era * 400;
    let doy = d - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    (year, month, day)
}

pub struct DurationCalcTool;
#[async_trait]
impl Tool for DurationCalcTool {
    fn name(&self) -> &str {
        "duration_calc"
    }
    fn description(&self) -> &str {
        "Convert a duration in seconds to human-readable breakdown (days, hours, minutes, seconds)."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let secs = args["seconds"].as_f64().ok_or("Missing 'seconds'")? as u64;
        let days = secs / 86400;
        let hours = (secs % 86400) / 3600;
        let mins = (secs % 3600) / 60;
        let s = secs % 60;
        let human = match (days, hours, mins, s) {
            (0, 0, 0, s) => format!("{s}s"),
            (0, 0, m, s) => format!("{m}m {s}s"),
            (0, h, m, s) => format!("{h}h {m}m {s}s"),
            (d, h, m, s) => format!("{d}d {h}h {m}m {s}s"),
        };
        Ok(ToolResult::json(
            &json!({ "days": days, "hours": hours, "minutes": mins, "seconds": s, "human": human, "total_seconds": secs }),
        ))
    }
}

pub struct TimestampParseTool;
#[async_trait]
impl Tool for TimestampParseTool {
    fn name(&self) -> &str {
        "timestamp_parse"
    }
    fn description(&self) -> &str {
        "Parse a Unix timestamp (seconds or ms) and return an ISO-8601 string."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let ts = args["timestamp"].as_i64().ok_or("Missing 'timestamp'")?;
        let secs = if ts > 1_000_000_000_000 {
            ts / 1000
        } else {
            ts
        } as u64;
        let s = secs % 60;
        let m = (secs / 60) % 60;
        let h = (secs / 3600) % 24;
        let (y, mo, d) = days_to_ymd(secs / 86400);
        Ok(ToolResult::json(
            &json!({ "iso8601": format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z"), "unix_seconds": secs }),
        ))
    }
}

// ── File System ───────────────────────────────────────────────────────────────

pub struct FileReadTool;
#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }
    fn description(&self) -> &str {
        "Read a local file (text) and return its contents."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let path = args["path"].as_str().ok_or("Missing 'path'")?;
        let max_bytes = args["max_bytes"].as_u64().unwrap_or(1_048_576) as usize;
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| e.to_string())?;
        let truncated = content.len() > max_bytes;
        let content = if truncated {
            content[..max_bytes].to_string()
        } else {
            content
        };
        Ok(ToolResult::json(
            &json!({ "content": content, "size": content.len(), "truncated": truncated, "path": path }),
        ))
    }
}

pub struct FileWriteTool;
#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }
    fn description(&self) -> &str {
        "Write text content to a local file (creates parent directories as needed)."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let path = args["path"].as_str().ok_or("Missing 'path'")?;
        let content = args["content"].as_str().ok_or("Missing 'content'")?;
        let p = std::path::Path::new(path);
        if let Some(parent) = p.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| e.to_string())?;
        }
        tokio::fs::write(path, content)
            .await
            .map_err(|e| e.to_string())?;
        Ok(ToolResult::json(
            &json!({ "path": path, "bytes_written": content.len() }),
        ))
    }
}

pub struct FileSearchTool;
#[async_trait]
impl Tool for FileSearchTool {
    fn name(&self) -> &str {
        "file_search"
    }
    fn description(&self) -> &str {
        "Search file contents recursively in a directory for a pattern (case-insensitive substring match)."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let dir = args["directory"].as_str().unwrap_or(".");
        let pattern = args["pattern"].as_str().ok_or("Missing 'pattern'")?;
        let lower = pattern.to_lowercase();
        let max_results = args["max_results"].as_u64().unwrap_or(20) as usize;
        let mut matches = Vec::new();
        search_files_recursive(std::path::Path::new(dir), &lower, &mut matches, max_results);
        Ok(ToolResult::json(
            &json!({ "matches": matches, "directory": dir, "pattern": pattern }),
        ))
    }
}

fn search_files_recursive(
    dir: &std::path::Path,
    pattern: &str,
    results: &mut Vec<Value>,
    max: usize,
) {
    if results.len() >= max {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if results.len() >= max {
            return;
        }
        let path = entry.path();
        if path.is_dir() {
            search_files_recursive(&path, pattern, results, max);
        } else if let Ok(content) = std::fs::read_to_string(&path) {
            for (i, line) in content.lines().enumerate() {
                if line.to_lowercase().contains(pattern) {
                    results.push(json!({ "file": path.to_string_lossy(), "line": i + 1, "content": line.trim() }));
                    if results.len() >= max {
                        return;
                    }
                }
            }
        }
    }
}

pub struct FileStatTool;
#[async_trait]
impl Tool for FileStatTool {
    fn name(&self) -> &str {
        "file_stat"
    }
    fn description(&self) -> &str {
        "Get file metadata: size, created, modified timestamps, is_dir."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let path = args["path"].as_str().ok_or("Missing 'path'")?;
        let meta = tokio::fs::metadata(path).await.map_err(|e| e.to_string())?;
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Ok(ToolResult::json(
            &json!({ "path": path, "size_bytes": meta.len(), "is_dir": meta.is_dir(), "is_file": meta.is_file(), "modified_unix": modified }),
        ))
    }
}

pub struct DirectoryListTool;
#[async_trait]
impl Tool for DirectoryListTool {
    fn name(&self) -> &str {
        "directory_list"
    }
    fn description(&self) -> &str {
        "List files and directories at a given path with sizes and types."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let path = args["path"].as_str().unwrap_or(".");
        let mut entries = Vec::new();
        let mut dir = tokio::fs::read_dir(path).await.map_err(|e| e.to_string())?;
        while let Ok(Some(entry)) = dir.next_entry().await {
            let meta = entry.metadata().await.ok();
            entries.push(json!({
                "name": entry.file_name().to_string_lossy(),
                "is_dir": meta.as_ref().map(|m| m.is_dir()).unwrap_or(false),
                "size_bytes": meta.as_ref().map(|m| m.len()).unwrap_or(0),
            }));
        }
        Ok(ToolResult::json(
            &json!({ "path": path, "entries": entries, "count": entries.len() }),
        ))
    }
}

// ── System / Process ──────────────────────────────────────────────────────────

pub struct EnvVarTool;
#[async_trait]
impl Tool for EnvVarTool {
    fn name(&self) -> &str {
        "env_var"
    }
    fn description(&self) -> &str {
        "Read an environment variable by name."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let name = args["name"].as_str().ok_or("Missing 'name'")?;
        let value = std::env::var(name).ok();
        Ok(ToolResult::json(
            &json!({ "name": name, "value": value, "exists": value.is_some() }),
        ))
    }
}

pub struct ExtSystemInfoTool;
#[async_trait]
impl Tool for ExtSystemInfoTool {
    fn name(&self) -> &str {
        "system_info_ext"
    }
    fn description(&self) -> &str {
        "Return OS, arch, CPU count, and available memory."
    }
    async fn run(&self, _args: &Value) -> Result<ToolResult, String> {
        use sysinfo::System;
        let mut sys = System::new_all();
        sys.refresh_all();
        Ok(ToolResult::json(&json!({
            "os":              System::name().unwrap_or_default(),
            "os_version":      System::os_version().unwrap_or_default(),
            "kernel":          System::kernel_version().unwrap_or_default(),
            "arch":            std::env::consts::ARCH,
            "cpu_count":       sys.cpus().len(),
            "total_memory_mb": sys.total_memory() / 1024 / 1024,
            "free_memory_mb":  sys.free_memory() / 1024 / 1024,
            "hostname":        System::host_name().unwrap_or_default(),
        })))
    }
}

pub struct ProcessListTool;
#[async_trait]
impl Tool for ProcessListTool {
    fn name(&self) -> &str {
        "process_list"
    }
    fn description(&self) -> &str {
        "List running processes matching a name filter."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        use sysinfo::{ProcessRefreshKind, RefreshKind, System};
        let filter = args["filter"].as_str().unwrap_or("").to_lowercase();
        let mut sys = System::new_with_specifics(
            RefreshKind::new().with_processes(ProcessRefreshKind::new()),
        );
        sys.refresh_processes();
        let procs: Vec<Value> = sys.processes().iter()
            .filter(|(_, p)| filter.is_empty() || p.name().to_string().to_lowercase().contains(&filter))
            .take(100)
            .map(|(pid, p)| json!({ "pid": pid.as_u32(), "name": p.name().to_string(), "cpu_percent": p.cpu_usage(), "memory_mb": p.memory() / 1024 / 1024 }))
            .collect();
        Ok(ToolResult::json(
            &json!({ "processes": procs, "count": procs.len() }),
        ))
    }
}

// ── Network ───────────────────────────────────────────────────────────────────

pub struct DnsLookupTool;
#[async_trait]
impl Tool for DnsLookupTool {
    fn name(&self) -> &str {
        "dns_lookup"
    }
    fn description(&self) -> &str {
        "Resolve a hostname to IP addresses using the system DNS resolver."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let host = args["host"].as_str().ok_or("Missing 'host'")?;
        let addrs = tokio::net::lookup_host(format!("{host}:0"))
            .await
            .map_err(|e| e.to_string())?
            .map(|a| a.ip().to_string())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        Ok(ToolResult::json(
            &json!({ "host": host, "addresses": addrs }),
        ))
    }
}

pub struct PortScanTool;
#[async_trait]
impl Tool for PortScanTool {
    fn name(&self) -> &str {
        "port_check"
    }
    fn description(&self) -> &str {
        "Check whether a specific TCP port is open on a host."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let host = args["host"].as_str().ok_or("Missing 'host'")?;
        let port = args["port"].as_u64().ok_or("Missing 'port'")? as u16;
        let addr = format!("{host}:{port}");
        let open = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            tokio::net::TcpStream::connect(&addr),
        )
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false);
        Ok(ToolResult::json(
            &json!({ "host": host, "port": port, "open": open }),
        ))
    }
}

pub struct WhoisTool;
#[async_trait]
impl Tool for WhoisTool {
    fn name(&self) -> &str {
        "whois_lookup"
    }
    fn description(&self) -> &str {
        "Query WHOIS data for a domain over TCP port 43."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let domain = args["domain"].as_str().ok_or("Missing 'domain'")?;
        let server = args["server"].as_str().unwrap_or("whois.iana.org");
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut stream = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            tokio::net::TcpStream::connect(format!("{server}:43")),
        )
        .await
        .map_err(|_| "Timeout")?
        .map_err(|e| e.to_string())?;
        stream
            .write_all(format!("{domain}\r\n").as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        let mut buf = String::new();
        stream
            .read_to_string(&mut buf)
            .await
            .map_err(|e| e.to_string())?;
        Ok(ToolResult::json(
            &json!({ "domain": domain, "server": server, "response": buf }),
        ))
    }
}

// ── Regex ─────────────────────────────────────────────────────────────────────

pub struct RegexTool;
#[async_trait]
impl Tool for RegexTool {
    fn name(&self) -> &str {
        "regex"
    }
    fn description(&self) -> &str {
        "Test a regex pattern against text, extract matches, or replace."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let pattern = args["pattern"].as_str().ok_or("Missing 'pattern'")?;
        let text = args["text"].as_str().ok_or("Missing 'text'")?;
        let op = args["operation"].as_str().unwrap_or("match");
        let re = regex::Regex::new(pattern).map_err(|e| e.to_string())?;
        match op {
            "test" => Ok(ToolResult::json(&json!({ "matches": re.is_match(text) }))),
            "find" => {
                let hits: Vec<&str> = re.find_iter(text).map(|m| m.as_str()).collect();
                Ok(ToolResult::json(
                    &json!({ "matches": hits, "count": hits.len() }),
                ))
            }
            "replace" => {
                let replacement = args["replacement"].as_str().unwrap_or("");
                Ok(ToolResult::json(
                    &json!({ "result": re.replace_all(text, replacement).as_ref() }),
                ))
            }
            "split" => {
                let parts: Vec<&str> = re.split(text).collect();
                Ok(ToolResult::json(&json!({ "parts": parts })))
            }
            _ => Err(format!("Unknown operation: {op}")),
        }
    }
}

// ── Creative / Utility ────────────────────────────────────────────────────────

pub struct QrCodeTool;
#[async_trait]
impl Tool for QrCodeTool {
    fn name(&self) -> &str {
        "qr_code"
    }
    fn description(&self) -> &str {
        "Generate a QR code as a base64-encoded PNG or ASCII art."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let text = args["text"].as_str().ok_or("Missing 'text'")?;
        let format = args["format"].as_str().unwrap_or("ascii");
        let code = qrcode::QrCode::new(text.as_bytes()).map_err(|e| e.to_string())?;
        match format {
            "ascii" => {
                let rendered: String = code
                    .render::<char>()
                    .quiet_zone(false)
                    .module_dimensions(2, 1)
                    .dark_color('█')
                    .light_color(' ')
                    .build();
                Ok(ToolResult::json(
                    &json!({ "format": "ascii", "qr": rendered }),
                ))
            }
            _ => {
                let img = code.render::<image::Luma<u8>>().build();
                let tmp =
                    std::env::temp_dir().join(format!("bonsai_qr_{}.png", uuid::Uuid::new_v4()));
                img.save(&tmp).map_err(|e| e.to_string())?;
                let buf = std::fs::read(&tmp).map_err(|e| e.to_string())?;
                let _ = std::fs::remove_file(&tmp);
                Ok(ToolResult::json(
                    &json!({ "format": "png_base64", "data": base64_encode(&buf) }),
                ))
            }
        }
    }
}

pub struct ColorConvertTool;
#[async_trait]
impl Tool for ColorConvertTool {
    fn name(&self) -> &str {
        "color_convert"
    }
    fn description(&self) -> &str {
        "Convert colors between HEX, RGB, HSL, and CMYK."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let input = args["color"].as_str().ok_or("Missing 'color'")?;
        let (r, g, b) = parse_color(input)?;
        let hex = format!("#{r:02X}{g:02X}{b:02X}");
        let (h, s, l) = rgb_to_hsl(r, g, b);
        let (c, m, yk, k) = rgb_to_cmyk(r, g, b);
        Ok(ToolResult::json(
            &json!({ "hex": hex, "rgb": { "r": r, "g": g, "b": b }, "hsl": { "h": h, "s": s, "l": l }, "cmyk": { "c": c, "m": m, "y": yk, "k": k } }),
        ))
    }
}

fn parse_color(s: &str) -> Result<(u8, u8, u8), String> {
    let s = s.trim().trim_start_matches('#');
    if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).map_err(|_| "Bad hex")?;
        let g = u8::from_str_radix(&s[2..4], 16).map_err(|_| "Bad hex")?;
        let b = u8::from_str_radix(&s[4..6], 16).map_err(|_| "Bad hex")?;
        return Ok((r, g, b));
    }
    if s.starts_with("rgb(") || s.starts_with("rgb ") {
        let nums: Vec<u8> = s
            .trim_start_matches("rgb(")
            .trim_end_matches(')')
            .split(',')
            .filter_map(|n| n.trim().parse().ok())
            .collect();
        if nums.len() >= 3 {
            return Ok((nums[0], nums[1], nums[2]));
        }
    }
    Err(format!("Cannot parse color: {s}"))
}

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let (rf, gf, bf) = (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let l = (max + min) / 2.0;
    let s = if max == min {
        0.0
    } else {
        (max - min) / (1.0 - (2.0 * l - 1.0).abs())
    };
    let h = if max == min {
        0.0
    } else if max == rf {
        60.0 * (((gf - bf) / (max - min)) % 6.0)
    } else if max == gf {
        60.0 * ((bf - rf) / (max - min) + 2.0)
    } else {
        60.0 * ((rf - gf) / (max - min) + 4.0)
    };
    (
        (h + 360.0) % 360.0,
        (s * 1000.0).round() / 10.0,
        (l * 1000.0).round() / 10.0,
    )
}

fn rgb_to_cmyk(r: u8, g: u8, b: u8) -> (f64, f64, f64, f64) {
    let (rf, gf, bf) = (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
    let k = 1.0 - rf.max(gf).max(bf);
    if k == 1.0 {
        return (0.0, 0.0, 0.0, 100.0);
    }
    let c = (1.0 - rf - k) / (1.0 - k);
    let m = (1.0 - gf - k) / (1.0 - k);
    let y = (1.0 - bf - k) / (1.0 - k);
    (
        (c * 100.0).round(),
        (m * 100.0).round(),
        (y * 100.0).round(),
        (k * 100.0).round(),
    )
}

// ── Notes / Productivity ──────────────────────────────────────────────────────

pub struct NoteTool;
#[async_trait]
impl Tool for NoteTool {
    fn name(&self) -> &str {
        "note"
    }
    fn description(&self) -> &str {
        "Save or retrieve a persistent text note by key in ~/.bonsai/notes/."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let key = args["key"].as_str().ok_or("Missing 'key'")?;
        let content = args.get("content");
        let notes_dir = dirs::home_dir().unwrap_or_default().join(".bonsai/notes");
        let _ = tokio::fs::create_dir_all(&notes_dir).await;
        let path = notes_dir.join(format!("{}.txt", key.replace(['/', '\\', ':'], "_")));
        if let Some(Value::String(text)) = content {
            tokio::fs::write(&path, text)
                .await
                .map_err(|e| e.to_string())?;
            Ok(ToolResult::json(
                &json!({ "key": key, "saved": true, "bytes": text.len() }),
            ))
        } else {
            let text = tokio::fs::read_to_string(&path).await.unwrap_or_default();
            Ok(ToolResult::json(
                &json!({ "key": key, "content": text, "exists": !text.is_empty() }),
            ))
        }
    }
}

pub struct TodoTool;
#[async_trait]
impl Tool for TodoTool {
    fn name(&self) -> &str {
        "todo"
    }
    fn description(&self) -> &str {
        "Manage a persistent TODO list: add/complete/list tasks in ~/.bonsai/todo.json."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let op = args["operation"].as_str().unwrap_or("list");
        let path = dirs::home_dir()
            .unwrap_or_default()
            .join(".bonsai/todo.json");
        let mut todos: Vec<Value> = tokio::fs::read_to_string(&path)
            .await
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        match op {
            "add" => {
                let text = args["text"].as_str().ok_or("Missing 'text'")?;
                let id = todos.len() + 1;
                todos.push(json!({ "id": id, "text": text, "done": false }));
                tokio::fs::write(
                    &path,
                    serde_json::to_string_pretty(&todos).unwrap_or_default(),
                )
                .await
                .ok();
                Ok(ToolResult::json(&json!({ "added": id, "text": text })))
            }
            "complete" => {
                let id = args["id"].as_u64().ok_or("Missing 'id'")? as usize;
                if let Some(t) = todos
                    .iter_mut()
                    .find(|t| t["id"].as_u64() == Some(id as u64))
                {
                    t["done"] = Value::Bool(true);
                }
                tokio::fs::write(
                    &path,
                    serde_json::to_string_pretty(&todos).unwrap_or_default(),
                )
                .await
                .ok();
                Ok(ToolResult::json(&json!({ "completed": id })))
            }
            "clear_done" => {
                todos.retain(|t| !t["done"].as_bool().unwrap_or(false));
                tokio::fs::write(
                    &path,
                    serde_json::to_string_pretty(&todos).unwrap_or_default(),
                )
                .await
                .ok();
                Ok(ToolResult::json(&json!({ "remaining": todos.len() })))
            }
            _ => Ok(ToolResult::json(
                &json!({ "todos": todos, "count": todos.len(), "done": todos.iter().filter(|t| t["done"].as_bool().unwrap_or(false)).count() }),
            )),
        }
    }
}

pub struct SlugifyTool;
#[async_trait]
impl Tool for SlugifyTool {
    fn name(&self) -> &str {
        "slugify"
    }
    fn description(&self) -> &str {
        "Convert text to a URL-safe slug (lowercase, hyphens, no special chars)."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let text = args["text"].as_str().ok_or("Missing 'text'")?;
        let slug: String = text
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        Ok(ToolResult::json(&json!({ "slug": slug, "original": text })))
    }
}

pub struct TruncateTool;
#[async_trait]
impl Tool for TruncateTool {
    fn name(&self) -> &str {
        "truncate"
    }
    fn description(&self) -> &str {
        "Truncate a string to a maximum length, appending an ellipsis if needed."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let text = args["text"].as_str().ok_or("Missing 'text'")?;
        let max_len = args["max_length"].as_u64().ok_or("Missing 'max_length'")? as usize;
        let suffix = args["suffix"].as_str().unwrap_or("...");
        let result = if text.len() <= max_len {
            text.to_string()
        } else {
            format!("{}{suffix}", &text[..max_len.saturating_sub(suffix.len())])
        };
        Ok(ToolResult::json(
            &json!({ "result": result, "truncated": text.len() > max_len, "original_length": text.len() }),
        ))
    }
}

pub struct StringCaseTool;
#[async_trait]
impl Tool for StringCaseTool {
    fn name(&self) -> &str {
        "string_case"
    }
    fn description(&self) -> &str {
        "Convert a string to camelCase, snake_case, PascalCase, SCREAMING_SNAKE, kebab-case."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let text = args["text"].as_str().ok_or("Missing 'text'")?;
        let target = args["to"].as_str().unwrap_or("snake_case");
        let words: Vec<String> = text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(|w| w.to_string())
            .collect();
        let result = match target {
            "snake_case" => words
                .iter()
                .map(|w| w.to_lowercase())
                .collect::<Vec<_>>()
                .join("_"),
            "camelCase" => {
                let mut parts = words.iter().map(|w| w.to_lowercase());
                let first = parts.next().unwrap_or_default();
                first
                    + &parts
                        .map(|w| {
                            let mut c = w.chars();
                            c.next()
                                .map(|ch| ch.to_uppercase().to_string() + c.as_str())
                                .unwrap_or_default()
                        })
                        .collect::<String>()
            }
            "PascalCase" => words
                .iter()
                .map(|w| {
                    let mut c = w.chars();
                    c.next()
                        .map(|ch| ch.to_uppercase().to_string() + &c.as_str().to_lowercase())
                        .unwrap_or_default()
                })
                .collect(),
            "SCREAMING_SNAKE" => words
                .iter()
                .map(|w| w.to_uppercase())
                .collect::<Vec<_>>()
                .join("_"),
            "kebab-case" => words
                .iter()
                .map(|w| w.to_lowercase())
                .collect::<Vec<_>>()
                .join("-"),
            _ => return Err(format!("Unknown case: {target}")),
        };
        Ok(ToolResult::json(
            &json!({ "result": result, "from": text, "to": target }),
        ))
    }
}

pub struct LoremIpsumTool;
#[async_trait]
impl Tool for LoremIpsumTool {
    fn name(&self) -> &str {
        "lorem_ipsum"
    }
    fn description(&self) -> &str {
        "Generate Lorem Ipsum placeholder text with a specified word count."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let words = args["words"].as_u64().unwrap_or(50) as usize;
        const BASE: &str = "lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua ut enim ad minim veniam quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur excepteur sint occaecat cupidatat non proident sunt in culpa qui officia deserunt mollit anim id est laborum";
        let base_words: Vec<&str> = BASE.split_whitespace().collect();
        let result: String = (0..words)
            .map(|i| base_words[i % base_words.len()])
            .collect::<Vec<_>>()
            .join(" ");
        let capitalized = {
            let mut c = result.chars();
            c.next()
                .map(|ch| ch.to_uppercase().to_string() + c.as_str())
                .unwrap_or_default()
        };
        Ok(ToolResult::json(
            &json!({ "text": capitalized + ".", "word_count": words }),
        ))
    }
}

pub struct LevenshteinTool;
#[async_trait]
impl Tool for LevenshteinTool {
    fn name(&self) -> &str {
        "levenshtein_distance"
    }
    fn description(&self) -> &str {
        "Compute the Levenshtein edit distance between two strings."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let a = args["a"].as_str().ok_or("Missing 'a'")?;
        let b = args["b"].as_str().ok_or("Missing 'b'")?;
        let dist = levenshtein(a, b);
        let max_len = a.len().max(b.len()) as f64;
        let similarity = if max_len == 0.0 {
            1.0
        } else {
            1.0 - dist as f64 / max_len
        };
        Ok(ToolResult::json(
            &json!({ "distance": dist, "similarity": (similarity * 1000.0).round() / 1000.0 }),
        ))
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut dp = vec![0usize; b.len() + 1];
    for i in 0..=b.len() {
        dp[i] = i;
    }
    for (i, ca) in a.iter().enumerate() {
        let mut prev = i;
        dp[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let temp = dp[j + 1];
            dp[j + 1] = if ca == cb {
                prev
            } else {
                1 + prev.min(dp[j]).min(dp[j + 1])
            };
            prev = temp;
        }
    }
    dp[b.len()]
}

pub struct NumberFormatTool;
#[async_trait]
impl Tool for NumberFormatTool {
    fn name(&self) -> &str {
        "number_format"
    }
    fn description(&self) -> &str {
        "Format a number with thousands separators and decimal places."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let value = args["value"].as_f64().ok_or("Missing 'value'")?;
        let decimals = args["decimals"].as_u64().unwrap_or(2) as usize;
        let separator = args["separator"].as_str().unwrap_or(",");
        let decimal_point = args["decimal_point"].as_str().unwrap_or(".");
        let prefix = args["prefix"].as_str().unwrap_or("");
        let suffix = args["suffix"].as_str().unwrap_or("");
        let formatted = format_number(value, decimals, separator, decimal_point);
        Ok(ToolResult::json(
            &json!({ "formatted": format!("{prefix}{formatted}{suffix}"), "value": value }),
        ))
    }
}

fn format_number(value: f64, decimals: usize, sep: &str, dec_point: &str) -> String {
    let factor = 10f64.powi(decimals as i32);
    let rounded = (value.abs() * factor).round() / factor;
    let int_part = rounded.floor() as i64;
    let frac_part = (rounded - rounded.floor()) * factor;
    let int_str = int_part.to_string();
    let with_seps: String = int_str
        .chars()
        .rev()
        .enumerate()
        .flat_map(|(i, c)| {
            if i > 0 && i % 3 == 0 {
                vec![sep.chars().next().unwrap_or(','), c]
            } else {
                vec![c]
            }
        })
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    let sign = if value < 0.0 { "-" } else { "" };
    if decimals == 0 {
        format!("{sign}{with_seps}")
    } else {
        format!(
            "{sign}{with_seps}{dec_point}{:0>width$}",
            frac_part as u64,
            width = decimals
        )
    }
}

pub struct CurrencyFormatTool;
#[async_trait]
impl Tool for CurrencyFormatTool {
    fn name(&self) -> &str {
        "currency_format"
    }
    fn description(&self) -> &str {
        "Format a number as a currency string (USD, EUR, GBP, JPY, etc.)."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let amount = args["amount"].as_f64().ok_or("Missing 'amount'")?;
        let currency = args["currency"].as_str().unwrap_or("USD").to_uppercase();
        let (symbol, decimals) = match currency.as_str() {
            "USD" => ("$", 2),
            "EUR" => ("€", 2),
            "GBP" => ("£", 2),
            "JPY" => ("¥", 0),
            "CNY" => ("¥", 2),
            "INR" => ("₹", 2),
            "CAD" => ("C$", 2),
            "AUD" => ("A$", 2),
            "CHF" => ("Fr.", 2),
            "KRW" => ("₩", 0),
            "BTC" => ("₿", 8),
            "ETH" => ("Ξ", 6),
            _ => ("", 2),
        };
        let formatted = format_number(amount, decimals, ",", ".");
        Ok(ToolResult::json(
            &json!({ "formatted": format!("{symbol}{formatted}"), "currency": currency, "amount": amount }),
        ))
    }
}

// ── Code / Developer ──────────────────────────────────────────────────────────

pub struct LineLengthCheckerTool;
#[async_trait]
impl Tool for LineLengthCheckerTool {
    fn name(&self) -> &str {
        "line_length_check"
    }
    fn description(&self) -> &str {
        "Find lines exceeding a max width in source code and report their locations."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let code = args["code"].as_str().ok_or("Missing 'code'")?;
        let max_len = args["max_length"].as_u64().unwrap_or(120) as usize;
        let violations: Vec<Value> = code.lines().enumerate()
            .filter(|(_, l)| l.len() > max_len)
            .map(|(i, l)| json!({ "line": i + 1, "length": l.len(), "content": &l[..l.len().min(80)] }))
            .collect();
        Ok(ToolResult::json(
            &json!({ "violations": violations, "count": violations.len(), "max_length": max_len }),
        ))
    }
}

pub struct IndentNormalizeTool;
#[async_trait]
impl Tool for IndentNormalizeTool {
    fn name(&self) -> &str {
        "indent_normalize"
    }
    fn description(&self) -> &str {
        "Convert tabs to spaces (or vice versa) in source code."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let code = args["code"].as_str().ok_or("Missing 'code'")?;
        let to_spaces = args["to_spaces"].as_bool().unwrap_or(true);
        let tab_size = args["tab_size"].as_u64().unwrap_or(4) as usize;
        let result = if to_spaces {
            code.replace('\t', &" ".repeat(tab_size))
        } else {
            let spaces = " ".repeat(tab_size);
            code.lines()
                .map(|l| {
                    let indent = l.len() - l.trim_start().len();
                    let tabs = indent / tab_size;
                    format!("{}{}", "\t".repeat(tabs), l.trim_start())
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        Ok(ToolResult::json(
            &json!({ "result": result, "to_spaces": to_spaces }),
        ))
    }
}

pub struct JsonMinifyTool;
#[async_trait]
impl Tool for JsonMinifyTool {
    fn name(&self) -> &str {
        "json_minify"
    }
    fn description(&self) -> &str {
        "Minify or pretty-print a JSON string."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let json = args["json"].as_str().ok_or("Missing 'json'")?;
        let pretty = args["pretty"].as_bool().unwrap_or(false);
        let parsed: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let result = if pretty {
            serde_json::to_string_pretty(&parsed)
        } else {
            serde_json::to_string(&parsed)
        }
        .map_err(|e| e.to_string())?;
        Ok(ToolResult::json(
            &json!({ "result": result, "original_size": json.len(), "result_size": result.len() }),
        ))
    }
}

pub struct UrlParseTool;
#[async_trait]
impl Tool for UrlParseTool {
    fn name(&self) -> &str {
        "url_parse"
    }
    fn description(&self) -> &str {
        "Parse a URL into its components (scheme, host, path, query params, fragment)."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let url = args["url"].as_str().ok_or("Missing 'url'")?;
        let parsed = parse_url(url);
        Ok(ToolResult::json(&parsed))
    }
}

fn parse_url(url: &str) -> Value {
    let (scheme, rest) = url
        .split_once("://")
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .unwrap_or_default();
    let (authority, path_query) = rest
        .split_once('/')
        .map(|(a, b)| (a.to_string(), format!("/{b}")))
        .unwrap_or_else(|| (rest.clone(), String::new()));
    let (host_port, _) = authority
        .split_once('@')
        .map(|(_, h)| (h, ""))
        .unwrap_or((&authority, ""));
    let (host, port) = host_port
        .split_once(':')
        .map(|(h, p)| (h.to_string(), p.parse::<u16>().ok()))
        .unwrap_or_else(|| (host_port.to_string(), None));
    let (path, query_frag) = path_query
        .split_once('?')
        .map(|(p, q)| (p.to_string(), q.to_string()))
        .unwrap_or_else(|| (path_query, String::new()));
    let (query, fragment) = query_frag
        .split_once('#')
        .map(|(q, f)| (q.to_string(), f.to_string()))
        .unwrap_or_else(|| (query_frag, String::new()));
    let params: serde_json::Map<String, Value> = query
        .split('&')
        .filter(|s| !s.is_empty())
        .filter_map(|kv| {
            kv.split_once('=')
                .map(|(k, v)| (k.to_string(), Value::String(v.to_string())))
        })
        .collect();
    json!({ "scheme": scheme, "host": host, "port": port, "path": path, "query": params, "fragment": if fragment.is_empty() { Value::Null } else { Value::String(fragment) } })
}

pub struct UrlEncodeTool;
#[async_trait]
impl Tool for UrlEncodeTool {
    fn name(&self) -> &str {
        "url_encode"
    }
    fn description(&self) -> &str {
        "URL-encode or decode a string (percent encoding)."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let text = args["text"].as_str().ok_or("Missing 'text'")?;
        let decode = args["decode"].as_bool().unwrap_or(false);
        let result = if decode {
            url_decode(text)
        } else {
            url_encode(text)
        };
        Ok(ToolResult::json(
            &json!({ "result": result, "decode": decode }),
        ))
    }
}

fn url_encode(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_alphanumeric() || "-_.~".contains(c) {
                vec![c]
            } else {
                format!("%{:02X}", c as u32).chars().collect()
            }
        })
        .collect()
}

fn url_decode(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h: String = chars.by_ref().take(2).collect();
            if let Ok(b) = u8::from_str_radix(&h, 16) {
                out.push(b as char);
            }
        } else if c == '+' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

// ── Image ─────────────────────────────────────────────────────────────────────

pub struct ImageInfoTool;
#[async_trait]
impl Tool for ImageInfoTool {
    fn name(&self) -> &str {
        "image_info"
    }
    fn description(&self) -> &str {
        "Read image dimensions, format, and color mode from a file path."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let path = args["path"].as_str().ok_or("Missing 'path'")?;
        let bytes = tokio::fs::read(path).await.map_err(|e| e.to_string())?;
        let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
        Ok(ToolResult::json(&json!({
            "path": path,
            "width": img.width(),
            "height": img.height(),
            "color": format!("{:?}", img.color()),
        })))
    }
}

pub struct ImageResizeTool;
#[async_trait]
impl Tool for ImageResizeTool {
    fn name(&self) -> &str {
        "image_resize"
    }
    fn description(&self) -> &str {
        "Resize an image to target dimensions and save to output path."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let input = args["input"].as_str().ok_or("Missing 'input'")?;
        let output = args["output"].as_str().ok_or("Missing 'output'")?;
        let width = args["width"].as_u64().ok_or("Missing 'width'")? as u32;
        let height = args["height"].as_u64().ok_or("Missing 'height'")? as u32;
        let bytes = tokio::fs::read(input).await.map_err(|e| e.to_string())?;
        let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
        let resized = img.resize(width, height, image::imageops::FilterType::Lanczos3);
        resized.save(output).map_err(|e| e.to_string())?;
        Ok(ToolResult::json(
            &json!({ "output": output, "width": resized.width(), "height": resized.height() }),
        ))
    }
}

pub struct ImageCropTool;
#[async_trait]
impl Tool for ImageCropTool {
    fn name(&self) -> &str {
        "image_crop"
    }
    fn description(&self) -> &str {
        "Crop an image to a rectangle and save to output path."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let input = args["input"].as_str().ok_or("Missing 'input'")?;
        let output = args["output"].as_str().ok_or("Missing 'output'")?;
        let x = args["x"].as_u64().unwrap_or(0) as u32;
        let y = args["y"].as_u64().unwrap_or(0) as u32;
        let w = args["width"].as_u64().ok_or("Missing 'width'")? as u32;
        let h = args["height"].as_u64().ok_or("Missing 'height'")? as u32;
        let bytes = tokio::fs::read(input).await.map_err(|e| e.to_string())?;
        let mut img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
        let cropped = img.crop(x, y, w, h);
        cropped.save(output).map_err(|e| e.to_string())?;
        Ok(ToolResult::json(
            &json!({ "output": output, "x": x, "y": y, "width": w, "height": h }),
        ))
    }
}

pub struct ImageGrayscaleTool;
#[async_trait]
impl Tool for ImageGrayscaleTool {
    fn name(&self) -> &str {
        "image_grayscale"
    }
    fn description(&self) -> &str {
        "Convert an image to grayscale and save to output path."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let input = args["input"].as_str().ok_or("Missing 'input'")?;
        let output = args["output"].as_str().ok_or("Missing 'output'")?;
        let bytes = tokio::fs::read(input).await.map_err(|e| e.to_string())?;
        let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
        img.grayscale().save(output).map_err(|e| e.to_string())?;
        Ok(ToolResult::json(&json!({ "output": output })))
    }
}

// ── AI / Prompt Utilities ─────────────────────────────────────────────────────

pub struct TokenCountTool;
#[async_trait]
impl Tool for TokenCountTool {
    fn name(&self) -> &str {
        "token_count"
    }
    fn description(&self) -> &str {
        "Estimate token count for text (approximates ~4 chars/token for English)."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let text = args["text"].as_str().ok_or("Missing 'text'")?;
        let chars = text.chars().count();
        let words = text.split_whitespace().count();
        let tokens = (chars as f64 / 4.0).ceil() as usize;
        let cost_1k = args["cost_per_1k"].as_f64().unwrap_or(0.002);
        let cost = tokens as f64 / 1000.0 * cost_1k;
        Ok(ToolResult::json(
            &json!({ "tokens": tokens, "characters": chars, "words": words, "estimated_cost_usd": (cost * 10000.0).round() / 10000.0 }),
        ))
    }
}

pub struct PromptBuilderTool;
#[async_trait]
impl Tool for PromptBuilderTool {
    fn name(&self) -> &str {
        "prompt_builder"
    }
    fn description(&self) -> &str {
        "Construct a structured LLM prompt with system/user/assistant messages."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let system = args["system"]
            .as_str()
            .unwrap_or("You are a helpful assistant.");
        let user = args["user"].as_str().ok_or("Missing 'user'")?;
        let format = args["format"].as_str().unwrap_or("chatml");
        let result = match format {
            "chatml" => format!("<|im_start|>system\n{system}<|im_end|>\n<|im_start|>user\n{user}<|im_end|>\n<|im_start|>assistant\n"),
            "llama3" => format!("<|start_header_id|>system<|end_header_id|>\n{system}<|eot_id|><|start_header_id|>user<|end_header_id|>\n{user}<|eot_id|><|start_header_id|>assistant<|end_header_id|>\n"),
            "alpaca" => format!("### Instruction:\n{system}\n\n### Input:\n{user}\n\n### Response:\n"),
            _        => format!("System: {system}\n\nUser: {user}\n\nAssistant: "),
        };
        Ok(ToolResult::json(
            &json!({ "prompt": result, "format": format, "system_tokens": system.len()/4, "user_tokens": user.len()/4 }),
        ))
    }
}

pub struct EmbeddingSimilarityTool;
#[async_trait]
impl Tool for EmbeddingSimilarityTool {
    fn name(&self) -> &str {
        "embedding_similarity"
    }
    fn description(&self) -> &str {
        "Compute cosine similarity between two embedding vectors (as JSON float arrays)."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let a: Vec<f64> = args["a"]
            .as_array()
            .ok_or("Missing 'a'")?
            .iter()
            .filter_map(|v| v.as_f64())
            .collect();
        let b: Vec<f64> = args["b"]
            .as_array()
            .ok_or("Missing 'b'")?
            .iter()
            .filter_map(|v| v.as_f64())
            .collect();
        if a.len() != b.len() {
            return Err("Vectors must have same dimension".into());
        }
        let dot: f64 = a.iter().zip(&b).map(|(x, y)| x * y).sum();
        let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
        let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
        let similarity = if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot / (norm_a * norm_b)
        };
        Ok(ToolResult::json(
            &json!({ "cosine_similarity": (similarity * 10000.0).round() / 10000.0, "dimensions": a.len() }),
        ))
    }
}

// ── Registration helper ───────────────────────────────────────────────────────

use crate::tool_registry::Tool as ToolTrait;
use std::sync::Arc;

pub fn all_expanded_tools() -> Vec<Arc<dyn ToolTrait>> {
    vec![
        // Web
        Arc::new(WebFetchTool),
        Arc::new(HttpPostTool),
        Arc::new(HttpHeadersTool),
        // Data
        Arc::new(CsvParseTool),
        Arc::new(JsonTransformTool),
        Arc::new(JsonSchemaTool),
        Arc::new(XmlParseTool),
        Arc::new(JsonMinifyTool),
        // Text
        Arc::new(SentimentTool),
        Arc::new(SummarizeTextTool),
        Arc::new(TextDiffTool),
        Arc::new(WordCountTool),
        Arc::new(ReadingLevelTool),
        Arc::new(MarkdownToHtmlTool),
        Arc::new(HtmlToMarkdownTool),
        Arc::new(TemplateRenderTool),
        Arc::new(SlugifyTool),
        Arc::new(TruncateTool),
        Arc::new(StringCaseTool),
        Arc::new(LoremIpsumTool),
        Arc::new(LevenshteinTool),
        // Math / Stats
        Arc::new(MathEvalTool),
        Arc::new(StatisticsTool),
        Arc::new(UnitConvertTool),
        Arc::new(NumberFormatTool),
        Arc::new(CurrencyFormatTool),
        // Crypto / Encoding
        Arc::new(HashTool),
        Arc::new(Base64Tool),
        Arc::new(UuidTool),
        Arc::new(RandomBytesTool),
        // Time
        Arc::new(TimeTool),
        Arc::new(DurationCalcTool),
        Arc::new(TimestampParseTool),
        // File System
        Arc::new(FileReadTool),
        Arc::new(FileWriteTool),
        Arc::new(FileSearchTool),
        Arc::new(FileStatTool),
        Arc::new(DirectoryListTool),
        // System
        Arc::new(EnvVarTool),
        Arc::new(ExtSystemInfoTool),
        Arc::new(ProcessListTool),
        // Network
        Arc::new(DnsLookupTool),
        Arc::new(PortScanTool),
        Arc::new(WhoisTool),
        // Regex
        Arc::new(RegexTool),
        // Creative
        Arc::new(QrCodeTool),
        Arc::new(ColorConvertTool),
        // Notes / Productivity
        Arc::new(NoteTool),
        Arc::new(TodoTool),
        // Code
        Arc::new(LineLengthCheckerTool),
        Arc::new(IndentNormalizeTool),
        Arc::new(UrlParseTool),
        Arc::new(UrlEncodeTool),
        // Image
        Arc::new(ImageInfoTool),
        Arc::new(ImageResizeTool),
        Arc::new(ImageCropTool),
        Arc::new(ImageGrayscaleTool),
        // AI Utilities
        Arc::new(TokenCountTool),
        Arc::new(PromptBuilderTool),
        Arc::new(EmbeddingSimilarityTool),
    ]
}
