/// Output Comparison and Fidelity Scoring
use serde_json::Value;

/// Compare two outputs and compute a fidelity score (0.0..=1.0)
pub fn compare_outputs(actual: &str, expected: &str) -> f64 {
    let actual_trimmed = actual.trim();
    let expected_trimmed = expected.trim();

    // Try JSON comparison first
    if let (Ok(actual_json), Ok(expected_json)) =
        (serde_json::from_str::<Value>(actual_trimmed),
         serde_json::from_str::<Value>(expected_trimmed))
    {
        return compare_json(&actual_json, &expected_json);
    }

    // Fall back to string comparison
    if actual_trimmed == expected_trimmed {
        1.0
    } else {
        // Compute string similarity (Levenshtein-like, simplified)
        string_similarity(actual_trimmed, expected_trimmed)
    }
}

/// Compare two JSON values recursively
fn compare_json(actual: &Value, expected: &Value) -> f64 {
    match (actual, expected) {
        (Value::Null, Value::Null) => 1.0,
        (Value::Bool(a), Value::Bool(e)) => if a == e { 1.0 } else { 0.0 },
        (Value::Number(a), Value::Number(e)) => {
            // Allow small floating-point differences
            if let (Some(av), Some(ev)) = (a.as_f64(), e.as_f64()) {
                let diff = (av - ev).abs();
                let max = av.abs().max(ev.abs());
                if max < 1e-9 {
                    1.0
                } else {
                    1.0 - (diff / max).min(1.0)
                }
            } else if a == e {
                1.0
            } else {
                0.0
            }
        }
        (Value::String(a), Value::String(e)) => {
            if a == e { 1.0 } else { string_similarity(a, e) }
        }
        (Value::Array(a), Value::Array(e)) => {
            if a.len() != e.len() {
                return 0.0;
            }
            let scores: Vec<f64> = a
                .iter()
                .zip(e.iter())
                .map(|(av, ev)| compare_json(av, ev))
                .collect();
            if scores.is_empty() {
                1.0
            } else {
                scores.iter().sum::<f64>() / scores.len() as f64
            }
        }
        (Value::Object(a), Value::Object(e)) => {
            if a.len() != e.len() {
                return 0.0;
            }
            let mut score_sum = 0.0;
            for (key, a_val) in a.iter() {
                if let Some(e_val) = e.get(key) {
                    score_sum += compare_json(a_val, e_val);
                } else {
                    return 0.0;
                }
            }
            if a.is_empty() {
                1.0
            } else {
                score_sum / a.len() as f64
            }
        }
        _ => 0.0,
    }
}

/// Simple string similarity metric (Levenshtein-inspired, capped for performance)
fn string_similarity(a: &str, b: &str) -> f64 {
    let a_len = a.len();
    let b_len = b.len();

    if a_len == 0 && b_len == 0 {
        return 1.0;
    }

    if a_len == 0 || b_len == 0 {
        return 0.0;
    }

    // Quick check: if strings are very different in length, similarity is low
    let len_ratio = a_len.min(b_len) as f64 / a_len.max(b_len) as f64;
    if len_ratio < 0.5 {
        return 0.0;
    }

    // Simple character-match ratio
    let mut matches = 0;
    for (ac, bc) in a.chars().zip(b.chars()) {
        if ac == bc {
            matches += 1;
        }
    }

    matches as f64 / a_len.max(b_len) as f64
}

/// Struct to hold comparison result with detailed metrics
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    pub fidelity: f64,
    pub passed: bool,
    pub threshold: f64,
}

impl ComparisonResult {
    pub fn new(actual: &str, expected: &str, threshold: f64) -> Self {
        let fidelity = compare_outputs(actual, expected);
        let passed = fidelity >= threshold;
        ComparisonResult {
            fidelity,
            passed,
            threshold,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_string_match() {
        assert_eq!(compare_outputs("hello", "hello"), 1.0);
    }

    #[test]
    fn test_whitespace_ignored() {
        assert_eq!(compare_outputs("  hello  ", "hello"), 1.0);
    }

    #[test]
    fn test_json_numbers() {
        assert_eq!(
            compare_outputs(r#"{"value": 42}"#, r#"{"value": 42}"#),
            1.0
        );
    }

    #[test]
    fn test_json_float_tolerance() {
        let fidelity = compare_outputs(r#"{"x": 1.0}"#, r#"{"x": 1.0000001}"#);
        assert!(fidelity > 0.99);
    }

    #[test]
    fn test_string_mismatch() {
        let fidelity = compare_outputs("abc", "xyz");
        assert!(fidelity < 0.5);
    }

    #[test]
    fn test_comparison_result() {
        let result = ComparisonResult::new("output", "output", 0.99);
        assert!(result.passed);
        assert_eq!(result.fidelity, 1.0);
    }
}
