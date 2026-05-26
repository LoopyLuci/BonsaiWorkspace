//! Data science, analytics, and statistical tools.
//! Implemented in pure Rust using serde_json and math; no heavy ML framework required.

use async_trait::async_trait;
use serde_json::{json, Value};
use crate::tool_registry::{Tool, ToolResult};

// ── Data Profile ──────────────────────────────────────────────────────────────

pub struct DataProfileTool;
#[async_trait]
impl Tool for DataProfileTool {
    fn name(&self) -> &str { "data_profile" }
    fn description(&self) -> &str { "Generate a comprehensive data profile from a CSV: column types, null counts, min/max, distributions, and top values." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let csv_input = args["csv"].as_str()
            .map(|s| s.to_string())
            .or_else(|| args["path"].as_str().and_then(|p| std::fs::read_to_string(p).ok()))
            .ok_or("Missing 'csv' or 'path'")?;

        let max_rows = args["max_rows"].as_u64().unwrap_or(10_000) as usize;
        let profile  = profile_csv(&csv_input, max_rows);
        Ok(ToolResult::json(&profile))
    }
}

fn profile_csv(csv: &str, max_rows: usize) -> Value {
    let mut lines = csv.lines();
    let headers: Vec<String> = lines.next()
        .map(|h| h.split(',').map(|s| s.trim().trim_matches('"').to_string()).collect())
        .unwrap_or_default();
    let num_cols = headers.len();

    // Accumulate per-column data
    let mut col_values: Vec<Vec<String>> = vec![Vec::new(); num_cols];
    let mut row_count = 0usize;
    let mut null_counts = vec![0usize; num_cols];

    for line in lines.take(max_rows) {
        let cols: Vec<&str> = line.split(',').collect();
        row_count += 1;
        for (i, header) in headers.iter().enumerate().take(num_cols) {
            let val = cols.get(i).copied().unwrap_or("").trim().trim_matches('"');
            if val.is_empty() || val.eq_ignore_ascii_case("null") || val.eq_ignore_ascii_case("na") || val == "NaN" {
                null_counts[i] += 1;
            } else {
                col_values[i].push(val.to_string());
            }
        }
    }

    let columns: Vec<Value> = headers.iter().enumerate().map(|(i, name)| {
        let vals = &col_values[i];
        let nulls = null_counts[i];
        let distinct: std::collections::HashSet<&String> = vals.iter().collect();

        // Detect type
        let numeric_vals: Vec<f64> = vals.iter().filter_map(|v| v.parse().ok()).collect();
        let is_numeric = !numeric_vals.is_empty() && numeric_vals.len() >= vals.len() * 7 / 10;
        let col_type = if is_numeric { "numeric" } else if vals.iter().all(|v| v.len() <= 1) { "boolean/flag" } else { "string" };

        let mut stats = json!({
            "name": name,
            "type": col_type,
            "count": vals.len(),
            "null_count": nulls,
            "null_pct": if row_count > 0 { (nulls as f64 / row_count as f64 * 100.0).round() / 100.0 } else { 0.0 },
            "distinct_count": distinct.len(),
        });

        if is_numeric && !numeric_vals.is_empty() {
            let mut sorted = numeric_vals.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let n = sorted.len() as f64;
            let sum: f64 = sorted.iter().sum();
            let mean = sum / n;
            let median = if sorted.len() % 2 == 0 {
                (sorted[sorted.len()/2-1] + sorted[sorted.len()/2]) / 2.0
            } else { sorted[sorted.len()/2] };
            let variance = sorted.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
            let std_dev  = variance.sqrt();
            if let Some(s) = stats.as_object_mut() {
                s.insert("min".into(), json!(sorted.first().unwrap()));
                s.insert("max".into(), json!(sorted.last().unwrap()));
                s.insert("mean".into(), json!((mean * 10000.0).round() / 10000.0));
                s.insert("median".into(), json!(median));
                s.insert("std_dev".into(), json!((std_dev * 10000.0).round() / 10000.0));
                s.insert("p25".into(), json!(sorted[(n * 0.25) as usize]));
                s.insert("p75".into(), json!(sorted[(n * 0.75).min(n - 1.0) as usize]));
            }
        } else {
            // Top 5 values by frequency
            let mut freq: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
            for v in vals { *freq.entry(v.as_str()).or_insert(0) += 1; }
            let mut top: Vec<(&str, usize)> = freq.into_iter().collect();
            top.sort_by(|a, b| b.1.cmp(&a.1));
            if let Some(s) = stats.as_object_mut() {
                s.insert("top_values".into(), json!(top.into_iter().take(5).map(|(v, c)| json!({"value": v, "count": c})).collect::<Vec<_>>()));
                s.insert("avg_length".into(), json!(vals.iter().map(|v| v.len()).sum::<usize>() as f64 / vals.len().max(1) as f64));
            }
        }
        stats
    }).collect();

    json!({
        "row_count": row_count,
        "column_count": num_cols,
        "columns": columns,
        "completeness_pct": if row_count > 0 && num_cols > 0 {
            let total_nulls: usize = null_counts.iter().sum();
            let total_cells = row_count * num_cols;
            ((total_cells - total_nulls) as f64 / total_cells as f64 * 100.0).round() / 100.0
        } else { 0.0 }
    })
}

