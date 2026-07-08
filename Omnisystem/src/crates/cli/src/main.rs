use anyhow::{anyhow, bail, Context, Result};
use blake3::Hasher;
use cargo_metadata::MetadataCommand;
use clap::{Parser, Subcommand, ValueEnum};
use rayon::prelude::*;
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

mod bug_hunt;

#[derive(Parser, Debug)]
#[command(name = "bonsai")]
#[command(about = "Unified Bonsai Developer Toolkit CLI")]
struct Cli {
    #[arg(long)]
    json: bool,
    #[arg(long)]
    workspace: Option<PathBuf>,
    #[arg(long)]
    verbose: bool,
    #[arg(long)]
    dry_run: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Setup,
    Build {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        release: bool,
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        crates: Option<String>,
        #[arg(long, default_value_t = 4)]
        parallel: usize,
    },
    Test {
        #[arg(long)]
        workspace: bool,
        #[arg(long)]
        unit: bool,
        #[arg(long)]
        integration: bool,
        #[arg(long)]
        performance: bool,
    },
    Lint,
    Run {
        service: String,
        #[arg(long)]
        detach: bool,
        #[arg(long)]
        port: Option<u16>,
    },
    Stop {
        service: String,
    },
    ListDetached,
    Logs {
        service: String,
        #[arg(long)]
        follow: bool,
    },
    Deploy {
        target: String,
    },
    Docs {
        #[arg(long)]
        serve: bool,
        #[arg(long)]
        port: Option<u16>,
    },
    New {
        #[arg(value_enum)]
        kind: NewKind,
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: Option<String>,
    },
    Agent {
        action: String,
    },
    Mcp {
        action: String,
    },
    Doctor {
        #[arg(long)]
        fix: bool,
    },
    BugHunt {
        #[command(subcommand)]
        action: BugHuntAction,
    },
    Status,
    Clean {
        #[arg(long)]
        cache: bool,
    },
    Version,
}

#[derive(Clone, Debug, ValueEnum)]
enum NewKind {
    Crate,
    AndroidApp,
}

#[derive(Clone, Debug, Subcommand)]
enum BugHuntAction {
    Scan {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, value_enum)]
        format: Option<ReportFormat>,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        quick: bool,
        #[arg(long)]
        ai: bool,
    },
    List {
        #[arg(long, value_enum)]
        severity: Option<String>,
    },
    Fix {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        all: bool,
        #[arg(long)]
        confirm: bool,
    },
    Status,
    ClearCache,
}

#[derive(Clone, Debug, ValueEnum)]
enum ReportFormat {
    Json,
    Sarif,
    Html,
    Markdown,
}

#[derive(Debug)]
enum DevKitCapability {
    Build,
    Test,
    Run,
    Deploy,
    Docs,
    Clean,
}

impl DevKitCapability {
    fn as_str(&self) -> &'static str {
        match self {
            DevKitCapability::Build => "DevKitCap:build",
            DevKitCapability::Test => "DevKitCap:test",
            DevKitCapability::Run => "DevKitCap:run",
            DevKitCapability::Deploy => "DevKitCap:deploy",
            DevKitCapability::Docs => "DevKitCap:docs",
            DevKitCapability::Clean => "DevKitCap:clean",
        }
    }
}

#[derive(Serialize)]
struct CommandResult {
    ok: bool,
    command: String,
    message: String,
    data: Option<Value>,
}

#[derive(Serialize)]
struct DevKitEvent {
    timestamp_ms: u128,
    event: String,
    command: String,
    duration_ms: Option<u128>,
    success: Option<bool>,
    error: Option<String>,
}

fn emit(json: bool, result: CommandResult) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
        );
    } else if result.ok {
        println!("[ok] {}", result.message);
    } else {
        println!("[error] {}", result.message);
    }
}

fn detect_workspace_root(start: &Path) -> Result<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join("Cargo.toml");
        if candidate.exists() {
            let content = fs::read_to_string(&candidate)
                .with_context(|| format!("failed reading {}", candidate.display()))?;
            if content.contains("[workspace]") {
                return Ok(current);
            }
        }
        if !current.pop() {
            break;
        }
    }
    bail!("could not find workspace root")
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_millis(0))
        .as_millis()
}

fn emit_event(workspace: &Path, event: DevKitEvent) {
    let logs_dir = workspace.join("logs");
    let _ = fs::create_dir_all(&logs_dir);
    let file = logs_dir.join("devkit-events.jsonl");
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(file) {
        if let Ok(line) = serde_json::to_string(&event) {
            let _ = writeln!(f, "{}", line);
        }
    }
}

