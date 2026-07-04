//! Server-side rich block rendering: Mermaid graphs, bar/line/pie charts, math passthrough.
//! Returns inline SVG strings consumed by the frontend RichMarkdown component.

use std::fmt::Write as FmtWrite;

// ── Public API ────────────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct ChartDataPoint {
    pub label: String,
    pub value: f64,
}

/// Render a rich content block to an SVG (or passthrough HTML for math).
#[tauri::command]
pub fn render_rich_block(
    block_type: String,
    content: String,
    data: Option<Vec<ChartDataPoint>>,
) -> Result<String, String> {
    match block_type.as_str() {
        "mermaid" => Ok(render_mermaid(&content)),
        "bar_chart" => Ok(render_bar_chart(data.as_deref().unwrap_or(&[]))),
        "line_chart" => Ok(render_line_chart(data.as_deref().unwrap_or(&[]))),
        "pie_chart" => Ok(render_pie_chart(data.as_deref().unwrap_or(&[]))),
        "math" => Ok(render_math(&content)),
        _ => Err(format!("Unknown block type: {block_type}")),
    }
}

// ── Mermaid ───────────────────────────────────────────────────────────────────

fn render_mermaid(source: &str) -> String {
    let mut nodes: Vec<String> = Vec::new();
    let mut edges: Vec<(String, String, Option<String>)> = Vec::new();

    for raw in source.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("graph") || line.starts_with("flowchart") {
            continue;
        }
        if let Some((lhs, rhs)) = line.split_once("-->") {
            let src = clean_node(lhs);
            let (label, dst) = if rhs.trim_start().starts_with('|') {
                let inner = rhs.trim_start().trim_start_matches('|');
                if let Some((lbl, rest)) = inner.split_once('|') {
                    (Some(lbl.trim().to_string()), clean_node(rest))
                } else {
                    (None, clean_node(rhs))
                }
            } else {
                (None, clean_node(rhs))
            };
            if !src.is_empty() && !dst.is_empty() {
                if !nodes.contains(&src) {
                    nodes.push(src.clone());
                }
                if !nodes.contains(&dst) {
                    nodes.push(dst.clone());
                }
                edges.push((src, dst, label));
            }
        } else {
            let n = clean_node(line);
            if !n.is_empty() && !nodes.contains(&n) {
                nodes.push(n);
            }
        }
    }

    let cols = ((nodes.len() as f64).sqrt().ceil() as usize).max(1);
    let node_w = 120u32;
    let node_h = 40u32;
    let gap_x = 60u32;
    let gap_y = 60u32;
    let pad = 20u32;
    let rows = (nodes.len() + cols - 1) / cols;
    let svg_w = pad * 2 + cols as u32 * node_w + (cols as u32).saturating_sub(1) * gap_x;
    let svg_h = pad * 2 + rows as u32 * node_h + (rows as u32).saturating_sub(1) * gap_y;

    let mut cx_map: std::collections::HashMap<&str, (u32, u32)> = Default::default();
    for (i, node) in nodes.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let cx = pad + col as u32 * (node_w + gap_x) + node_w / 2;
        let cy = pad + row as u32 * (node_h + gap_y) + node_h / 2;
        cx_map.insert(node.as_str(), (cx, cy));
    }

    let style = concat!(
        "<style>",
        ".mn{fill:#1e293b;stroke:#38bdf8;stroke-width:1.5}",
        ".mt{fill:#e2e8f0;font:12px sans-serif;text-anchor:middle;dominant-baseline:central}",
        ".me{stroke:#64748b;stroke-width:1.5;fill:none}",
        ".ml{fill:#94a3b8;font:10px sans-serif;text-anchor:middle}",
        "</style>"
    );
    let defs = concat!(
        "<defs><marker id=\"arr\" markerWidth=\"8\" markerHeight=\"8\" refX=\"6\" refY=\"3\" orient=\"auto\">",
        "<path d=\"M0,0 L0,6 L9,3 z\" fill=\"#64748b\"/>",
        "</marker></defs>"
    );

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{svg_w}\" height=\"{svg_h}\" viewBox=\"0 0 {svg_w} {svg_h}\">"
    );
    svg.push_str(style);
    svg.push_str(defs);

    for (src, dst, label) in &edges {
        if let (Some(&(x1, y1)), Some(&(x2, y2))) =
            (cx_map.get(src.as_str()), cx_map.get(dst.as_str()))
        {
            let mx = (x1 + x2) / 2;
            let my = (y1 + y2) / 2;
            let _ = write!(svg, "<line class=\"me\" x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" marker-end=\"url(#arr)\"/>");
            if let Some(lbl) = label {
                let _ = write!(
                    svg,
                    "<text class=\"ml\" x=\"{mx}\" y=\"{my}\">{}</text>",
                    escape_xml(lbl)
                );
            }
        }
    }

    for (node, &(cx, cy)) in &cx_map {
        let x = cx - node_w / 2;
        let y = cy - node_h / 2;
        let _ = write!(
            svg,
            "<rect class=\"mn\" x=\"{x}\" y=\"{y}\" width=\"{node_w}\" height=\"{node_h}\" rx=\"6\"/>",
        );
        let _ = write!(
            svg,
            "<text class=\"mt\" x=\"{cx}\" y=\"{cy}\">{}</text>",
            escape_xml(node)
        );
    }

    svg.push_str("</svg>");
    svg
}

