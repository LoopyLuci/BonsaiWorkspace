//! Extended creative tools: SVG generation, emoji lookup, color palette, ASCII art,
//! audio waveform visualization, and GIF creation.

use async_trait::async_trait;
use serde_json::{json, Value};
use crate::tool_registry::{Tool, ToolResult};

// ── Text to SVG ───────────────────────────────────────────────────────────────

pub struct TextToSvgTool;
#[async_trait]
impl Tool for TextToSvgTool {
    fn name(&self) -> &str { "text_to_svg" }
    fn description(&self) -> &str { "Render text as an SVG with custom font size, color, background, and style." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let text       = args["text"].as_str().ok_or("Missing 'text'")?;
        let font_size  = args["font_size"].as_u64().unwrap_or(48) as u32;
        let color      = args["color"].as_str().unwrap_or("#333333");
        let bg_color   = args["background"].as_str().unwrap_or("#ffffff");
        let font_family = args["font_family"].as_str().unwrap_or("monospace");
        let bold       = args["bold"].as_bool().unwrap_or(false);
        let italic     = args["italic"].as_bool().unwrap_or(false);
        let padding    = 20u32;
        let line_height = (font_size as f64 * 1.4) as u32;
        let lines: Vec<&str> = text.lines().collect();
        let width  = (lines.iter().map(|l| l.len()).max().unwrap_or(10) as u32 * font_size / 2).max(200) + padding * 2;
        let height = line_height * lines.len() as u32 + padding * 2;
        let weight = if bold { "bold" } else { "normal" };
        let style  = if italic { "italic" } else { "normal" };

        let text_els: String = lines.iter().enumerate().map(|(i, line)| {
            let y = padding + (i as u32 + 1) * line_height - font_size / 4;
            let escaped = line.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
            format!(r#"  <text x="{padding}" y="{y}" font-family="{font_family}" font-size="{font_size}" font-weight="{weight}" font-style="{style}" fill="{color}">{escaped}</text>"#)
        }).collect::<Vec<_>>().join("\n");

        let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">
  <rect width="{width}" height="{height}" fill="{bg_color}"/>
{text_els}
</svg>"#);

        Ok(ToolResult::json(&json!({ "svg": svg, "width": width, "height": height })))
    }
}

// ── Emoji Explain ─────────────────────────────────────────────────────────────

pub struct EmojiExplainTool;
#[async_trait]
impl Tool for EmojiExplainTool {
    fn name(&self) -> &str { "emoji_explain" }
    fn description(&self) -> &str { "Look up an emoji's Unicode name, category, and common meanings/uses." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let emoji = args["emoji"].as_str().ok_or("Missing 'emoji'")?;
        let first_char = emoji.chars().next().ok_or("Empty string")?;
        let codepoint  = first_char as u32;

        // Categorize by Unicode block
        let (category, description) = categorize_emoji(codepoint, emoji);

        Ok(ToolResult::json(&json!({
            "emoji": emoji,
            "codepoint": format!("U+{:04X}", codepoint),
            "category": category,
            "description": description,
            "hex": format!("{:X}", codepoint),
            "decimal": codepoint,
        })))
    }
}

fn categorize_emoji(cp: u32, emoji: &str) -> (&'static str, String) {
    let lower = emoji;
    // Common emoji patterns by codepoint range
    let desc = match cp {
        0x1F600..=0x1F64F => ("Smileys & Emotion", emoji_name_hint(cp)),
        0x1F300..=0x1F5FF => ("Symbols & Pictographs", emoji_name_hint(cp)),
        0x1F680..=0x1F6FF => ("Transport & Map", emoji_name_hint(cp)),
        0x1F900..=0x1F9FF => ("Supplemental Symbols", emoji_name_hint(cp)),
        0x2600..=0x26FF   => ("Miscellaneous Symbols", emoji_name_hint(cp)),
        0x2700..=0x27BF   => ("Dingbats", emoji_name_hint(cp)),
        0x1F1E0..=0x1F1FF => ("Regional Indicator", "Country flag letter component".to_string()),
        0x0041..=0x007A   => ("Latin", format!("Letter '{}'", emoji)),
        0x0030..=0x0039   => ("Digit", format!("Number digit '{}'", emoji)),
        _ => ("Other Unicode", format!("Unicode character U+{:04X}", cp)),
    };
    desc
}

fn emoji_name_hint(cp: u32) -> String {
    // Minimal lookup for common emoji
    match cp {
        0x1F600 => "😀 Grinning face — joy, happiness",
        0x1F601 => "😁 Beaming face — glee, excitement",
        0x1F602 => "😂 Face with tears — laughing, LOL",
        0x1F60D => "😍 Heart eyes — love, admiration",
        0x1F614 => "😔 Pensive face — sad, thoughtful",
        0x1F621 => "😡 Angry face — anger, frustration",
        0x1F44D => "👍 Thumbs up — approval, like",
        0x1F44E => "👎 Thumbs down — disapproval, dislike",
        0x1F525 => "🔥 Fire — hot, trending, lit",
        0x2764  => "❤️ Red heart — love, care",
        0x1F389 => "🎉 Party popper — celebration",
        0x1F914 => "🤔 Thinking face — pondering, curious",
        0x1F440 => "👀 Eyes — watching, looking",
        0x1F4AF => "💯 100 — perfect score, totally",
        0x1F680 => "🚀 Rocket — launch, fast, startup",
        _       => "Unicode emoji symbol",
    }.to_string()
}

// ── Color Palette ─────────────────────────────────────────────────────────────

pub struct ColorPaletteTool;
#[async_trait]
impl Tool for ColorPaletteTool {
    fn name(&self) -> &str { "color_palette" }
    fn description(&self) -> &str { "Extract a dominant color palette (up to 8 colors) from an image file." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let path    = args["path"].as_str().ok_or("Missing 'path'")?;
        let k       = args["colors"].as_u64().unwrap_or(5).min(16) as usize;
        let bytes   = tokio::fs::read(path).await.map_err(|e| e.to_string())?;
        let img     = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
        let resized = img.resize(64, 64, image::imageops::FilterType::Nearest).to_rgb8();

        // Collect all pixel colors (sampling)
        let pixels: Vec<[u8; 3]> = resized.pixels().map(|p| [p[0], p[1], p[2]]).collect();

        // K-means clustering in RGB space
        let palette = kmeans_colors(&pixels, k);
        let colors: Vec<Value> = palette.iter().map(|[r, g, b]| {
            let hex = format!("#{r:02X}{g:02X}{b:02X}");
            let (h, s, l) = rgb_to_hsl(*r, *g, *b);
            json!({ "hex": hex, "rgb": { "r": r, "g": g, "b": b }, "hsl": { "h": h, "s": s, "l": l } })
        }).collect();

        Ok(ToolResult::json(&json!({ "palette": colors, "count": colors.len() })))
    }
}

fn kmeans_colors(pixels: &[[u8; 3]], k: usize) -> Vec<[u8; 3]> {
    if pixels.is_empty() || k == 0 { return vec![]; }
    let k = k.min(pixels.len());
    // Initialize with evenly-spaced pixels
    let step = pixels.len() / k;
    let mut centroids: Vec<[f64; 3]> = (0..k).map(|i| {
        let p = pixels[i * step];
        [p[0] as f64, p[1] as f64, p[2] as f64]
    }).collect();

    for _ in 0..20 {
        let mut sums   = vec![[0.0f64; 3]; k];
        let mut counts = vec![0usize; k];
        for &p in pixels {
            let (pf, ci) = ([p[0] as f64, p[1] as f64, p[2] as f64], nearest_centroid(&p, &centroids));
            for c in 0..3 { sums[ci][c] += pf[c]; }
            counts[ci] += 1;
        }
        for i in 0..k {
            if counts[i] > 0 {
                for c in 0..3 { centroids[i][c] = sums[i][c] / counts[i] as f64; }
            }
        }
    }
    centroids.iter().map(|c| [c[0] as u8, c[1] as u8, c[2] as u8]).collect()
}

fn nearest_centroid(p: &[u8; 3], centroids: &[[f64; 3]]) -> usize {
    centroids.iter().enumerate().min_by_key(|(_, c)| {
        let d: f64 = (0..3).map(|i| (p[i] as f64 - c[i]).powi(2)).sum();
        (d * 1000.0) as u64
    }).map(|(i, _)| i).unwrap_or(0)
}

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let (rf, gf, bf) = (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
    let max = rf.max(gf).max(bf); let min = rf.min(gf).min(bf);
    let l = (max + min) / 2.0;
    let s = if max == min { 0.0 } else { (max - min) / (1.0 - (2.0 * l - 1.0).abs()) };
    let h = if max == min { 0.0 }
        else if max == rf { 60.0 * (((gf - bf) / (max - min)) % 6.0) }
        else if max == gf { 60.0 * ((bf - rf) / (max - min) + 2.0) }
        else              { 60.0 * ((rf - gf) / (max - min) + 4.0) };
    ((h + 360.0) % 360.0, (s * 100.0).round(), (l * 100.0).round())
}

// ── Image to ASCII ────────────────────────────────────────────────────────────

pub struct ImageToAsciiTool;
#[async_trait]
impl Tool for ImageToAsciiTool {
    fn name(&self) -> &str { "image_to_ascii" }
    fn description(&self) -> &str { "Convert an image to ASCII art with adjustable width and character density." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let path  = args["path"].as_str().ok_or("Missing 'path'")?;
        let width = args["width"].as_u64().unwrap_or(80) as u32;
        let dense = args["dense"].as_bool().unwrap_or(false);

        let bytes = tokio::fs::read(path).await.map_err(|e| e.to_string())?;
        let img   = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
        let aspect = img.height() as f64 / img.width() as f64;
        let height = (width as f64 * aspect * 0.55) as u32;
        let resized = img.resize_exact(width, height.max(1), image::imageops::FilterType::Nearest).to_luma8();

        let chars: &[char] = if dense {
            &[' ', '.', ':', '-', '=', '+', '*', '#', '%', '@']
        } else {
            &[' ', '.', ':', '|', '(', 'S', '%', '@']
        };

        let mut ascii = String::new();
        for row in resized.rows() {
            for pix in row {
                let idx = (pix[0] as f64 / 255.0 * (chars.len() - 1) as f64).round() as usize;
                ascii.push(chars[idx]);
            }
            ascii.push('\n');
        }
        Ok(ToolResult::json(&json!({ "ascii": ascii, "width": width, "height": height })))
    }
}

// ── Generate Color Palette from Description ───────────────────────────────────

pub struct GeneratePaletteTool;
#[async_trait]
impl Tool for GeneratePaletteTool {
    fn name(&self) -> &str { "generate_palette" }
    fn description(&self) -> &str { "Generate a harmonious color palette from a mood, style, or description using color theory." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let description = args["description"].as_str().ok_or("Missing 'description'")?;
        let count       = args["count"].as_u64().unwrap_or(5).min(10) as usize;

        let lower = description.to_lowercase();
        // Map common moods to base hue ranges
        let base_hue: f64 = if lower.contains("warm") || lower.contains("sunset") || lower.contains("fire") { 20.0 }
            else if lower.contains("cool") || lower.contains("ocean") || lower.contains("sky") || lower.contains("blue") { 210.0 }
            else if lower.contains("nature") || lower.contains("forest") || lower.contains("green") { 120.0 }
            else if lower.contains("purple") || lower.contains("royal") || lower.contains("magic") { 270.0 }
            else if lower.contains("pink") || lower.contains("rose") || lower.contains("romantic") { 340.0 }
            else if lower.contains("earth") || lower.contains("brown") || lower.contains("wood") { 30.0 }
            else if lower.contains("monochrome") || lower.contains("grey") || lower.contains("gray") { 0.0 }
            else { (description.len() as f64 * 37.0) % 360.0 };

        let saturation: f64 = if lower.contains("muted") || lower.contains("pastel") { 40.0 }
            else if lower.contains("vibrant") || lower.contains("vivid") || lower.contains("bold") { 85.0 }
            else { 65.0 };
        let lightness: f64 = if lower.contains("dark") || lower.contains("deep") { 30.0 }
            else if lower.contains("light") || lower.contains("soft") || lower.contains("pale") { 70.0 }
            else { 50.0 };

        let scheme = args["scheme"].as_str().unwrap_or("analogous"); // analogous|complementary|triadic|split_complementary
        let hues: Vec<f64> = match scheme {
            "complementary"       => vec![base_hue, (base_hue + 180.0) % 360.0],
            "triadic"             => vec![base_hue, (base_hue + 120.0) % 360.0, (base_hue + 240.0) % 360.0],
            "split_complementary" => vec![base_hue, (base_hue + 150.0) % 360.0, (base_hue + 210.0) % 360.0],
            _                     => (0..count).map(|i| (base_hue + i as f64 * 30.0) % 360.0).collect(),
        };

        let colors: Vec<Value> = hues.iter().cycle().take(count).enumerate().map(|(i, &h)| {
            let l_adj = lightness + (i as f64 - count as f64 / 2.0) * 8.0;
            let (r, g, b) = hsl_to_rgb(h, saturation / 100.0, (l_adj.clamp(20.0, 85.0)) / 100.0);
            let hex = format!("#{r:02X}{g:02X}{b:02X}");
            json!({ "hex": hex, "hsl": { "h": h.round(), "s": saturation.round(), "l": l_adj.clamp(20.0, 85.0).round() }, "rgb": { "r": r, "g": g, "b": b } })
        }).collect();

        Ok(ToolResult::json(&json!({ "palette": colors, "scheme": scheme, "description": description })))
    }
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r1, g1, b1) = match h as u32 {
        0..=59   => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179=> (0.0, c, x),
        180..=239=> (0.0, x, c),
        240..=299=> (x, 0.0, c),
        _        => (c, 0.0, x),
    };
    (((r1 + m) * 255.0).round() as u8, ((g1 + m) * 255.0).round() as u8, ((b1 + m) * 255.0).round() as u8)
}

