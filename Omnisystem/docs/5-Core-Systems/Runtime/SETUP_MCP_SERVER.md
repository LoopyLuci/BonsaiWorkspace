# 🔧 Omnisystem MCP Server Setup Guide

**Purpose:** Enable Claude and all agents to access Omnisystem ecosystem tools via MCP protocol

**Status:** Infrastructure ready, awaiting Rust environment setup

---

## 📋 Quick Start

### For Claude and Agents in VS Code
Add this to `.vscode/settings.json`:

```json
{
  "claudeCode.mcpServers": {
    "omnisystem-mcp": {
      "command": "cargo",
      "args": [
        "run",
        "--manifest-path",
        "Z:\\Projects\\OmnisystemWorkspace\\crates\\mcp-server\\Cargo.toml",
        "--release"
      ],
      "env": {
        "RUST_LOG": "debug,omnisystem_mcp=trace",
        "OMNISYSTEM_CONFIG": "Z:\\Projects\\OmnisystemWorkspace\\.omnisystem\\config.toml"
      }
    }
  }
}
```

### For Anthropic Claude Web Interface
Update `~/Library/Application\ Support/Code/User/settings.json` (macOS) or `%APPDATA%\Code\User\settings.json` (Windows) with the above.

---

## 🎯 Available Tools (30 Total)

### Linting Tools (4)
- `omnisystem_lint` – Lint files/directories
- `omnisystem_apply_fix` – Apply automatic fixes
- `omnisystem_dismiss_diagnostic` – Mark false positives
- `omnisystem_report_false_positive` – Improve rule confidence

### Bug Hunter Tools (10)
- `omnisystem_scan_repo` – Full/incremental/quick repo scan
- `omnisystem_scan_status` – Check scan progress
- `omnisystem_list_findings` – List findings by severity
- `omnisystem_get_finding` – Get finding details
- `omnisystem_auto_fix` – Auto-fix findings
- `omnisystem_explain_diagnostic` – AI explanation
- `omnisystem_prioritize_findings` – Sort by impact/effort
- `omnisystem_generate_report` – Create scan reports

### Phase C Tools (3)
- `omnisystem_verify_rule` – Axiom formal proofs
- `omnisystem_predict_issues` – ML predictions
- `omnisystem_omnisystem_lint` – Titan/Aether/Sylva/Axiom

### Collaboration Tools (4)
- `omnisystem_team_profile` – Team rule overrides
- `omnisystem_vote_proposal` – Rule voting
- `omnisystem_marketplace_search` – Plugin discovery
- `omnisystem_install_plugin` – Install plugins

### Observability Tools (2)
- `omnisystem_metrics` – Real-time metrics
- `omnisystem_impact_analysis` – Bug density impact

---

## 🚀 Installation Steps

### Step 1: Install Rust (if not already installed)

```bash
# Windows (PowerShell)
$ProgressPreference = 'SilentlyContinue'; \
Invoke-WebRequest https://win.rustup.rs/x86_64 -OutFile rustup.exe; \
.\rustup.exe -y; \
$env:PATH += ";$env:USERPROFILE\.cargo\bin"

# macOS/Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Step 2: Build MCP Server

```bash
cd Z:\Projects\OmnisystemWorkspace
cargo build --package mcp-server --release
```

### Step 3: Verify Installation

```bash
cargo run --package mcp-server --release -- --list-tools
```

Expected output:
```
Registered MCP tools:
  ✓ omnisystem_lint
  ✓ omnisystem_scan_repo
  ✓ omnisystem_list_findings
  ... (30 tools total)
```

### Step 4: Start the MCP Server

**Option A: Manual Start**
```bash
cd Z:\Projects\OmnisystemWorkspace
cargo run --package mcp-server --release
```

**Option B: Daemonized (Windows)**
```powershell
# Start in background
Start-Process powershell `
  -ArgumentList "-NoExit -Command `"cd Z:\Projects\OmnisystemWorkspace; cargo run --package mcp-server --release`"" `
  -WindowStyle Minimized
```

**Option C: Systemd Service (Linux/macOS)**
Create `/etc/systemd/user/omnisystem-mcp.service`:
```ini
[Unit]
Description=Omnisystem MCP Server
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/home/user/OmnisystemWorkspace
ExecStart=/usr/local/cargo/bin/cargo run --package mcp-server --release
Restart=always
RestartSec=10

[Install]
WantedBy=default.target
```

Then enable:
```bash
systemctl --user enable omnisystem-mcp
systemctl --user start omnisystem-mcp
```

---

## 🧪 Testing Tools Access

### Test 1: List Available Tools
```bash
curl http://localhost:3000/tools
```

Expected response:
```json
{
  "tools": [
    "omnisystem_lint",
    "omnisystem_scan_repo",
    ... (30 total)
  ]
}
```

### Test 2: Get Tool Definition
```bash
curl http://localhost:3000/tools/omnisystem_lint
```

### Test 3: Call a Tool
```bash
curl -X POST http://localhost:3000/tools/omnisystem_scan_repo \
  -H "Content-Type: application/json" \
  -d '{
    "path": "Z:\\Projects\\OmnisystemWorkspace",
    "mode": "quick",
    "ai_review": true
  }'
