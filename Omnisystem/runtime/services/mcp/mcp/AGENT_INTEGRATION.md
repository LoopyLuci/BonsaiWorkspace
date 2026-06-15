# 🤖 Agent Integration Guide - Using Bonsai MCP Tools

**Purpose:** Enable Claude and other agents to automatically use Bonsai MCP tools for code quality, bug hunting, and verification

**Status:** Ready for use once MCP server is running

---

## 🎯 What Agents Can Do Now

### Claude (in VS Code or Web)
Once MCP server starts, Claude can automatically:

1. **Scan repositories** for bugs, vulnerabilities, and issues
2. **Apply fixes** automatically
3. **Explain findings** in natural language
4. **Verify rules** using formal proofs (Axiom)
5. **Predict issues** before they occur (ML)
6. **Manage team profiles** and voting
7. **Search marketplace** and install plugins
8. **Access real-time metrics** and dashboards

### Other Agents
Any agent that implements MCP client protocol can:
- List available tools
- Call tools with arguments
- Process results
- Chain tool calls

---

## 💬 How to Ask Claude

### Example 1: Full Vulnerability Scan

**You ask:**
```
"Scan the BonsaiWorkspace repository for all critical and high-severity 
vulnerabilities. Show me the findings, explain each one, and fix the ones 
you can automatically."
```

**Claude automatically:**
1. Calls `bonsai_scan_repo(path="Z:\Projects\BonsaiWorkspace", mode="full", ai_review=true)`
2. Gets back `scan_id: "scan-abc123"`
3. Calls `bonsai_list_findings(scan_id="scan-abc123", severity="critical,high")`
4. For each finding, calls `bonsai_explain_diagnostic(finding_id=...)`
5. For fixable findings, calls `bonsai_auto_fix(finding_id=...)`
6. Summarizes results

**Claude shows you:**
```
Found 12 issues in BonsaiWorkspace:

CRITICAL (3):
  ▸ SQL Injection in user_handler.rs:145
    Explanation: User input is directly concatenated into SQL query...
    Status: FIXED ✓

  ▸ Missing error handling in main.rs:78
    Explanation: Function doesn't catch potential panics...
    Status: Manual review needed

HIGH (9):
  ▸ Unused variable in lib.rs:234
    Status: FIXED ✓
  ...
```

### Example 2: Formal Verification

**You ask:**
```
"Verify that the 'unused-import' rule is sound using Axiom. 
If it's verified, mark it as trusted."
```

**Claude calls:**
```
bonsai_verify_rule(rule_id="unused-import", proof_level="soundness")
```

**Result:**
```
Rule "unused-import" - VERIFIED ✓
Proof level: Soundness
FP guarantee: <1%
Status: TRUSTED (can run with higher privilege)
```

### Example 3: Impact Analysis

**You ask:**
```
"Which rules have the biggest impact on bug reduction? 
Show me the metrics and trends."
```

**Claude calls:**
1. `bonsai_metrics(metric="cache_hit_rate", time_range="30d")`
2. `bonsai_impact_analysis(rule_id=...)` for top rules
3. `bonsai_metrics(metric="false_positive_rate", time_range="30d")`

**Result:**
```
Top Rules by Impact:

1. unused-import
   Bug density reduction: 18% ↓
   False positive rate: 1.2%
   Trend: Improving ↑

2. unhandled-error
   Bug density reduction: 24% ↓
   False positive rate: 0.8%
   Trend: Stable →

3. type-mismatch
   Bug density reduction: 31% ↓
   False positive rate: 0.3%
   Trend: Improving ↑
```

### Example 4: Multi-Language Omnisystem Linting

**You ask:**
```
"Lint all Titan, Aether, Sylva, and Axiom files in the project. 
Focus on effect violations and type safety issues."
```

**Claude calls:**
- `bonsai_omnisystem_lint(file="...", language="titan")`
- `bonsai_omnisystem_lint(file="...", language="aether")`
- `bonsai_omnisystem_lint(file="...", language="sylva")`
- `bonsai_omnisystem_lint(file="...", language="axiom")`

**Result:**
```
TITAN files (3):
  ✓ No effect violations found
  ✓ All effect annotations correct

AETHER files (2):
  ⚠ Missing supervision for actor_pool
  ✓ Dead-letter queue properly configured

SYLVA files (5):
  ✓ No injection vulnerabilities
  ⚠ Potential sandbox escape in script_executor.sylva:34

AXIOM files (1):
  ✓ All type proofs valid
  ✓ Memory safety guaranteed
```

---

## 🔄 Tool Chaining Examples

### Pattern 1: Find and Fix

```
Claude chains:
1. bonsai_scan_repo() → scan_id
2. bonsai_list_findings(scan_id) → findings[]
3. FOR EACH finding:
   - bonsai_auto_fix(finding_id) if fixable
   - bonsai_explain_diagnostic(finding_id) if not
```

### Pattern 2: Verify and Deploy

```
Claude chains:
1. bonsai_verify_rule(rule_id) for each rule
2. If verified, promote to "trusted"
3. bonsai_marketplace_search() to find similar rules
4. Compare impact metrics
```

### Pattern 3: Monitor and Alert

```
Claude chains:
1. bonsai_metrics(metric="false_positive_rate")
2. IF fp_rate > threshold:
   - bonsai_list_findings() to find culprits
   - Recommend rule adjustments
```

---

## 📡 MCP Protocol Details

### Tool Discovery
Agents automatically discover available tools on startup:

```json
{
  "method": "list_tools",
  "params": {}
}
```

Response lists all 30 tools with schemas.

### Calling Tools

```json
{
  "method": "call_tool",
  "params": {
    "name": "bonsai_scan_repo",
    "arguments": {
      "path": "Z:\\Projects\\BonsaiWorkspace",
      "mode": "full",
      "ai_review": true,
      "output_format": "json"
    }
  }
}
```

### Streaming Results

For long-running tools like `bonsai_scan_repo`, results stream back:

```json
{
  "method": "call_tool",
  "params": {
    "name": "bonsai_scan_repo",
    "stream": true,
    "arguments": {...}
  }
}
```

Events stream:
```
{"event": "scan_started", "scan_id": "scan-abc123"}
{"event": "file_scanned", "file": "src/main.rs", "findings": 5}
{"event": "file_scanned", "file": "src/lib.rs", "findings": 3}
...
{"event": "scan_complete", "total_findings": 87}
```

---

## 🎨 Building Custom Workflows

### Workflow 1: Continuous Code Quality

```
Every 1 hour:
  1. bonsai_scan_repo(mode="incremental")
  2. bonsai_list_findings(severity="high|critical")
  3. If findings > threshold:
     - bonsai_prioritize_findings(strategy="impact")
     - Alert team
     - Create issues
```

### Workflow 2: Rule Verification Pipeline

```
When new rule created:
  1. bonsai_verify_rule(rule_id, proof_level="soundness")
  2. If verified:
     - bonsai_marketplace_publish(rule)
  3. Else:
     - Log for manual review
     - Suggest refinements
```

### Workflow 3: Team Collaboration

```
When proposal created:
  1. bonsai_team_profile(team_id) to get current rules
  2. Test proposal against codebase
  3. bonsai_prioritize_findings() by impact
  4. Present results
  5. When approved: bonsai_vote_proposal(vote="approve")
```

---

## 🔒 Security Considerations

### Tool Permissions

Some tools require elevated permissions:
- `bonsai_auto_fix` – modify files
- `bonsai_install_plugin` – change configuration
- `bonsai_marketplace_publish` – publish to marketplace

These require explicit approval or auth token.

### Rate Limiting

MCP server implements rate limits:
- 100 tool calls per minute per agent
- 10 concurrent scans maximum
- 1GB max scan output per request

### Audit Trail

All tool calls are logged:
```
[2026-06-02 10:45:23] TOOL CALL: bonsai_scan_repo
  Agent: claude-opus-4-8
  Args: path=Z:\Projects\BonsaiWorkspace, mode=full
  Duration: 12500ms
  Status: success
  Findings: 87
```

---

## 📊 Monitoring Agent Usage

### View Agent Tool Calls

```bash
# Last 100 tool calls
curl http://localhost:3000/api/audit/tool-calls?limit=100

# Calls by agent
curl http://localhost:3000/api/audit/by-agent

# Calls by tool
curl http://localhost:3000/api/audit/by-tool
```

### Performance Metrics

```bash
# Average tool latency
curl http://localhost:3000/metrics/latency

# Tool success rate
curl http://localhost:3000/metrics/success-rate

# Agent activity
curl http://localhost:3000/metrics/agents
```

---

## 🚀 Getting Started

### Step 1: Start MCP Server
```bash
cd Z:\Projects\BonsaiWorkspace
cargo run --package mcp-server --release
```

### Step 2: Verify Server is Running
```bash
curl http://localhost:3000/health
# Response: {"status": "ok"}
```

### Step 3: List Available Tools
```bash
curl http://localhost:3000/tools | jq '.[] | .name'
```

### Step 4: Ask Claude to Use Tools

In VS Code or web interface:
```
"Scan the BonsaiWorkspace repository for all vulnerabilities 
and automatically fix the ones you can."
```

Claude will automatically use the MCP tools!

---

## 🧪 Testing Tools Manually

### Test Lint Tool
```bash
curl -X POST http://localhost:3000/tools/bonsai_lint \
  -H "Content-Type: application/json" \
  -d '{
    "path": "Z:\\Projects\\BonsaiWorkspace",
    "languages": ["rust", "python"],
    "fix": false
  }'
```

### Test Bug Hunter
```bash
curl -X POST http://localhost:3000/tools/bonsai_scan_repo \
  -H "Content-Type: application/json" \
  -d '{
    "path": "Z:\\Projects\\BonsaiWorkspace",
    "mode": "quick",
    "ai_review": true
  }'
```

### Test Formal Verification
```bash
curl -X POST http://localhost:3000/tools/bonsai_verify_rule \
  -H "Content-Type: application/json" \
  -d '{
    "rule_id": "unused-import",
    "proof_level": "soundness"
  }'
```

---

## 📚 Full Tool Reference

See `mcp/mcp_config.json` for complete tool definitions including:
- Input schemas
- Output formats
- Required parameters
- Optional parameters

---

## ✅ Verification Checklist

- [ ] MCP server running: `curl http://localhost:3000/health`
- [ ] Tools listed: `curl http://localhost:3000/tools | jq length`
- [ ] Claude can discover MCP server
- [ ] Can call `bonsai_lint` successfully
- [ ] Can call `bonsai_scan_repo` successfully
- [ ] Can call `bonsai_verify_rule` successfully
- [ ] Can chain multiple tools together

---

**Status: Ready for use** ✅

Once Rust environment is set up and MCP server is running, all 30 tools are available to Claude and other agents.