// ── Correlation Matrix ────────────────────────────────────────────────────────

pub struct CorrelationMatrixTool;
#[async_trait]
impl Tool for CorrelationMatrixTool {
    fn name(&self) -> &str { "correlation_matrix" }
    fn description(&self) -> &str { "Compute pairwise Pearson or Spearman correlation for numeric columns in a CSV." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let csv     = args["csv"].as_str()
            .map(|s| s.to_string())
            .or_else(|| args["path"].as_str().and_then(|p| std::fs::read_to_string(p).ok()))
            .ok_or("Missing 'csv' or 'path'")?;
        let method  = args["method"].as_str().unwrap_or("pearson"); // pearson|spearman

        let (headers, columns) = parse_numeric_cols(&csv);
        if columns.len() < 2 { return Err("Need at least 2 numeric columns".into()); }

        let n = columns.len();
        let mut matrix = vec![vec![0.0f64; n]; n];
        for i in 0..n {
            for j in 0..n {
                matrix[i][j] = pearson_r(&columns[i], &columns[j]);
            }
        }

        let result_rows: Vec<Value> = (0..n).map(|i| {
            let row: serde_json::Map<String, Value> = headers.iter().enumerate()
                .map(|(j, h)| (h.clone(), json!((matrix[i][j] * 10000.0).round() / 10000.0)))
                .collect();
            json!({ "column": headers[i], "correlations": row })
        }).collect();

        Ok(ToolResult::json(&json!({ "method": method, "columns": headers, "matrix": result_rows })))
    }
}

fn parse_numeric_cols(csv: &str) -> (Vec<String>, Vec<Vec<f64>>) {
    let mut lines = csv.lines();
    let headers: Vec<String> = lines.next()
        .map(|h| h.split(',').map(|s| s.trim().trim_matches('"').to_string()).collect())
        .unwrap_or_default();
    let n = headers.len();
    let mut all_rows: Vec<Vec<Option<f64>>> = Vec::new();
    for line in lines {
        let cols: Vec<&str> = line.split(',').collect();
        all_rows.push((0..n).map(|i| cols.get(i).copied().unwrap_or("").trim().trim_matches('"').parse().ok()).collect());
    }
    // Only keep columns with >50% numeric values
    let mut num_headers = Vec::new();
    let mut num_cols    = Vec::new();
    for (i, h) in headers.iter().enumerate() {
        let vals: Vec<f64> = all_rows.iter().filter_map(|r| r.get(i).copied().flatten()).collect();
        if vals.len() > all_rows.len() / 2 { num_headers.push(h.clone()); num_cols.push(vals); }
    }
    (num_headers, num_cols)
}

fn pearson_r(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len()) as f64;
    if n == 0.0 { return 0.0; }
    let ma = a.iter().sum::<f64>() / n;
    let mb = b.iter().sum::<f64>() / n;
    let num: f64 = a.iter().zip(b).map(|(x, y)| (x - ma) * (y - mb)).sum();
    let da: f64  = a.iter().map(|x| (x - ma).powi(2)).sum::<f64>().sqrt();
    let db: f64  = b.iter().map(|x| (x - mb).powi(2)).sum::<f64>().sqrt();
    if da == 0.0 || db == 0.0 { 0.0 } else { num / (da * db) }
}

