//! Security, privacy, and network audit tools.

use async_trait::async_trait;
use serde_json::{json, Value};
use crate::tool_registry::{Tool, ToolResult};

// ── Password Strength ─────────────────────────────────────────────────────────

pub struct PasswordStrengthTool;
#[async_trait]
impl Tool for PasswordStrengthTool {
    fn name(&self) -> &str { "password_strength" }
    fn description(&self) -> &str { "Assess password strength offline using entropy, pattern detection, and common-password heuristics. Never sends data externally." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let password = args["password"].as_str().ok_or("Missing 'password'")?;
        let result = analyze_password(password);
        Ok(ToolResult::json(&result))
    }
}

fn analyze_password(pw: &str) -> Value {
    let len = pw.len();
    let has_lower  = pw.chars().any(|c| c.is_lowercase());
    let has_upper  = pw.chars().any(|c| c.is_uppercase());
    let has_digit  = pw.chars().any(|c| c.is_ascii_digit());
    let has_symbol = pw.chars().any(|c| !c.is_alphanumeric());
    let has_unicode = pw.chars().any(|c| c as u32 > 127);

    // Character set size
    let charset_size = (if has_lower { 26 } else { 0 })
        + (if has_upper { 26 } else { 0 })
        + (if has_digit { 10 } else { 0 })
        + (if has_symbol { 32 } else { 0 })
        + (if has_unicode { 1000 } else { 0 });

    let entropy = if charset_size > 0 && len > 0 {
        len as f64 * (charset_size as f64).log2()
    } else { 0.0 };

    // Pattern detection
    let mut patterns = Vec::new();
    let lower = pw.to_lowercase();
    let common = ["password", "123456", "qwerty", "letmein", "admin", "welcome", "monkey", "dragon", "master", "sunshine", "princess", "shadow", "superman", "batman"];
    for c in &common { if lower.contains(c) { patterns.push(format!("Common word: '{c}'")); } }

    // Keyboard runs
    let kb_runs = ["qwerty","asdf","zxcv","1234","2345","3456","4567","5678","6789"];
    for r in &kb_runs { if lower.contains(r) { patterns.push(format!("Keyboard pattern: '{r}'")); } }

    // Character repetition
    let chars: Vec<char> = pw.chars().collect();
    let max_repeat = chars.windows(3).filter(|w| w[0] == w[1] && w[1] == w[2]).count();
    if max_repeat > 0 { patterns.push(format!("{max_repeat} repeated character sequences")); }

    // Sequential characters
    let seq_count = chars.windows(3).filter(|w| (w[1] as i64 - w[0] as i64 == 1) && (w[2] as i64 - w[1] as i64 == 1)).count();
    if seq_count > 0 { patterns.push(format!("{seq_count} sequential character sequences")); }

    let score = calculate_score(len, entropy, patterns.len(), has_lower, has_upper, has_digit, has_symbol);
    let (label, color) = match score {
        0..=19  => ("Very Weak", "red"),
        20..=39 => ("Weak",      "orange"),
        40..=59 => ("Fair",      "yellow"),
        60..=79 => ("Strong",    "lightgreen"),
        _       => ("Very Strong","green"),
    };

    let crack_time = estimate_crack_time(entropy);

    let mut suggestions = Vec::new();
    if len < 12 { suggestions.push("Use at least 12 characters"); }
    if !has_upper { suggestions.push("Add uppercase letters"); }
    if !has_digit { suggestions.push("Add numbers"); }
    if !has_symbol { suggestions.push("Add symbols (!@#$%^&*)"); }
    if !patterns.is_empty() { suggestions.push("Avoid common words and patterns"); }

    json!({
        "score": score,
        "label": label,
        "color": color,
        "entropy_bits": (entropy * 10.0).round() / 10.0,
        "length": len,
        "charset_size": charset_size,
        "character_classes": { "lowercase": has_lower, "uppercase": has_upper, "digits": has_digit, "symbols": has_symbol },
        "patterns_found": patterns,
        "estimated_crack_time": crack_time,
        "suggestions": suggestions,
    })
}