fn parse_capabilities() -> HashSet<String> {
    let mut caps = HashSet::new();
    if let Ok(raw) = std::env::var("BONSAI_CAP_TOKEN") {
        if raw.trim().is_empty() {
            return caps;
        }
        if let Ok(v) = serde_json::from_str::<Value>(&raw) {
            if let Some(arr) = v.get("capabilities").and_then(|x| x.as_array()) {
                for cap in arr.iter().filter_map(|x| x.as_str()) {
                    caps.insert(cap.to_string());
                }
                return caps;
            }
        }
        for cap in raw.split(',') {
            let c = cap.trim();
            if !c.is_empty() {
                caps.insert(c.to_string());
            }
        }
    }
    caps
}

fn require_capability(cap: DevKitCapability) -> Result<()> {
    let caps = parse_capabilities();
    if caps.is_empty() {
        // Local default: allow when no token is provided.
        return Ok(());
    }
    if caps.contains("DevKitCap:*") || caps.contains(cap.as_str()) {
        Ok(())
    } else {
        bail!("missing capability {}", cap.as_str())
    }
}

fn run_cmd(workspace: &Path, dry_run: bool, verbose: bool, program: &str, args: &[&str]) -> Result<()> {
    if dry_run {
        println!("DRY RUN: {} {}", program, args.join(" "));
        return Ok(());
    }
    if verbose {
        println!("> {} {}", program, args.join(" "));
    }
    let status = Command::new(program)
        .args(args)
        .current_dir(workspace)
        .status()
        .with_context(|| format!("failed to run {}", program))?;
    if !status.success() {
        bail!("command failed: {} {}", program, args.join(" "));
    }
    Ok(())
}

fn script_for_os(base: &str, windows_ext: &str, unix_ext: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{}{}", base, windows_ext)
    } else {
        format!("{}{}", base, unix_ext)
    }
}

fn pid_dir(workspace: &Path) -> PathBuf {
    workspace.join(".bonsai").join("pids")
}

fn log_dir(workspace: &Path) -> PathBuf {
    workspace.join("logs")
}

fn spawn_service(
    workspace: &Path,
    service: &str,
    port: Option<u16>,
    detach: bool,
    dry_run: bool,
) -> Result<Option<u32>> {
    let mut cmd = if service == "desktop" {
        let mut c = Command::new("npm");
        c.current_dir(workspace.join("bonsai-workspace"))
            .args(["run", "tauri", "dev"]);
        c
    } else {
        let mut c = Command::new("cargo");
        c.current_dir(workspace);
        match service {
            "mcp-server" => {
                c.args(["run", "-p", "mcp-server", "--"]);
                if let Some(p) = port {
                    c.args(["--port", &p.to_string()]);
                }
            }
            "uacs" => {
                c.args(["run", "-p", "mcp-server", "--", "visual"]);
                if let Some(p) = port {
                    c.args(["--port", &p.to_string()]);
                }
            }
            "remote-desktop" => {
                c.args(["run", "-p", "bonsai-remote-desktop", "--", "--host"]);
                if let Some(p) = port {
                    c.args(["--port", &p.to_string()]);
                }
            }
            "tui" => {
                c.args(["run", "-p", "bonsai-tui"]);
            }
            _ => bail!("unsupported service: {}", service),
        }
        c
    };

    if dry_run {
        return Ok(None);
    }

    if detach {
        let pids = pid_dir(workspace);
        let logs = log_dir(workspace);
        fs::create_dir_all(&pids)?;
        fs::create_dir_all(&logs)?;
        let out_path = logs.join(format!("{}.out.log", service));
        let err_path = logs.join(format!("{}.err.log", service));
        let out = File::create(out_path)?;
        let err = File::create(err_path)?;
        let child = cmd.stdout(Stdio::from(out)).stderr(Stdio::from(err)).spawn()?;
        let pid = child.id();
        fs::write(pids.join(format!("{}.pid", service)), pid.to_string())?;
        Ok(Some(pid))
    } else {
        let status = cmd.status()?;
        if !status.success() {
            bail!("service exited with error")
        }
        Ok(None)
    }
}