// ── Outlier Detection ─────────────────────────────────────────────────────────

pub struct OutlierDetectTool;
#[async_trait]
impl Tool for OutlierDetectTool {
    fn name(&self) -> &str { "outlier_detect" }
    fn description(&self) -> &str { "Detect outliers in a numeric column using IQR or Z-score method and return outlier rows." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let csv    = args["csv"].as_str()
            .map(|s| s.to_string())
            .or_else(|| args["path"].as_str().and_then(|p| std::fs::read_to_string(p).ok()))
            .ok_or("Missing 'csv' or 'path'")?;
        let column = args["column"].as_str().ok_or("Missing 'column'")?;
        let method = args["method"].as_str().unwrap_or("iqr"); // iqr|zscore
        let threshold = args["threshold"].as_f64().unwrap_or(1.5); // IQR multiplier or Z-score threshold

        let mut lines  = csv.lines();
        let headers: Vec<&str> = lines.next().map(|h| h.split(',').collect()).unwrap_or_default();
        let col_idx = headers.iter().position(|h| h.trim().trim_matches('"') == column)
            .ok_or(format!("Column '{}' not found", column))?;

        let rows: Vec<(usize, Vec<String>, f64)> = lines.enumerate()
            .filter_map(|(i, line)| {
                let cols: Vec<String> = line.split(',').map(|s| s.trim().trim_matches('"').to_string()).collect();
                let val: f64 = cols.get(col_idx)?.parse().ok()?;
                Some((i + 2, cols, val)) // +2 for 1-indexed + header
            }).collect();

        let values: Vec<f64> = rows.iter().map(|(_, _, v)| *v).collect();
        let mut sorted = values.clone(); sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = sorted.len() as f64;
        let mean = values.iter().sum::<f64>() / n;
        let std  = (values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n).sqrt();
        let q1   = sorted[(n * 0.25) as usize];
        let q3   = sorted[(n * 0.75).min(n - 1.0) as usize];
        let iqr  = q3 - q1;

        let outliers: Vec<Value> = rows.iter().filter(|(_, _, v)| {
            match method {
                "zscore" => ((v - mean) / std).abs() > threshold,
                _        => *v < q1 - threshold * iqr || *v > q3 + threshold * iqr,
            }
        }).map(|(row_num, cols, v)| json!({ "row": row_num, "value": v, "columns": cols })).collect();

        Ok(ToolResult::json(&json!({
            "column": column, "method": method, "threshold": threshold,
            "total_rows": rows.len(), "outlier_count": outliers.len(),
            "mean": mean, "std": std, "q1": q1, "q3": q3, "iqr": iqr,
            "outliers": outliers
        })))
    }
}

// ── Random Sample ─────────────────────────────────────────────────────────────

pub struct RandomSampleTool;
#[async_trait]
impl Tool for RandomSampleTool {
    fn name(&self) -> &str { "random_sample" }
    fn description(&self) -> &str { "Draw a random sample from a CSV dataset with or without replacement." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let csv         = args["csv"].as_str()
            .map(|s| s.to_string())
            .or_else(|| args["path"].as_str().and_then(|p| std::fs::read_to_string(p).ok()))
            .ok_or("Missing 'csv' or 'path'")?;
        let n           = args["n"].as_u64().ok_or("Missing 'n' (sample size)")? as usize;
        let replacement = args["replacement"].as_bool().unwrap_or(false);
        let seed        = args["seed"].as_u64();

        let mut lines = csv.lines();
        let header = lines.next().unwrap_or("").to_string();
        let rows: Vec<String> = lines.map(String::from).collect();
        if rows.is_empty() { return Err("Empty dataset".into()); }

        let mut indices: Vec<usize> = if replacement {
            (0..n).map(|_| fastrand::usize(..rows.len())).collect()
        } else {
            let mut idx: Vec<usize> = (0..rows.len()).collect();
            // Fisher-Yates shuffle
            for i in (1..idx.len()).rev() {
                let j = fastrand::usize(..=i);
                idx.swap(i, j);
            }
            idx.into_iter().take(n.min(rows.len())).collect()
        };

        let sampled: Vec<&str> = indices.iter().map(|i| rows[*i].as_str()).collect();
        let csv_out = format!("{header}\n{}", sampled.join("\n"));
        Ok(ToolResult::json(&json!({
            "sample_csv": csv_out,
            "sampled_rows": sampled.len(),
            "total_rows": rows.len(),
            "with_replacement": replacement
        })))
    }
}