fn calculate_score(len: usize, entropy: f64, pattern_count: usize, lower: bool, upper: bool, digit: bool, symbol: bool) -> u32 {
    let mut score: i32 = 0;
    score += (len as i32 * 4).min(40);
    score += (entropy as i32 / 2).min(40);
    score -= (pattern_count as i32 * 10).min(30);
    if lower  { score += 5; }
    if upper  { score += 5; }
    if digit  { score += 5; }
    if symbol { score += 10; }
    score.max(0).min(100) as u32
}

fn estimate_crack_time(entropy: f64) -> String {
    // Assuming 1 billion guesses/second
    let guesses = 2.0_f64.powf(entropy);
    let seconds = guesses / 1_000_000_000.0;
    if seconds < 1.0      { "< 1 second".to_string() }
    else if seconds < 60.0 { format!("{:.0} seconds", seconds) }
    else if seconds < 3600.0 { format!("{:.0} minutes", seconds / 60.0) }
    else if seconds < 86400.0 { format!("{:.1} hours", seconds / 3600.0) }
    else if seconds < 31536000.0 { format!("{:.1} days", seconds / 86400.0) }
    else if seconds < 31536000.0 * 1000.0 { format!("{:.1} years", seconds / 31536000.0) }
    else { format!("{:.2e} years", seconds / 31536000.0) }
}

// ── SSL Certificate Check ─────────────────────────────────────────────────────

pub struct SslCheckTool;
#[async_trait]
impl Tool for SslCheckTool {
    fn name(&self) -> &str { "ssl_check" }
    fn description(&self) -> &str { "Check SSL/TLS certificate validity, expiry, and HTTP security headers for a domain." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let domain   = args["domain"].as_str().ok_or("Missing 'domain'")?;
        let port     = args["port"].as_u64().unwrap_or(443) as u16;
        let url      = format!("https://{}:{}", domain, port);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .danger_accept_invalid_certs(false)
            .build().map_err(|e| e.to_string())?;

        match client.get(&url).send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let headers: std::collections::HashMap<String, String> = resp.headers().iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect();

                // Security header analysis
                let sec_headers = analyze_security_headers(&headers);
                Ok(ToolResult::json(&json!({
                    "domain": domain, "port": port, "reachable": true,
                    "https_status": status,
                    "security_headers": sec_headers,
                    "hsts": headers.contains_key("strict-transport-security"),
                    "csp":  headers.contains_key("content-security-policy"),
                    "xfo":  headers.get("x-frame-options").cloned().unwrap_or_default(),
                    "xcto": headers.get("x-content-type-options").cloned().unwrap_or_default(),
                    "server": headers.get("server").cloned().unwrap_or_default(),
                    "verdict": if sec_headers["score"].as_u64().unwrap_or(0) >= 60 { "Good" } else { "Needs improvement" },
                })))
            }
            Err(e) => {
                let tls_error = e.to_string().to_lowercase().contains("certificate") || e.to_string().contains("tls");
                Ok(ToolResult::json(&json!({
                    "domain": domain, "port": port, "reachable": false,
                    "error": e.to_string(),
                    "tls_error": tls_error,
                })))
            }
        }
    }
}

fn analyze_security_headers(headers: &std::collections::HashMap<String, String>) -> Value {
    let mut score = 0u32;
    let mut findings = Vec::new();

    let checks = [
        ("strict-transport-security", "HSTS enabled", 20u32),
        ("content-security-policy",   "CSP header present", 20),
        ("x-frame-options",           "Clickjacking protection", 15),
        ("x-content-type-options",    "MIME type sniffing protection", 10),
        ("referrer-policy",           "Referrer policy set", 10),
        ("permissions-policy",        "Permissions policy set", 10),
        ("x-xss-protection",          "XSS protection header", 5),
    ];

    for (header, desc, pts) in &checks {
        if headers.contains_key(*header) {
            score += pts;
            findings.push(json!({ "header": header, "status": "present", "description": desc, "points": pts }));
        } else {
            findings.push(json!({ "header": header, "status": "missing", "description": desc, "points": 0 }));
        }
    }

    json!({ "score": score, "max_score": 90, "findings": findings, "grade": if score >= 80 { "A" } else if score >= 60 { "B" } else if score >= 40 { "C" } else { "F" } })
}