```

### Test 4: From Python (MCP Client)
```python
import httpx
import json

client = httpx.Client()

# List tools
response = client.get("http://localhost:3000/tools")
print(response.json())

# Call tool
response = client.post(
    "http://localhost:3000/tools/omnisystem_scan_repo",
    json={
        "path": ".",
        "mode": "quick"
    }
)
print(response.json())
```

---

## 🔐 Security & Configuration

### Environment Variables

Create `.omnisystem/config.toml`:
```toml
[mcp_server]
bind_address = "127.0.0.1"
bind_port = 3000
enable_auth = true
auth_token = "your-secret-token-here"

[linting]
max_files = 10000
max_size_mb = 1000
timeout_seconds = 300

[bug_hunt]
ai_enabled = true
ai_model = "claude-opus-4-8"

[observability]
metrics_enabled = true
tracing_level = "debug"
```

### Access Control
Only agents with valid auth token can call tools:
```bash
curl -H "Authorization: Bearer your-secret-token-here" \
  http://localhost:3000/tools/omnisystem_lint
```

---

## 📊 Monitoring & Logs

### View Logs
```bash
# Real-time logs
journalctl -u omnisystem-mcp -f

# Or from file
tail -f ~/.omnisystem/logs/mcp-server.log
```

### Metrics Endpoint
```bash
curl http://localhost:3000/metrics
```

Response:
```json
{
  "tools_called": 42,
  "scan_duration_ms": 12500,
  "findings_found": 123,
  "cache_hit_rate": 0.87,
  "uptime_seconds": 3600
}
```

---

## 🧠 Using Tools from Claude

Once MCP server is running, Claude can automatically invoke tools:

### Example 1: Scan Repository
```
Claude: "Scan the OmnisystemWorkspace repository for critical issues"

Claude will automatically call:
  omnisystem_scan_repo(
    path="Z:\Projects\OmnisystemWorkspace",
    mode="full",
    ai_review=true
  )
```

### Example 2: Fix Issues
```
Claude: "Find and fix all high-severity bugs in the code"

Claude will:
  1. Call omnisystem_scan_repo() to find issues
  2. Call omnisystem_list_findings() to get details
  3. Call omnisystem_auto_fix() for each fixable issue
  4. Call omnisystem_explain_diagnostic() for others
```

### Example 3: Analyze Impact
```
Claude: "Which rule has the most impact on bug reduction?"

Claude will:
  1. Call omnisystem_metrics() to get current stats
  2. Call omnisystem_impact_analysis() for each rule
  3. Summarize findings
```

---

## 🔧 Troubleshooting

### Issue: "cargo: command not found"
**Solution:** Add Rust to PATH
```bash
# Windows
$env:PATH += ";$env:USERPROFILE\.cargo\bin"

# macOS/Linux
export PATH="$HOME/.cargo/bin:$PATH"
```

### Issue: "Port 3000 already in use"
**Solution:** Use different port
```bash
OMNISYSTEM_MCP_PORT=3001 cargo run --package mcp-server --release
```

### Issue: "MCP server not responding"
**Solution:** Check logs and restart
```bash
# Check if running
curl http://localhost:3000/health

# If not, restart
killall cargo
cargo run --package mcp-server --release
```

---

## 📚 API Reference

### Tool Call Format
```json
{
  "method": "call_tool",
  "params": {
    "name": "tool_name",
    "arguments": {
      "param1": "value1",
      "param2": "value2"
    }
  }
}
```

### Response Format
```json
{
  "status": "success|error",
  "result": {
    "tool": "tool_name",
    "output": "..."
  },
  "error": null
}
```

---

## ✅ Verification Checklist

- [ ] Rust installed and in PATH
- [ ] MCP server builds: `cargo build --package mcp-server --release`
- [ ] Server starts: `cargo run --package mcp-server --release`
- [ ] Health check passes: `curl http://localhost:3000/health`
- [ ] Tools listed: `curl http://localhost:3000/tools`
- [ ] Claude can discover MCP server
- [ ] Can call tools from Claude

---

## 🎯 What's Next

Once MCP server is running:

1. **Scan the codebase** – Identify all issues
2. **Auto-fix where possible** – Automated improvements
3. **Analyze impact** – Understand effect on code quality
4. **Get explanations** – Understand each finding
5. **Implement fixes** – Strategic improvements

---

**Status: Ready for Rust environment setup** ✅

Once Rust is installed, all 30 Omnisystem MCP tools will be available to Claude and all agents.

