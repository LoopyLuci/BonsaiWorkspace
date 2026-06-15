use reqwest::Client;
use serde_json::Value;
use anyhow::Result;
use lazy_static::lazy_static;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::tool_registry::McpToolRegistry;

lazy_static! {
    static ref HTTP_CLIENT: Client = Client::new();
    static ref DAEMON_URL: String = std::env::var("BONSAI_DAEMON_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8080/api".to_string());
    static ref TOOL_REGISTRY: McpToolRegistry = McpToolRegistry::new();
}

pub async fn call_bonsai(token: &str, tool_name: &str, args: Value) -> Result<Value> {
    if tool_name.starts_with("bonsai_") {
        if TOOL_REGISTRY.get_tool(tool_name).is_some() {
            return TOOL_REGISTRY.execute_tool(tool_name, args)
                .await
                .map_err(|e| anyhow::anyhow!(e));
        }
        return run_devkit_tool(tool_name, args);
    }

    let endpoint = match tool_name {
        "read_file" => "file/read",
        "write_file" => "file/write",
        "chat" => "chat",
        "run_cargo_check" => "tools/run",
        "run_cargo_test" => "tools/run",
        "pull_model" => "models/pull",
        "list_models" => "models/list",
        "submit_issue" => "issues/create",
        "suggest_fix" => "survival/suggest_fix",
        _ => tool_name,
    };
    let url = format!("{}/{}", *DAEMON_URL, endpoint);
    let response = HTTP_CLIENT
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .json(&args)
        .send()
        .await?
        .error_for_status()?;
    Ok(response.json().await?)
}

fn run_devkit_tool(tool_name: &str, args: Value) -> Result<Value> {
    let workspace = find_workspace_root(std::env::current_dir()?.as_path())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let mut cli_args: Vec<String> = vec!["--json".to_string()];
    match tool_name {
        "bonsai_setup" => {
            cli_args.push("setup".to_string());
        }
        "bonsai_build" => {
            cli_args.push("build".to_string());
            if let Some(target) = args.get("target").and_then(|v| v.as_str()) {
                cli_args.push("--target".to_string());
                cli_args.push(target.to_string());
            }
            if args.get("release").and_then(|v| v.as_bool()).unwrap_or(false) {
                cli_args.push("--release".to_string());
            }
            if let Some(crates) = args.get("crates") {
                if let Some(arr) = crates.as_array() {
                    let list = arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(",");
                    if !list.is_empty() {
                        cli_args.push("--crates".to_string());
                        cli_args.push(list);
                    }
                }
            }
        }
        "bonsai_test" => {
            cli_args.push("test".to_string());
            match args.get("suite").and_then(|v| v.as_str()).unwrap_or("workspace") {
                "unit" => cli_args.push("--unit".to_string()),
                "integration" => cli_args.push("--integration".to_string()),
                "performance" => cli_args.push("--performance".to_string()),
                _ => cli_args.push("--workspace".to_string()),
            }
        }
        "bonsai_run" => {
            cli_args.push("run".to_string());
            let service = args
                .get("service")
                .and_then(|v| v.as_str())
                .or_else(|| args.get("component").and_then(|v| v.as_str()))
                .unwrap_or("desktop");
            cli_args.push(service.to_string());
            if args.get("detach").and_then(|v| v.as_bool()).unwrap_or(false) {
                cli_args.push("--detach".to_string());
            }
            if let Some(port) = args.get("port").and_then(|v| v.as_u64()) {
                cli_args.push("--port".to_string());
                cli_args.push(port.to_string());
            }
        }
        "bonsai_logs" => {
            cli_args.push("logs".to_string());
            let service = args
                .get("service")
                .and_then(|v| v.as_str())
                .unwrap_or("mcp-server");
            cli_args.push(service.to_string());
            if args.get("follow").and_then(|v| v.as_bool()).unwrap_or(false) {
                cli_args.push("--follow".to_string());
            }
        }
        "bonsai_stop" => {
            cli_args.push("stop".to_string());
            let service = args
                .get("service")
                .and_then(|v| v.as_str())
                .unwrap_or("mcp-server");
            cli_args.push(service.to_string());
        }
        "bonsai_list_detached" => {
            cli_args.push("list-detached".to_string());
        }
        "bonsai_clean" => {
            cli_args.push("clean".to_string());
            if args.get("cache").and_then(|v| v.as_bool()).unwrap_or(false) {
                cli_args.push("--cache".to_string());
            }
        }
        "bonsai_deploy" => {
            cli_args.push("deploy".to_string());
            let target = args
                .get("target")
                .and_then(|v| v.as_str())
                .unwrap_or("windows");
            cli_args.push(target.to_string());
        }
        "bonsai_docs" => {
            cli_args.push("docs".to_string());
            if args.get("serve").and_then(|v| v.as_bool()).unwrap_or(false) {
                cli_args.push("--serve".to_string());
                if let Some(port) = args.get("port").and_then(|v| v.as_u64()) {
                    cli_args.push("--port".to_string());
                    cli_args.push(port.to_string());
                }
            }
        }
        "bonsai_status" => {
            cli_args.push("status".to_string());
        }
        // Linting tools (handled by async handlers, not CLI)
        "bonsai_lint_file" | "bonsai_lint_repo" | "bonsai_generate_lint_rule"
        | "bonsai_explain_diagnostic" | "bonsai_report_false_positive"
        | "bonsai_dismiss_diagnostic" | "bonsai_apply_fix" => {
            return Ok(serde_json::json!({
                "error": "Use async handler for linting tools",
                "tool": tool_name
            }));
        }
        _ => {
            return Ok(serde_json::json!({
                "ok": false,
                "tool": tool_name,
                "error": "Unsupported DevKit tool"
            }));
        }
    }

    let output = run_bonsai_cli(&workspace, &cli_args)?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        if let Ok(parsed) = serde_json::from_str::<Value>(&stdout) {
            Ok(serde_json::json!({
                "success": true,
                "result": parsed,
                "stderr": stderr,
            }))
        } else {
            Ok(serde_json::json!({
                "success": true,
                "result": stdout,
                "stderr": stderr,
            }))
        }
    } else {
        Ok(serde_json::json!({
            "success": false,
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": output.status.code(),
        }))
    }
}

fn run_bonsai_cli(workspace: &Path, cli_args: &[String]) -> Result<std::process::Output> {
    #[cfg(target_os = "windows")]
    {
        let exe_path = workspace.join("target").join("release").join("bonsai-cli.exe");
        if exe_path.exists() {
            let output = Command::new(exe_path)
                .args(cli_args)
                .current_dir(workspace)
                .output()?;
            return Ok(output);
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let exe_path = workspace.join("target").join("release").join("bonsai-cli");
        if exe_path.exists() {
            let output = Command::new(exe_path)
                .args(cli_args)
                .current_dir(workspace)
                .output()?;
            return Ok(output);
        }
    }

    let mut cargo_args = vec![
        "run".to_string(),
        "-p".to_string(),
        "bonsai-cli".to_string(),
        "--".to_string(),
    ];
    cargo_args.extend(cli_args.iter().cloned());

    let output = Command::new("cargo")
        .args(&cargo_args)
        .current_dir(workspace)
        .output()?;
    Ok(output)
}

fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") {
                    return Some(current);
                }
            }
        }
        if !current.pop() {
            return None;
        }
    }
}