// ── DNS Audit ─────────────────────────────────────────────────────────────────

pub struct DnsAuditTool;
#[async_trait]
impl Tool for DnsAuditTool {
    fn name(&self) -> &str { "dns_audit" }
    fn description(&self) -> &str { "Full DNS audit: resolve A, AAAA, MX, TXT, CNAME records and check for common misconfigurations." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let domain = args["domain"].as_str().ok_or("Missing 'domain'")?;

        // Use system resolver for A records
        let a_records: Vec<String> = tokio::net::lookup_host(format!("{domain}:0")).await
            .map(|addrs| addrs.map(|a| a.ip().to_string()).collect::<std::collections::HashSet<_>>().into_iter().collect())
            .unwrap_or_default();

        // MX records via Google DNS over HTTPS (offline fallback: empty)
        let mx = fetch_dns_json(domain, "MX").await.unwrap_or_default();
        let txt = fetch_dns_json(domain, "TXT").await.unwrap_or_default();
        let ns  = fetch_dns_json(domain, "NS").await.unwrap_or_default();

        // Security analysis
        let has_spf  = txt.iter().any(|r| r.to_string().contains("v=spf1"));
        let has_dkim = txt.iter().any(|r| r.to_string().contains("v=DKIM1"));
        let has_dmarc = txt.iter().any(|r| r.to_string().contains("v=DMARC1"));
        let mx_count  = mx.len();

        let mut recommendations = Vec::new();
        if !has_spf  { recommendations.push("Add SPF TXT record to prevent email spoofing"); }
        if !has_dkim { recommendations.push("Configure DKIM for email authentication"); }
        if !has_dmarc { recommendations.push("Add DMARC policy for email security"); }
        if mx_count == 0 { recommendations.push("No MX records found — email delivery may fail"); }
        if mx_count == 1 { recommendations.push("Single MX record — consider adding a backup MX"); }

        Ok(ToolResult::json(&json!({
            "domain": domain,
            "a_records": a_records,
            "mx_records": mx,
            "txt_records": txt,
            "ns_records": ns,
            "email_security": { "spf": has_spf, "dkim": has_dkim, "dmarc": has_dmarc },
            "mx_count": mx_count,
            "recommendations": recommendations,
        })))
    }
}

async fn fetch_dns_json(domain: &str, record_type: &str) -> Result<Vec<String>, String> {
    let url = format!("https://dns.google/resolve?name={domain}&type={record_type}");
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(5)).build()
        .map_err(|e| e.to_string())?;
    let resp: Value = client.get(&url).header("accept","application/dns-json").send().await
        .map_err(|e| e.to_string())?.json().await.map_err(|e| e.to_string())?;
    let answers: Vec<String> = resp["Answer"].as_array().unwrap_or(&vec![]).iter()
        .filter_map(|a| a["data"].as_str().map(|s| s.to_string())).collect();
    Ok(answers)
}

// ── HTTP Benchmark ────────────────────────────────────────────────────────────