fn clean_node(s: &str) -> String {
    s.trim()
        .trim_matches(|c: char| matches!(c, '[' | ']' | '(' | ')' | '{' | '}' | '"'))
        .trim()
        .to_string()
}

// ── Bar Chart ─────────────────────────────────────────────────────────────────

fn render_bar_chart(data: &[ChartDataPoint]) -> String {
    if data.is_empty() {
        return empty_svg("No data");
    }
    let max_val = data
        .iter()
        .map(|d| d.value)
        .fold(f64::MIN, f64::max)
        .max(1.0);
    let bar_w = 40u32;
    let gap = 20u32;
    let chart_h = 200u32;
    let pad_l = 40u32;
    let pad_b = 40u32;
    let pad_t = 20u32;
    let svg_w = pad_l + (bar_w + gap) * data.len() as u32 + gap;
    let svg_h = chart_h + pad_t + pad_b;

    let mut svg =
        format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{svg_w}\" height=\"{svg_h}\">");
    svg.push_str("<style>text{font:11px sans-serif;fill:#e2e8f0}</style>");

    for (i, pt) in data.iter().enumerate() {
        let bar_h = ((pt.value / max_val) * chart_h as f64) as u32;
        let x = pad_l + i as u32 * (bar_w + gap) + gap;
        let y = pad_t + chart_h - bar_h;
        let lx = x + bar_w / 2;
        let _ = write!(svg, "<rect x=\"{x}\" y=\"{y}\" width=\"{bar_w}\" height=\"{bar_h}\" fill=\"#38bdf8\" rx=\"3\"/>");
        let _ = write!(
            svg,
            "<text x=\"{lx}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
            pad_t + chart_h + 15,
            escape_xml(&pt.label)
        );
        let _ = write!(
            svg,
            "<text x=\"{lx}\" y=\"{}\" text-anchor=\"middle\">{:.1}</text>",
            y.saturating_sub(4),
            pt.value
        );
    }

    svg.push_str("</svg>");
    svg
}

// ── Line Chart ────────────────────────────────────────────────────────────────

