use crate::parser::SkillMetadata;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Rule {
    pub condition: String,
    pub action: String,
    /// Heuristic confidence 0.0–1.0
    pub confidence: f32,
}

/// Extract rules from the markdown body using lightweight heuristics.
/// No LLM call here — keeps compilation fully offline and deterministic.
pub fn extract_rules(_metadata: &SkillMetadata, body: &str) -> Result<Vec<Rule>> {
    let mut rules = Vec::new();

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Bullet list items → unconditional rules
        if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            rules.push(Rule {
                condition: "always".into(),
                action: rest.to_string(),
                confidence: 0.8,
            });
        // Numbered list items
        } else if let Some(idx) = line.find(". ") {
            let prefix = &line[..idx];
            if prefix.chars().all(|c| c.is_ascii_digit()) {
                rules.push(Rule {
                    condition: "always".into(),
                    action: line[idx + 2..].to_string(),
                    confidence: 0.75,
                });
            }
        // Explicit if/then patterns
        } else {
            let lower = line.to_lowercase();
            if lower.contains("if ") && lower.contains(" then ") {
                if let Some((cond, act)) = line.split_once(" then ") {
                    rules.push(Rule {
                        condition: cond.to_string(),
                        action: act.to_string(),
                        confidence: 0.9,
                    });
                }
            } else if lower.starts_with("always ") || lower.starts_with("never ") {
                rules.push(Rule {
                    condition: "always".into(),
                    action: line.to_string(),
                    confidence: 0.85,
                });
            }
        }
    }

    Ok(rules)
}

/// Infer required permissions from rule text.
pub fn extract_permissions(_metadata: &SkillMetadata, rules: &[Rule]) -> Vec<String> {
    let combined: String = rules
        .iter()
        .flat_map(|r| [r.condition.as_str(), r.action.as_str()])
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();

    let mut perms = Vec::new();
    if combined.contains("read file") || combined.contains("read_file") {
        perms.push("read_fs".into());
    }
    if combined.contains("write file") || combined.contains("write_file") {
        perms.push("write_fs".into());
    }
    if combined.contains("run command") || combined.contains("run_command") || combined.contains("shell") {
        perms.push("run_shell".into());
    }
    if combined.contains("network") || combined.contains("http") || combined.contains("url") {
        perms.push("network".into());
    }
    if combined.contains("database") || combined.contains("sql") {
        perms.push("database".into());
    }
    perms
}