fn stop_service(workspace: &Path, service: &str) -> Result<()> {
    let pid_file = pid_dir(workspace).join(format!("{}.pid", service));
    if !pid_file.exists() {
        bail!("no pid file for service {}", service);
    }
    let pid_str = fs::read_to_string(&pid_file)?;
    let pid = pid_str.trim();
    if cfg!(target_os = "windows") {
        let status = Command::new("taskkill").args(["/PID", pid, "/F"]).status()?;
        if !status.success() {
            bail!("failed to stop pid {}", pid);
        }
    } else {
        let status = Command::new("kill").args(["-TERM", pid]).status()?;
        if !status.success() {
            bail!("failed to stop pid {}", pid);
        }
    }
    fs::remove_file(pid_file)?;
    Ok(())
}

fn list_detached(workspace: &Path) -> Result<Vec<HashMap<String, String>>> {
    let mut rows = Vec::new();
    let pids = pid_dir(workspace);
    if !pids.exists() {
        return Ok(rows);
    }
    for entry in fs::read_dir(pids)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|x| x.to_str()) == Some("pid") {
            let service = path
                .file_stem()
                .and_then(|x| x.to_str())
                .unwrap_or("unknown")
                .to_string();
            let pid = fs::read_to_string(&path).unwrap_or_else(|_| "unknown".to_string());
            let mut m = HashMap::new();
            m.insert("service".to_string(), service);
            m.insert("pid".to_string(), pid.trim().to_string());
            rows.push(m);
        }
    }
    Ok(rows)
}

fn tail_logs(workspace: &Path, service: &str, follow: bool) -> Result<()> {
    let file_path = log_dir(workspace).join(format!("{}.out.log", service));
    if !file_path.exists() {
        bail!("log file not found: {}", file_path.display());
    }

    if !follow {
        let content = fs::read_to_string(file_path)?;
        print!("{}", content);
        return Ok(());
    }

    let mut file = OpenOptions::new().read(true).open(file_path)?;
    let mut pos = file.metadata()?.len();
    loop {
        let len = file.metadata()?.len();
        if len > pos {
            file.seek(SeekFrom::Start(pos))?;
            let mut buf = String::new();
            file.read_to_string(&mut buf)?;
            print!("{}", buf);
            pos = len;
        }
        std::thread::sleep(Duration::from_millis(700));
    }
}

#[derive(Clone)]
struct CrateInfo {
    name: String,
    root: PathBuf,
}

fn workspace_crates(workspace: &Path) -> Result<Vec<CrateInfo>> {
    let meta = MetadataCommand::new().current_dir(workspace).exec()?;
    let mut crates = Vec::new();
    for pkg in meta.workspace_packages() {
        let root = pkg
            .manifest_path
            .as_std_path()
            .parent()
            .ok_or_else(|| anyhow!("invalid manifest path for {}", pkg.name))?
            .to_path_buf();
        crates.push(CrateInfo {
            name: pkg.name.to_string(),
            root,
        });
    }
    Ok(crates)
}

fn crate_hash(root: &Path) -> Result<String> {
    let mut hasher = Hasher::new();
    let mut files = Vec::new();
    let cargo = root.join("Cargo.toml");
    if cargo.exists() {
        files.push(cargo);
    }
    let build_rs = root.join("build.rs");
    if build_rs.exists() {
        files.push(build_rs);
    }
    let src = root.join("src");
    if src.exists() {
        for e in WalkDir::new(src).into_iter().filter_map(|x| x.ok()) {
            if e.file_type().is_file()
                && e.path().extension().and_then(|x| x.to_str()) == Some("rs")
            {
                files.push(e.path().to_path_buf());
            }
        }
    }
    files.sort();
    for f in files {
        hasher.update(f.to_string_lossy().as_bytes());
        let bytes = fs::read(&f)?;
        hasher.update(&bytes);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn build_with_cache(workspace: &Path, release: bool, parallel: usize, dry_run: bool, verbose: bool) -> Result<(usize, usize)> {
    let crates = workspace_crates(workspace)?;
    let cache_root = workspace.join(".bonsai").join("cache").join("build");
    fs::create_dir_all(&cache_root)?;

    let mut to_build = Vec::new();
    let mut skipped = 0usize;

    for c in crates {
        let hash = crate_hash(&c.root)?;
        let marker = cache_root.join(format!("{}-{}.ok", c.name.replace('/', "_"), hash));
        if marker.exists() {
            skipped += 1;
        } else {
            to_build.push((c.name, marker));
        }
    }

    if to_build.is_empty() {
        return Ok((0, skipped));
    }

    let _ = rayon::ThreadPoolBuilder::new().num_threads(parallel).build_global();

    let results: Vec<Result<()>> = to_build
        .par_iter()
        .map(|(name, marker)| {
            if dry_run {
                return Ok(());
            }
            if verbose {
                println!("> cargo build -p {}