fn render_line_chart(data: &[ChartDataPoint]) -> String {
    if data.is_empty() {
        return empty_svg("No data");
    }
    let max_val = data
        .iter()
        .map(|d| d.value)
        .fold(f64::MIN, f64::max)
        .max(1.0);
    let chart_w = 400u32;
    let chart_h = 200u32;
    let pad_l = 40u32;
    let pad_t = 20u32;
    let pad_b = 40u32;
    let svg_w = chart_w + pad_l + 20;
    let svg_h = chart_h + pad_t + pad_b;
    let n = data.len().max(1);

    let points: Vec<(u32, u32)> = data
        .iter()
        .enumerate()
        .map(|(i, pt)| {
            let x = pad_l + (i as u32 * chart_w) / n as u32;
            let y = pad_t + chart_h - ((pt.value / max_val) * chart_h as f64) as u32;
            (x, y)
        })
        .collect();

    let path = points
        .iter()
        .enumerate()
        .map(|(i, (x, y))| {
            if i == 0 {
                format!("M{x},{y}")
            } else {
                format!("L{x},{y}")
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let mut svg =
        format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{svg_w}\" height=\"{svg_h}\">");
    svg.push_str("<style>text{font:11px sans-serif;fill:#e2e8f0}</style>");
    let _ = write!(
        svg,
        "<path d=\"{path}\" stroke=\"#38bdf8\" stroke-width=\"2\" fill=\"none\"/>"
    );

    for ((x, y), pt) in points.iter().zip(data.iter()) {
        let _ = write!(
            svg,
            "<circle cx=\"{x}\" cy=\"{y}\" r=\"4\" fill=\"#38bdf8\"/>"
        );
        let _ = write!(
            svg,
            "<text x=\"{x}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
            pad_t + chart_h + 15,
            escape_xml(&pt.label)
        );
    }

    svg.push_str("</svg>");
    svg
}

// ── Pie Chart ─────────────────────────────────────────────────────────────────

fn render_pie_chart(data: &[ChartDataPoint]) -> String {
    if data.is_empty() {
        return empty_svg("No data");
    }
    let total: f64 = data.iter().map(|d| d.value).sum();
    if total == 0.0 {
        return empty_svg("All values zero");
    }

    let cx = 120.0f64;
    let cy = 120.0f64;
    let r = 100.0f64;
    let colors = [
        "#38bdf8", "#818cf8", "#34d399", "#fb923c", "#f472b6", "#a3e635",
    ];

    let mut svg =
        String::from("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"300\" height=\"260\">");
    svg.push_str("<style>text{font:11px sans-serif;fill:#e2e8f0}</style>");

    let mut angle = -std::f64::consts::FRAC_PI_2;
    for (i, pt) in data.iter().enumerate() {
        let sweep = (pt.value / total) * 2.0 * std::f64::consts::PI;
        let x1 = cx + r * angle.cos();
        let y1 = cy + r * angle.sin();
        angle += sweep;
        let x2 = cx + r * angle.cos();
        let y2 = cy + r * angle.sin();
        let large = if sweep > std::f64::consts::PI { 1 } else { 0 };
        let color = colors[i % colors.len()];
        let _ = write!(svg, "<path d=\"M{cx},{cy} L{x1:.2},{y1:.2} A{r},{r} 0 {large},1 {x2:.2},{y2:.2} Z\" fill=\"{color}\"/>");
        let ly = 20 + i as u32 * 18;
        let _ = write!(
            svg,
            "<rect x=\"245\" y=\"{ly}\" width=\"12\" height=\"12\" fill=\"{color}\"/>"
        );
        let _ = write!(
            svg,
            "<text x=\"261\" y=\"{}\">{} ({:.1}%)</text>",
            ly + 10,
            escape_xml(&pt.label),
            (pt.value / total) * 100.0
        );
    }

    svg.push_str("</svg>");
    svg
}

// ── Math (passthrough) ────────────────────────────────────────────────────────

fn render_math(source: &str) -> String {
    format!(
        "<span class=\"katex-block\" data-expr=\"{}\">{}</span>",
        escape_attr(source),
        escape_xml(source)
    )
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn empty_svg(msg: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"200\" height=\"40\"><text x=\"10\" y=\"24\" font=\"12px sans-serif\" fill=\"#94a3b8\">{}</text></svg>",
        escape_xml(msg)
    )
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\n', " ")
}