// ── GIF Create ────────────────────────────────────────────────────────────────

pub struct GifCreateTool;
#[async_trait]
impl Tool for GifCreateTool {
    fn name(&self) -> &str { "gif_create" }
    fn description(&self) -> &str { "Combine multiple PNG/JPEG images into an animated GIF with configurable frame delay." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let paths  = args["paths"].as_array().ok_or("Missing 'paths' array")?;
        let output = args["output"].as_str().ok_or("Missing 'output' path")?;
        let delay  = args["delay_ms"].as_u64().unwrap_or(200).min(5000) as u16;
        let size   = args["size"].as_u64().unwrap_or(256) as u32;

        let gif_delay = delay / 10; // GIF delay is in centiseconds
        let mut encoder: Option<gif::Encoder<std::fs::File>> = None;

        for path_val in paths {
            let path = path_val.as_str().ok_or("Invalid path in array")?;
            let bytes = tokio::fs::read(path).await.map_err(|e| e.to_string())?;
            let img   = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
            let frame_img = img.resize(size, size, image::imageops::FilterType::Nearest).to_rgba8();
            let (w, h)    = frame_img.dimensions();

            // Quantize to 256 colors (simple uniform palette)
            let pixels: Vec<u8> = frame_img.pixels().flat_map(|p| [p[0], p[1], p[2]]).collect();

            if encoder.is_none() {
                let file = std::fs::File::create(output).map_err(|e| e.to_string())?;
                let mut enc = gif::Encoder::new(file, w as u16, h as u16, &[]).map_err(|e| e.to_string())?;
                enc.set_repeat(gif::Repeat::Infinite).map_err(|e| e.to_string())?;
                encoder = Some(enc);
            }

            if let Some(ref mut enc) = encoder {
                let mut frame = gif::Frame::from_rgb(w as u16, h as u16, &pixels);
                frame.delay = gif_delay;
                enc.write_frame(&frame).map_err(|e| e.to_string())?;
            }
        }

        Ok(ToolResult::json(&json!({
            "output": output,
            "frames": paths.len(),
            "delay_ms": delay,
            "size": size,
        })))
    }
}