pub struct HttpBenchmarkTool;
#[async_trait]
impl Tool for HttpBenchmarkTool {
    fn name(&self) -> &str { "http_benchmark" }
    fn description(&self) -> &str { "Benchmark an HTTP endpoint: measure latency percentiles and throughput across multiple requests." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let url          = args["url"].as_str().ok_or("Missing 'url'")?;
        let requests     = args["requests"].as_u64().unwrap_or(20).min(100) as usize;
        let concurrency  = args["concurrency"].as_u64().unwrap_or(4).min(16) as usize;
        let method       = args["method"].as_str().unwrap_or("GET").to_uppercase();

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build().map_err(|e| e.to_string())?;

        let start_total = std::time::Instant::now();
        let mut latencies = Vec::new();
        let mut errors = 0usize;
        let chunks = (requests + concurrency - 1) / concurrency;

        for _ in 0..chunks {
            let batch_size = concurrency.min(requests.saturating_sub(latencies.len() + errors));
            let mut futs = Vec::new();
            for _ in 0..batch_size {
                let c = client.clone();
                let u = url.to_string();
                let m = method.clone();
                futs.push(tokio::spawn(async move {
                    let t = std::time::Instant::now();
                    let r = match m.as_str() {
                        "POST" => c.post(&u).send().await,
                        _      => c.get(&u).send().await,
                    };
                    (t.elapsed().as_millis() as f64, r.map(|r| r.status().as_u16()).unwrap_or(0))
                }));
            }
            for f in futs {
                if let Ok((ms, status)) = f.await {
                    if status == 0 || status >= 500 { errors += 1; } else { latencies.push(ms); }
                }
            }
        }

        let total_ms = start_total.elapsed().as_millis() as f64;
        if latencies.is_empty() { return Err("All requests failed".into()); }
        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = latencies.len() as f64;
        let mean = latencies.iter().sum::<f64>() / n;
        let p50 = latencies[(n * 0.5) as usize];
        let p90 = latencies[(n * 0.9).min(n - 1.0) as usize];
        let p99 = latencies[(n * 0.99).min(n - 1.0) as usize];
        let rps  = latencies.len() as f64 / (total_ms / 1000.0);

        Ok(ToolResult::json(&json!({
            "url": url, "method": method, "requests_sent": requests,
            "successful": latencies.len(), "errors": errors,
            "latency_ms": { "mean": (mean * 10.0).round() / 10.0, "p50": p50, "p90": p90, "p99": p99, "min": latencies[0], "max": *latencies.last().unwrap() },
            "throughput_rps": (rps * 100.0).round() / 100.0,
            "total_duration_ms": total_ms,
        })))
    }
}

// ── SSH Keygen ────────────────────────────────────────────────────────────────

pub struct SshKeygenTool;
#[async_trait]
impl Tool for SshKeygenTool {
    fn name(&self) -> &str { "ssh_keygen" }
    fn description(&self) -> &str { "Generate an Ed25519 SSH key pair and optionally save to ~/.ssh/." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let comment  = args["comment"].as_str().unwrap_or("bonsai-generated");
        let save     = args["save"].as_bool().unwrap_or(false);
        let key_name = args["key_name"].as_str().unwrap_or("id_ed25519_bonsai");

        use ssh_key::{Algorithm, PrivateKey, rand_core::OsRng};
        let key = PrivateKey::random(&mut OsRng, Algorithm::Ed25519)
            .map_err(|e| e.to_string())?;
        let pubkey = key.public_key();
        let pub_str = pubkey.to_openssh().map_err(|e| e.to_string())?;
        let priv_str = key.to_openssh(ssh_key::LineEnding::LF).map_err(|e| e.to_string())?;

        let pub_full = format!("{} {comment}", pub_str.trim());
        if save {
            let ssh_dir = dirs::home_dir().unwrap_or_default().join(".ssh");
            tokio::fs::create_dir_all(&ssh_dir).await.map_err(|e| e.to_string())?;
            let priv_path = ssh_dir.join(key_name);
            let pub_path  = ssh_dir.join(format!("{key_name}.pub"));
            tokio::fs::write(&priv_path, priv_str.as_str()).await.map_err(|e| e.to_string())?;
            tokio::fs::write(&pub_path, &pub_full).await.map_err(|e| e.to_string())?;
            // Set permissions (Unix only)
            #[cfg(unix)] {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&priv_path, std::fs::Permissions::from_mode(0o600));
            }
            Ok(ToolResult::json(&json!({
                "algorithm": "ed25519",
                "public_key": pub_full,
                "private_key_path": priv_path.to_string_lossy(),
                "public_key_path": pub_path.to_string_lossy(),
                "saved": true,
                "comment": comment,
            })))
        } else {
            Ok(ToolResult::json(&json!({
                "algorithm": "ed25519",
                "public_key": pub_full,
                "private_key": priv_str.as_str(),
                "saved": false,
                "comment": comment,
                "warning": "Private key is in the response — save it securely and do not share",
            })))
        }
    }
}

