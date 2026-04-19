pub struct FormattedMessage {
    /// Chunks ready to send in order (each fits within platform limits).
    pub chunks: Vec<String>,
}

impl FormattedMessage {
    fn single(text: impl Into<String>) -> Self {
        Self { chunks: vec![text.into()] }
    }
}

pub fn format(reply: &str, platform: &str) -> FormattedMessage {
    match platform {
        "discord"  => format_discord(reply),
        "telegram" => format_telegram_mdv2(reply),
        "matrix"   => format_matrix_html(reply),
        "email"    => format_email_html(reply),
        _          => FormattedMessage::single(reply),
    }
}

// ── Discord ───────────────────────────────────────────────────────────────────

fn format_discord(reply: &str) -> FormattedMessage {
    // Standard Markdown. Split at 1990 chars preserving paragraph boundaries.
    FormattedMessage { chunks: split_at(reply, 1990) }
}

// ── Telegram MarkdownV2 ───────────────────────────────────────────────────────

fn format_telegram_mdv2(reply: &str) -> FormattedMessage {
    let escaped = escape_telegram_mdv2(reply);
    FormattedMessage { chunks: split_at(&escaped, 4000) }
}

fn escape_telegram_mdv2(s: &str) -> String {
    const SPECIAL: &[char] = &[
        '.', '!', '(', ')', '-', '=', '+', '{', '}', '|', '~', '>', '#', '[', ']', '*', '_', '`', '\\',
    ];
    let mut out = String::with_capacity(s.len() + 64);
    for c in s.chars() {
        if SPECIAL.contains(&c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

// ── Matrix HTML ───────────────────────────────────────────────────────────────

fn format_matrix_html(reply: &str) -> FormattedMessage {
    // Wrap plain text with minimal HTML. In practice matrix-sdk sends m.text + formatted_body.
    let html = format!("<p>{}</p>", html_escape(reply));
    FormattedMessage::single(html)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
}

// ── Email HTML ────────────────────────────────────────────────────────────────

fn format_email_html(reply: &str) -> FormattedMessage {
    let escaped = html_escape(reply).replace('\n', "<br>");
    let html = format!(
        r#"<!DOCTYPE html><html><body style="font-family:sans-serif;max-width:600px;margin:auto">
<div style="background:#2d5016;color:#fff;padding:12px 16px;border-radius:8px 8px 0 0">
  🌿 Bonsai Buddy
</div>
<div style="padding:16px;border:1px solid #ddd;border-radius:0 0 8px 8px">
  {escaped}
</div>
<p style="color:#999;font-size:11px">Bonsai Buddy · local AI assistant</p>
</body></html>"#
    );
    FormattedMessage::single(html)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn split_at(text: &str, limit: usize) -> Vec<String> {
    if text.len() <= limit {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut remaining = text;
    while !remaining.is_empty() {
        if remaining.len() <= limit {
            chunks.push(remaining.to_string());
            break;
        }
        // Try to split on paragraph boundary
        let slice = &remaining[..limit];
        let split_at = slice.rfind("\n\n")
            .or_else(|| slice.rfind('\n'))
            .unwrap_or(limit);
        chunks.push(remaining[..split_at].to_string());
        remaining = remaining[split_at..].trim_start();
    }
    chunks
}