// ── Audio Waveform Visualize ──────────────────────────────────────────────────

pub struct AudioVisualizeTool;
#[async_trait]
impl Tool for AudioVisualizeTool {
    fn name(&self) -> &str { "audio_visualize" }
    fn description(&self) -> &str { "Generate a waveform PNG image from a WAV audio file." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let input  = args["input"].as_str().ok_or("Missing 'input' (WAV path)")?;
        let output = args["output"].as_str().ok_or("Missing 'output' (PNG path)")?;
        let width  = args["width"].as_u64().unwrap_or(1024) as u32;
        let height = args["height"].as_u64().unwrap_or(256) as u32;
        let color  = args["color"].as_str().unwrap_or("#00aaff");

        // Read WAV using hound
        let reader = hound::WavReader::open(input).map_err(|e| e.to_string())?;
        let spec   = reader.spec();
        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float =>
                reader.into_samples::<f32>().filter_map(|s| s.ok()).collect(),
            hound::SampleFormat::Int =>
                reader.into_samples::<i32>().filter_map(|s| s.ok())
                    .map(|s| s as f32 / i32::MAX as f32).collect(),
        };

        let fg_rgb = parse_hex_color(color);
        let mut img = image::RgbaImage::new(width, height);
        // Fill background
        for pix in img.pixels_mut() { *pix = image::Rgba([20, 20, 30, 255]); }

