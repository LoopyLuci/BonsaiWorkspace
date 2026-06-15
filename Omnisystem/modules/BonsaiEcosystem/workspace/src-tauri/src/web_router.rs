//! Trusted Documentation Router — asymmetric web content tiering.
//!
//! Whitelisted domains (docs.rs, svelte.dev, etc.) receive full markdown content.
//! All other domains receive a DreamAgent-generated ≤125-character summary.
//!
//! This keeps the model context clean: trusted documentation is rendered faithfully;
//! untrusted pages are never injected raw into the prompt.
//!
//! The whitelist lives in `config/features.yaml` under `web_router.whitelist`.
//! The DreamAgent summarizer is called at http://127.0.0.1:8082 (configurable).

use std::time::Duration;

use serde::{Deserialize, Serialize};

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebRouterConfig {
    #[serde(default = "default_whitelist")]
    pub whitelist: Vec<String>,
    #[serde(default = "default_max_chars")]
    pub max_excerpt_chars: usize,
    #[serde(default = "default_summarizer_url")]
    pub summarizer_url: String,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
}

fn default_whitelist() -> Vec<String> {
    vec![
        "docs.rs".into(),
        "doc.rust-lang.org".into(),
        "svelte.dev".into(),
        "tauri.app".into(),
        "python.org".into(),
        "developer.mozilla.org".into(),
        "npmjs.com".into(),
        "crates.io".into(),
        "github.com".into(),
        "stackoverflow.com".into(),
    ]
}
fn default_max_chars() -> usize {
    125
}
fn default_summarizer_url() -> String {
    "http://127.0.0.1:8082".into()
}
fn default_timeout_secs() -> u64 {
    15
}

impl Default for WebRouterConfig {
    fn default() -> Self {
        Self {
            whitelist: default_whitelist(),
            max_excerpt_chars: default_max_chars(),
            summarizer_url: default_summarizer_url(),
            timeout_secs: default_timeout_secs(),
        }
    }
}

// ── Result ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebResult {
    pub url: String,
    pub trusted: bool,
    /// Full markdown (trusted) or ≤125-char summary (untrusted).
    pub content: String,
    pub title: Option<String>,
    pub truncated: bool,
}

// ── Router ────────────────────────────────────────────────────────────────────

pub struct WebRouter {
    config: WebRouterConfig,
    client: reqwest::Client,
}

impl WebRouter {
    pub fn new(config: WebRouterConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .user_agent("BonsaiWorkspace/1.0 (documentation fetch)")
            .build()
            .unwrap_or_default();
        Self { config, client }
    }

    pub fn is_trusted(&self, url: &str) -> bool {
        let lower = url.to_lowercase();
        self.config
            .whitelist
            .iter()
            .any(|domain| lower.contains(domain.as_str()))
    }

    /// Fetch a URL and return either full markdown (trusted) or a brief summary.
    pub async fn fetch(&self, url: &str) -> Result<WebResult, String> {
        let html = self.fetch_html(url).await?;
        let title = extract_title(&html);

        if self.is_trusted(url) {
            let markdown = html_to_markdown(&html);
            let truncated = markdown.len() > 32_000;
            let content = if truncated {
                markdown[..32_000].to_string() + "\n\n*(content truncated)*"
            } else {
                markdown
            };
            Ok(WebResult {
                url: url.to_string(),
                trusted: true,
                content,
                title,
                truncated,
            })
        } else {
            let plain = html_to_plain(&html);
            let summary = self.summarize(&plain, url).await;
            Ok(WebResult {
                url: url.to_string(),
                trusted: false,
                content: summary,
                title,
                truncated: false,
            })
        }
    }

    async fn fetch_html(&self, url: &str) -> Result<String, String> {
        self.client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("fetch error: {e}"))?
            .text()
            .await
            .map_err(|e| format!("decode error: {e}"))
    }

    /// Call the DreamAgent summarizer (llama-server on port 8082).
    /// Falls back to a truncated plain-text excerpt if the sidecar is unavailable.
    async fn summarize(&self, plain_text: &str, url: &str) -> String {
        let excerpt = if plain_text.len() > 4_000 {
            &plain_text[..4_000]
        } else {
            plain_text
        };

        let prompt = format!(
            "Summarize this webpage in at most {} characters. URL: {url}\n\nContent:\n{excerpt}",
            self.config.max_excerpt_chars,
        );

        let payload = serde_json::json!({
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": 64,
            "temperature": 0.1,
        });

        match self
            .client
            .post(format!(
                "{}/v1/chat/completions",
                self.config.summarizer_url
            ))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(s) = json["choices"][0]["message"]["content"].as_str() {
                        return s.chars().take(self.config.max_excerpt_chars).collect();
                    }
                }
                // Sidecar responded but JSON parse failed
                excerpt
                    .chars()
                    .take(self.config.max_excerpt_chars)
                    .collect()
            }
            Err(_) => {
                // DreamAgent sidecar not running — use plain excerpt
                excerpt
                    .chars()
                    .take(self.config.max_excerpt_chars)
                    .collect()
            }
        }
    }
}