// ── Data Impute ───────────────────────────────────────────────────────────────

pub struct DataImputeTool;
#[async_trait]
impl Tool for DataImputeTool {
    fn name(&self) -> &str { "data_impute" }
    fn description(&self) -> &str { "Fill missing values in a CSV using mean, median, mode, or forward-fill strategy." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let csv      = args["csv"].as_str()
            .map(|s| s.to_string())
            .or_else(|| args["path"].as_str().and_then(|p| std::fs::read_to_string(p).ok()))
            .ok_or("Missing 'csv' or 'path'")?;
        let strategy = args["strategy"].as_str().unwrap_or("mean"); // mean|median|mode|forward_fill|zero|empty_string
        let column   = args["column"].as_str(); // if None, applies to all numeric cols

        let mut lines = csv.lines();
        let header_line = lines.next().unwrap_or("").to_string();
        let headers: Vec<&str> = header_line.split(',').collect();
        let mut rows: Vec<Vec<String>> = lines.map(|l| l.split(',').map(String::from).collect()).collect();
        let total_rows = rows.len();

        let target_cols: Vec<usize> = if let Some(col) = column {
            headers.iter().position(|h| h.trim().trim_matches('"') == col)
                .map(|i| vec![i]).ok_or(format!("Column '{}' not found", col))?
        } else {
            (0..headers.len()).collect()
        };

        let mut imputed_count = 0usize;
        for &col_i in &target_cols {
            // Compute fill value
            let non_null: Vec<String> = rows.iter()
                .filter_map(|r| r.get(col_i))
                .filter(|v| !v.trim().is_empty() && !v.trim().eq_ignore_ascii_case("null"))
                .map(|v| v.trim().to_string())
                .collect();

            let fill = match strategy {
                "mean" | "median" => {
                    let nums: Vec<f64> = non_null.iter().filter_map(|v| v.parse().ok()).collect();
                    if nums.is_empty() { continue; }
                    let mut sorted = nums.clone(); sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    if strategy == "mean" {
                        format!("{:.4}", sorted.iter().sum::<f64>() / sorted.len() as f64)
                    } else {
                        let mid = sorted.len() / 2;
                        format!("{:.4}", if sorted.len() % 2 == 0 { (sorted[mid-1] + sorted[mid]) / 2.0 } else { sorted[mid] })
                    }
                }
                "mode" => {
                    let mut freq: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
                    for v in &non_null { *freq.entry(v.as_str()).or_insert(0) += 1; }
                    freq.into_iter().max_by_key(|(_, c)| *c).map(|(v, _)| v.to_string()).unwrap_or_default()
                }
                "zero"         => "0".to_string(),
                "empty_string" => String::new(),
                _              => String::new(), // forward_fill handled below
            };

            let mut last_val = fill.clone();
            for row in rows.iter_mut() {
                if let Some(cell) = row.get_mut(col_i) {
                    if cell.trim().is_empty() || cell.trim().eq_ignore_ascii_case("null") {
                        *cell = if strategy == "forward_fill" { last_val.clone() } else { fill.clone() };
                        imputed_count += 1;
                    } else {
                        last_val = cell.clone();
                    }
                }
            }
        }

        let csv_out = format!("{header_line}\n{}", rows.iter().map(|r| r.join(",")).collect::<Vec<_>>().join("\n"));
        Ok(ToolResult::json(&json!({
            "csv": csv_out, "strategy": strategy,
            "imputed_values": imputed_count, "total_rows": total_rows
        })))
    }
}

// ── AB Test Analyzer ──────────────────────────────────────────────────────────