        let mid_y  = height / 2;
        let n = samples.len();
        for x in 0..width {
            let start = (x as usize * n / width as usize).min(n.saturating_sub(1));
            let end   = ((x as usize + 1) * n / width as usize).min(n);
            let chunk = if start < end { &samples[start..end] } else { &samples[start..=start.min(n-1)] };
            let peak  = chunk.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
            let amp   = (peak * mid_y as f32) as u32;
            for dy in 0..amp.min(mid_y) {
                let alpha = ((1.0 - dy as f32 / mid_y as f32) * 200.0) as u8 + 55;
                img.put_pixel(x, mid_y - dy, image::Rgba([fg_rgb[0], fg_rgb[1], fg_rgb[2], alpha]));
                img.put_pixel(x, mid_y + dy, image::Rgba([fg_rgb[0], fg_rgb[1], fg_rgb[2], alpha]));
            }
        }

        img.save(output).map_err(|e| e.to_string())?;
        Ok(ToolResult::json(&json!({
            "output": output, "width": width, "height": height,
            "sample_rate": spec.sample_rate,
            "channels": spec.channels,
            "duration_secs": n as f64 / spec.sample_rate as f64 / spec.channels as f64,
        })))
    }
}

fn parse_hex_color(hex: &str) -> [u8; 3] {
    let s = hex.trim_start_matches('#');
    if s.len() >= 6 {
        let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(170);
        let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(255);
        [r, g, b]
    } else { [0, 170, 255] }
}

