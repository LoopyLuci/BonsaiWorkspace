/// Test Runner - Execute tests in various languages
use std::process::Output;
use std::time::Duration;

/// Execute a test in a specific language with given input
pub async fn run_test(
    lang: &str,
    source: &str,
    input: &str,
    seed: Option<u64>,
    runner_template: Option<&str>,
    timeout: Duration,
) -> anyhow::Result<Output> {
    // Default runner commands for common languages
    let default_cmd = default_runner_for(lang)?;
    let template = runner_template.unwrap_or(default_cmd);

    // Substitute placeholders
    let cmd_str = template
        .replace("{src}", source)
        .replace("{input}", input)
        .replace("{seed}", &seed.map(|s| s.to_string()).unwrap_or_default());

    tracing::debug!("Running: {}", cmd_str);

    let mut parts = cmd_str.split_whitespace().collect::<Vec<_>>();
    if parts.is_empty() {
        anyhow::bail!("Empty command");
    }

    let program = parts.remove(0);
    let output = tokio::time::timeout(
        timeout,
        tokio::process::Command::new(program)
            .args(parts)
            .output(),
    )
    .await
    .map_err(|_| anyhow::anyhow!("Test execution timeout"))?
    .map_err(|e| anyhow::anyhow!("Failed to execute: {}", e))?;

    Ok(output)
}

/// Get the default runner command for a language
fn default_runner_for(lang: &str) -> anyhow::Result<&'static str> {
    match lang {
        "python" | "py" | "python3" => Ok("python3 {src} {input}"),
        "rust" | "rs" => Ok("cargo run --quiet --manifest-path {src} {input}"),
        "javascript" | "js" | "node" => Ok("node {src} {input}"),
        "go" | "golang" => Ok("go run {src} {input}"),
        "java" => Ok("java -cp {src} Main {input}"),
        "cpp" | "c++" => Ok("g++ -O2 {src} -o /tmp/a.out && /tmp/a.out {input}"),
        "c" => Ok("gcc -O2 {src} -o /tmp/a.out && /tmp/a.out {input}"),
        "csharp" | "cs" => Ok("csc {src} && ./main.exe {input}"),
        "ruby" | "rb" => Ok("ruby {src} {input}"),
        "php" => Ok("php {src} {input}"),
        "perl" => Ok("perl {src} {input}"),
        "lua" => Ok("lua {src} {input}"),
        _ => anyhow::bail!("Unknown language: {}", lang),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_runner_python() {
        let cmd = default_runner_for("python").unwrap();
        assert!(cmd.contains("python3"));
    }

    #[test]
    fn test_default_runner_rust() {
        let cmd = default_runner_for("rust").unwrap();
        assert!(cmd.contains("cargo"));
    }

    #[test]
    fn test_unknown_language() {
        assert!(default_runner_for("unknown123").is_err());
    }
}