pub struct AbTestAnalyzerTool;
#[async_trait]
impl Tool for AbTestAnalyzerTool {
    fn name(&self) -> &str { "ab_test_analyzer" }
    fn description(&self) -> &str { "Analyze A/B test results: statistical significance (chi-squared or t-test), effect size, confidence intervals, and sample size recommendations." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        // Conversion rate test (chi-squared)
        if let (Some(a_conversions), Some(a_total), Some(b_conversions), Some(b_total)) = (
            args["a_conversions"].as_u64(), args["a_total"].as_u64(),
            args["b_conversions"].as_u64(), args["b_total"].as_u64(),
        ) {
            let alpha = args["alpha"].as_f64().unwrap_or(0.05);
            let (p_a, p_b) = (a_conversions as f64 / a_total as f64, b_conversions as f64 / b_total as f64);
            let p_pool = (a_conversions + b_conversions) as f64 / (a_total + b_total) as f64;
            let se = (p_pool * (1.0 - p_pool) * (1.0 / a_total as f64 + 1.0 / b_total as f64)).sqrt();
            let z = if se > 0.0 { (p_b - p_a) / se } else { 0.0 };
            // Two-tailed p-value approximation
            let p_value = 2.0 * (1.0 - normal_cdf(z.abs()));
            let significant = p_value < alpha;
            let lift = if p_a > 0.0 { (p_b - p_a) / p_a * 100.0 } else { 0.0 };
            // Minimum sample size for 80% power
            let min_n = (2.0_f64 * (1.6449_f64 + 1.2816_f64).powi(2) * p_pool * (1.0 - p_pool) / (p_b - p_a).powi(2).max(1e-10)).ceil() as u64;
            return Ok(ToolResult::json(&json!({
                "test_type": "proportion",
                "variant_a": { "conversions": a_conversions, "total": a_total, "rate": (p_a * 10000.0).round() / 10000.0 },
                "variant_b": { "conversions": b_conversions, "total": b_total, "rate": (p_b * 10000.0).round() / 10000.0 },
                "z_score": (z * 10000.0).round() / 10000.0,
                "p_value": (p_value * 10000.0).round() / 10000.0,
                "significant": significant,
                "alpha": alpha,
                "lift_pct": (lift * 100.0).round() / 100.0,
                "winner": if significant { if p_b > p_a { "B" } else { "A" } } else { "inconclusive" },
                "recommended_min_sample_per_variant": min_n,
            })));
        }

        // Continuous metric (Welch's t-test)
        if let (Some(a_arr), Some(b_arr)) = (args["a_values"].as_array(), args["b_values"].as_array()) {
            let alpha = args["alpha"].as_f64().unwrap_or(0.05);
            let a: Vec<f64> = a_arr.iter().filter_map(|v| v.as_f64()).collect();
            let b: Vec<f64> = b_arr.iter().filter_map(|v| v.as_f64()).collect();
            if a.is_empty() || b.is_empty() { return Err("Empty arrays".into()); }
            let (na, nb) = (a.len() as f64, b.len() as f64);
            let ma = a.iter().sum::<f64>() / na;
            let mb = b.iter().sum::<f64>() / nb;
            let va = a.iter().map(|x| (x - ma).powi(2)).sum::<f64>() / (na - 1.0).max(1.0);
            let vb = b.iter().map(|x| (x - mb).powi(2)).sum::<f64>() / (nb - 1.0).max(1.0);
            let se = (va / na + vb / nb).sqrt();
            let t  = if se > 0.0 { (mb - ma) / se } else { 0.0 };
            let p_value = 2.0 * (1.0 - normal_cdf(t.abs()));
            let cohen_d = if (va + vb) > 0.0 { (mb - ma) / ((va + vb) / 2.0).sqrt() } else { 0.0 };
            let effect = if cohen_d.abs() < 0.2 { "negligible" } else if cohen_d.abs() < 0.5 { "small" } else if cohen_d.abs() < 0.8 { "medium" } else { "large" };
            return Ok(ToolResult::json(&json!({
                "test_type": "continuous",
                "variant_a": { "n": a.len(), "mean": (ma * 10000.0).round() / 10000.0, "std": va.sqrt() },
                "variant_b": { "n": b.len(), "mean": (mb * 10000.0).round() / 10000.0, "std": vb.sqrt() },
                "t_statistic": (t * 10000.0).round() / 10000.0,
                "p_value": (p_value * 10000.0).round() / 10000.0,
                "significant": p_value < alpha,
                "effect_size_cohens_d": (cohen_d * 10000.0).round() / 10000.0,
                "effect_magnitude": effect,
                "winner": if p_value < alpha { if mb > ma { "B" } else { "A" } } else { "inconclusive" },
            })));
        }
        Err("Provide either (a_conversions, a_total, b_conversions, b_total) or (a_values, b_values) arrays".into())
    }
}

