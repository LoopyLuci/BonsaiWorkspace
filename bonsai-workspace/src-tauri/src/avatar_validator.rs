/// SVG sanitizer + 14-viseme rig completeness validator.
/// Strips <script>, external refs, and on* event attrs.
/// Verifies all data-viseme="0"..="13" paths exist.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AvatarRigReport {
    pub valid:           bool,
    pub missing_visemes: Vec<u8>,
    pub warnings:        Vec<String>,
}

/// Strip dangerous constructs from raw SVG and return cleaned text.
pub fn sanitize_svg(raw: &str) -> Result<String, String> {
    // Quick sanity check
    if !raw.contains("<svg") {
        return Err("Input does not look like an SVG file".into());
    }
    if raw.len() > 512 * 1024 {
        return Err("SVG exceeds 512 KB limit".into());
    }

    let mut out = String::with_capacity(raw.len());
    let mut i = 0;
    let bytes = raw.as_bytes();

    while i < bytes.len() {
        // Detect opening tag
        if bytes[i] == b'<' {
            // Find end of tag name
            let tag_start = i + 1;
            let tag_end = bytes[tag_start..]
                .iter()
                .position(|&b| b == b' ' || b == b'>' || b == b'/')
                .map(|p| tag_start + p)
                .unwrap_or(bytes.len());
            let tag_name = &raw[tag_start..tag_end].to_lowercase();
            let tag_name = tag_name.trim_start_matches('/');

            // Strip <script> … </script> entirely
            if tag_name == "script" {
                let close = raw[i..].find("</script>")
                    .map(|p| i + p + 9)
                    .unwrap_or(bytes.len());
                i = close;
                continue;
            }

            // Find close of this tag (> or />)
            let tag_close = raw[i..].find('>').map(|p| i + p + 1).unwrap_or(bytes.len());
            let tag_text = &raw[i..tag_close];

            // Strip external href / xlink:href pointing outside the SVG
            let cleaned = strip_external_refs(tag_text);

            // Strip on* event attributes
            let cleaned = strip_event_attrs(&cleaned);

            out.push_str(&cleaned);
            i = tag_close;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }

    Ok(out)
}

fn strip_external_refs(tag: &str) -> String {
    // Replace href="http..." or xlink:href="http..." with href="#"
    let re_patterns = [
        ("href=\"http", "href=\"#"),
        ("href='http",  "href='#"),
        ("xlink:href=\"http", "xlink:href=\"#"),
        ("xlink:href='http",  "xlink:href='#"),
        // data: URIs with JS
        ("href=\"data:", "href=\"#"),
        ("href='data:",  "href='#"),
    ];
    let mut s = tag.to_string();
    for (pat, rep) in &re_patterns {
        while let Some(pos) = s.find(pat) {
            // find end of attribute value
            let after = pos + pat.len();
            let end = s[after..].find(|c| c == '"' || c == '\'')
                .map(|p| after + p)
                .unwrap_or(s.len());
            s.replace_range(pos..end, rep);
        }
    }
    s
}

fn strip_event_attrs(tag: &str) -> String {
    // Remove on* attributes: onclick=, onload=, etc.
    let mut s = tag.to_string();
    loop {
        // Find " on" or space on
        if let Some(pos) = find_event_attr(&s) {
            // Find end of attribute (next unquoted space or >)
            let end = find_attr_end(&s, pos);
            s.replace_range(pos..end, "");
        } else {
            break;
        }
    }
    s
}

fn find_event_attr(s: &str) -> Option<usize> {
    let lower = s.to_lowercase();
    // Look for " on" or "\ton" followed by letters and "="
    let mut i = 0;
    let bytes = lower.as_bytes();
    while i < bytes.len().saturating_sub(3) {
        if (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n')
            && bytes[i + 1] == b'o'
            && bytes[i + 2] == b'n'
        {
            // Check that next chars are [a-z]+ then '='
            let mut j = i + 3;
            while j < bytes.len() && bytes[j].is_ascii_alphabetic() { j += 1; }
            if j < bytes.len() && bytes[j] == b'=' { return Some(i); }
        }
        i += 1;
    }
    None
}

fn find_attr_end(s: &str, start: usize) -> usize {
    let bytes = s.as_bytes();
    let mut i = start + 1;
    // Skip past = and value
    while i < bytes.len() && bytes[i] != b'=' { i += 1; }
    if i >= bytes.len() { return i; }
    i += 1; // skip '='
    if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
        let quote = bytes[i];
        i += 1;
        while i < bytes.len() && bytes[i] != quote { i += 1; }
        i += 1; // closing quote
    } else {
        while i < bytes.len() && bytes[i] != b' ' && bytes[i] != b'>' { i += 1; }
    }
    i
}

/// Validate that the SVG contains all 14 required data-viseme paths.
pub fn validate_rig(svg: &str) -> Result<AvatarRigReport, String> {
    let mut missing: Vec<u8> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    for id in 0u8..=13 {
        let needle = format!("data-viseme=\"{id}\"");
        let needle2 = format!("data-viseme='{id}'");
        if !svg.contains(&needle) && !svg.contains(&needle2) {
            missing.push(id);
        }
    }

    if !svg.contains("<svg") {
        return Err("No <svg> root element found".into());
    }

    // Warn about suspicious patterns that slipped through
    if svg.to_lowercase().contains("javascript:") {
        warnings.push("Suspicious javascript: URI found after sanitization".into());
    }

    Ok(AvatarRigReport {
        valid: missing.is_empty() && warnings.is_empty(),
        missing_visemes: missing,
        warnings,
    })
}