// ── Generate SVG Logo ─────────────────────────────────────────────────────────

pub struct GenerateLogoTool;
#[async_trait]
impl Tool for GenerateLogoTool {
    fn name(&self) -> &str { "generate_logo" }
    fn description(&self) -> &str { "Generate a simple SVG logo with initials, icon shape, and color from a text description." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let name    = args["name"].as_str().ok_or("Missing 'name'")?;
        let style   = args["style"].as_str().unwrap_or("circle"); // circle|square|hexagon|badge
        let primary = args["primary_color"].as_str().unwrap_or("#4A90E2");
        let secondary = args["secondary_color"].as_str().unwrap_or("#ffffff");
        let size    = args["size"].as_u64().unwrap_or(200) as u32;

        // Extract initials
        let initials: String = name.split_whitespace()
            .filter_map(|w| w.chars().next())
            .take(2)
            .map(|c| c.to_uppercase().next().unwrap_or(c))
            .collect();
        let font_size = size / 3;
        let cx = size / 2;
        let cy = size / 2;
        let text_y = cy + font_size / 3;

        let shape = match style {
            "square"   => format!(r#"<rect x="0" y="0" width="{size}" height="{size}" fill="{primary}" rx="{}" />"#, size / 10),
            "hexagon"  => {
                let r = size / 2;
                let pts: String = (0..6).map(|i| {
                    let a = std::f64::consts::PI / 3.0 * i as f64 - std::f64::consts::PI / 6.0;
                    format!("{},{}", cx as f64 + r as f64 * a.cos(), cy as f64 + r as f64 * a.sin())
                }).collect::<Vec<_>>().join(" ");
                format!(r#"<polygon points="{pts}" fill="{primary}" />"#)
            }
            "badge"    => format!(r#"<rect x="0" y="0" width="{size}" height="{size}" fill="{primary}" rx="{}" />
  <rect x="{}" y="{}" width="{}" height="{}" fill="{secondary}" opacity="0.15" />"#,
                size / 20, size / 10, size * 3 / 10, size * 4 / 5, size / 5),
            _          => format!(r#"<circle cx="{cx}" cy="{cy}" r="{}" fill="{primary}" />"#, size / 2),
        };

        let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 {size} {size}">
  {shape}
  <text x="{cx}" y="{text_y}" text-anchor="middle" font-family="Arial, sans-serif" font-size="{font_size}" font-weight="bold" fill="{secondary}">{initials}</text>
</svg>"#);

        Ok(ToolResult::json(&json!({ "svg": svg, "initials": initials, "style": style, "size": size })))
    }
}

// ── Registration ──────────────────────────────────────────────────────────────

use std::sync::Arc;
use crate::tool_registry::Tool as ToolTrait;

pub fn all_creative_ext_tools() -> Vec<Arc<dyn ToolTrait>> {
    vec![
        Arc::new(TextToSvgTool),
        Arc::new(EmojiExplainTool),
        Arc::new(ColorPaletteTool),
        Arc::new(ImageToAsciiTool),
        Arc::new(GeneratePaletteTool),
        Arc::new(GifCreateTool),
        Arc::new(AudioVisualizeTool),
        Arc::new(GenerateLogoTool),
    ]
}