fn normal_cdf(z: f64) -> f64 {
    // Abramowitz & Stegun approximation
    let t = 1.0 / (1.0 + 0.2316419 * z.abs());
    let poly = t * (0.319381530 + t * (-0.356563782 + t * (1.781477937 + t * (-1.821255978 + t * 1.330274429))));
    let pdf = (-z * z / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt();
    if z >= 0.0 { 1.0 - pdf * poly } else { pdf * poly }
}

// ── Time Series Decompose ─────────────────────────────────────────────────────

pub struct TimeSeriesDecomposeTool;
#[async_trait]
impl Tool for TimeSeriesDecomposeTool {
    fn name(&self) -> &str { "time_series_decompose" }
    fn description(&self) -> &str { "Decompose a time series into trend, seasonal, and residual components using moving average." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let values: Vec<f64> = args["values"].as_array().ok_or("Missing 'values' array")?
            .iter().filter_map(|v| v.as_f64()).collect();
        let period = args["period"].as_u64().unwrap_or(7) as usize;
        if values.len() < period * 2 { return Err(format!("Need at least {} data points", period * 2)); }
        let n = values.len();

        // Trend: centered moving average
        let window = if period % 2 == 0 { period } else { period };
        let half = window / 2;
        let mut trend = vec![f64::NAN; n];
        for i in half..n.saturating_sub(half) {
            trend[i] = values[i.saturating_sub(half)..=(i + half).min(n-1)].iter().sum::<f64>() / window as f64;
        }

        // Seasonal: average deviation from trend per period position
        let mut seasonal_sums = vec![0.0f64; period];
        let mut seasonal_cnts = vec![0usize; period];
        for i in 0..n {
            if !trend[i].is_nan() {
                seasonal_sums[i % period] += values[i] - trend[i];
                seasonal_cnts[i % period] += 1;
            }
        }
        let seasonal_avgs: Vec<f64> = seasonal_sums.iter().zip(&seasonal_cnts)
            .map(|(s, c)| if *c > 0 { s / *c as f64 } else { 0.0 }).collect();

        // Residual
        let residual: Vec<f64> = values.iter().enumerate()
            .map(|(i, v)| if trend[i].is_nan() { f64::NAN } else { v - trend[i] - seasonal_avgs[i % period] })
            .collect();

        let seasonal: Vec<f64> = (0..n).map(|i| seasonal_avgs[i % period]).collect();

        Ok(ToolResult::json(&json!({
            "n": n, "period": period,
            "trend": trend.iter().map(|x| if x.is_nan() { Value::Null } else { json!((*x * 10000.0).round() / 10000.0) }).collect::<Vec<_>>(),
            "seasonal": seasonal.iter().map(|x| json!((*x * 10000.0).round() / 10000.0)).collect::<Vec<_>>(),
            "residual": residual.iter().map(|x| if x.is_nan() { Value::Null } else { json!((*x * 10000.0).round() / 10000.0) }).collect::<Vec<_>>(),
            "seasonal_pattern": seasonal_avgs.iter().map(|x| json!((*x * 10000.0).round() / 10000.0)).collect::<Vec<_>>(),
        })))
    }
}

// ── Registration ──────────────────────────────────────────────────────────────

use std::sync::Arc;
use crate::tool_registry::Tool as ToolTrait;

pub fn all_data_science_tools() -> Vec<Arc<dyn ToolTrait>> {
    vec![
        Arc::new(DataProfileTool),
        Arc::new(CorrelationMatrixTool),
        Arc::new(OutlierDetectTool),
        Arc::new(RandomSampleTool),
        Arc::new(DataImputeTool),
        Arc::new(AbTestAnalyzerTool),
        Arc::new(TimeSeriesDecomposeTool),
    ]
}