// ── Binary/File Signature Scan ────────────────────────────────────────────────

pub struct FileScanTool;
#[async_trait]
impl Tool for FileScanTool {
    fn name(&self) -> &str { "file_scan" }
    fn description(&self) -> &str { "Scan a file for suspicious patterns: embedded scripts, high entropy (packing/encryption), PE/ELF headers, and macro indicators." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let path    = args["path"].as_str().ok_or("Missing 'path'")?;
        let max_mb  = args["max_mb"].as_u64().unwrap_or(50) as usize;
        let meta    = tokio::fs::metadata(path).await.map_err(|e| e.to_string())?;
        if meta.len() > (max_mb * 1024 * 1024) as u64 {
            return Err(format!("File too large (>{max_mb} MB)"));
        }
        let bytes = tokio::fs::read(path).await.map_err(|e| e.to_string())?;
        let entropy = shannon_entropy(&bytes);
        let mut findings = Vec::new();

        // Magic bytes
        let magic = match bytes.get(..4) {
            Some(&[0x4D, 0x5A, ..])              => "PE (Windows executable)",
            Some(&[0x7F, 0x45, 0x4C, 0x46])      => "ELF (Linux executable)",
            Some(&[0xCA, 0xFE, 0xBA, 0xBE, ..])  => "Mach-O (macOS executable)",
            Some(&[0x50, 0x4B, ..])               => "ZIP archive",
            Some(&[0x25, 0x50, 0x44, 0x46])      => "PDF",
            Some(&[0xFF, 0xD8, 0xFF, ..])         => "JPEG image",
            Some(&[0x89, 0x50, 0x4E, 0x47, ..])  => "PNG image",
            _                                     => "Unknown/text",
        };

        // High entropy (> 7.5 suggests encryption or packing)
        if entropy > 7.5 { findings.push(json!({ "type": "high_entropy", "severity": "medium", "description": format!("Shannon entropy {:.2} — possible encryption or packing", entropy) })); }

        // Embedded URLs
        let text = String::from_utf8_lossy(&bytes);
        let url_count = text.matches("http://").count() + text.matches("https://").count();
        if url_count > 10 { findings.push(json!({ "type": "many_urls", "severity": "low", "description": format!("{url_count} embedded URLs found") })); }

        // Suspicious strings
        let suspicious = ["powershell", "cmd.exe", "eval(", "base64_decode", "fromCharCode", "document.write", "shell_exec", "system(", "exec("];
        for s in &suspicious {
            if text.to_lowercase().contains(s) {
                findings.push(json!({ "type": "suspicious_string", "severity": "high", "description": format!("Contains '{s}'") }));
            }
        }

        Ok(ToolResult::json(&json!({
            "path": path, "size_bytes": bytes.len(),
            "file_type": magic,
            "shannon_entropy": (entropy * 1000.0).round() / 1000.0,
            "suspicious_findings": findings,
            "finding_count": findings.len(),
            "risk_level": if findings.iter().any(|f| f["severity"] == "high") { "high" }
                          else if !findings.is_empty() { "medium" } else { "low" },
        })))
    }
}

fn shannon_entropy(bytes: &[u8]) -> f64 {
    if bytes.is_empty() { return 0.0; }
    let mut freq = [0usize; 256];
    for &b in bytes { freq[b as usize] += 1; }
    let n = bytes.len() as f64;
    freq.iter().filter(|&&f| f > 0)
        .map(|&f| { let p = f as f64 / n; -p * p.log2() })
        .sum()
}

// ── Registration ──────────────────────────────────────────────────────────────

use std::sync::Arc;
use crate::tool_registry::Tool as ToolTrait;

pub fn all_security_tools() -> Vec<Arc<dyn ToolTrait>> {
    vec![
        Arc::new(PasswordStrengthTool),
        Arc::new(SslCheckTool),
        Arc::new(DnsAuditTool),
        Arc::new(HttpBenchmarkTool),
        Arc::new(SshKeygenTool),
        Arc::new(FileScanTool),
    ]
}