// ── HTML utilities ────────────────────────────────────────────────────────────

fn extract_title(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let start = lower.find("<title>")? + 7;
    let end = lower.find("</title>")?;
    if start < end {
        Some(html[start..end].trim().to_string())
    } else {
        None
    }
}

/// Minimal HTML → Markdown converter.
/// Handles headings, paragraphs, code blocks, links, and bold/italic.
/// For trusted documentation this is good enough; we don't need a full parser.
fn html_to_markdown(html: &str) -> String {
    // Strip scripts and style blocks (regex crate doesn't support backreferences,
    // so handle each tag separately).
    let re_script = regex::Regex::new(r"(?si)<script[^>]*>.*?</script>").unwrap();
    let re_style = regex::Regex::new(r"(?si)<style[^>]*>.*?</style>").unwrap();
    let stripped = re_script.replace_all(html, "");
    let s = re_style.replace_all(&stripped, "").to_string();

    // Convert common tags
    let rules: &[(&str, &str)] = &[
        (r"(?i)<h1[^>]*>(.*?)</h1>", "# $1\n\n"),
        (r"(?i)<h2[^>]*>(.*?)</h2>", "## $1\n\n"),
        (r"(?i)<h3[^>]*>(.*?)</h3>", "### $1\n\n"),
        (r"(?i)<h4[^>]*>(.*?)</h4>", "#### $1\n\n"),
        (r"(?i)<strong[^>]*>(.*?)</strong>", "**$1**"),
        (r"(?i)<b[^>]*>(.*?)</b>", "**$1**"),
        (r"(?i)<em[^>]*>(.*?)</em>", "*$1*"),
        (r"(?i)<i[^>]*>(.*?)</i>", "*$1*"),
        (r#"(?i)<a[^>]*href="([^"]*)"[^>]*>(.*?)</a>"#, "[$2]($1)"),
        (
            r"(?si)<pre[^>]*><code[^>]*>(.*?)</code></pre>",
            "```\n$1\n```\n",
        ),
        (r"(?si)<code[^>]*>(.*?)</code>", "`$1`"),
        (r"(?i)<li[^>]*>(.*?)</li>", "- $1\n"),
        (r"(?i)<br\s*/?>", "\n"),
        (r"(?i)<p[^>]*>(.*?)</p>", "$1\n\n"),
        (r"<[^>]+>", ""), // strip remaining tags
    ];

    let mut out = s.to_string();
    for (pattern, replacement) in rules {
        if let Ok(re) = regex::Regex::new(pattern) {
            out = re.replace_all(&out, *replacement).to_string();
        }
    }

    // Decode HTML entities
    out = out
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    // Collapse excessive blank lines
    let re_blanks = regex::Regex::new(r"\n{3,}").unwrap();
    re_blanks.replace_all(&out, "\n\n").trim().to_string()
}

/// Strip all HTML tags, leaving only plain text.  Used for untrusted pages.
fn html_to_plain(html: &str) -> String {
    let re = regex::Regex::new(r"<[^>]+>").unwrap_or_else(|_| unreachable!());
    let plain = re.replace_all(html, " ");
    let re_ws = regex::Regex::new(r"\s{2,}").unwrap_or_else(|_| unreachable!());
    re_ws.replace_all(plain.trim(), " ").to_string()
}

// ── Middleware integration ────────────────────────────────────────────────────

use crate::middleware::{MiddlewareOutcome, ToolCall, ToolMiddleware};
use std::sync::Arc;

pub struct WebRouterMiddleware {
    pub router: Arc<WebRouter>,
}

impl ToolMiddleware for WebRouterMiddleware {
    fn name(&self) -> &'static str {
        "web_router"
    }

    fn intercept(&self, call: ToolCall) -> MiddlewareOutcome {
        // Only intercept web_search / browse_url tool calls
        if !matches!(
            call.tool.as_str(),
            "web_search" | "browse_url" | "fetch_url"
        ) {
            return MiddlewareOutcome::Continue(call);
        }

        let url = call.args["url"]
            .as_str()
            .or_else(|| call.args["query"].as_str())
            .unwrap_or("")
            .to_owned();

        if url.is_empty() {
            return MiddlewareOutcome::Continue(call);
        }

        let trusted = self.router.is_trusted(&url);
        let mut rewritten = call;
        rewritten.args["__web_router"] = serde_json::Value::Bool(true);
        rewritten.args["__trusted"] = serde_json::Value::Bool(trusted);
        MiddlewareOutcome::Continue(rewritten)
    }
}
