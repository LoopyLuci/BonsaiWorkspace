#!/usr/bin/env python3
"""
Example Python Agent for Bonsai Ecosystem

This agent connects to the Bonsai Universal Agent Control System via MCP and
demonstrates reading files, running tools, and handling approvals.
"""

import requests
import json
import time
import sys
from typing import Any, Dict

# Configuration
MCP_URL = "http://127.0.0.1:11426/mcp"
CAPABILITY_TOKEN = "demo-token"  # In production, use secure token management

# Tool catalog
TOOLS = {
    "read_file": "Read a file from the workspace",
    "write_file": "Write a file to the workspace",
    "run_cargo_check": "Run 'cargo check' on the workspace",
    "run_cargo_test": "Run 'cargo test' on the workspace",
    "search_codebase": "Search for patterns in the codebase",
    "run_shell": "Execute a shell command",
}

def make_request(method: str, params: Dict[str, Any]) -> Dict[str, Any]:
    """Make an MCP JSON-RPC request to the Bonsai server"""
    payload = {
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": int(time.time() * 1000)
    }

    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {CAPABILITY_TOKEN}"
    }

    response = requests.post(MCP_URL, json=payload, headers=headers)
    response.raise_for_status()
    return response.json()

def list_tools() -> Dict[str, str]:
    """List all available tools"""
    print("\n📋 Available Tools:")
    for tool, desc in TOOLS.items():
        print(f"  • {tool}: {desc}")
    return TOOLS

def read_file(path: str) -> str:
    """Read a file from the workspace"""
    print(f"\n📖 Reading file: {path}")
    result = make_request("tools/call", {
        "name": "read_file",
        "arguments": {"path": path}
    })

    if "error" in result:
        print(f"  ❌ Error: {result['error']}")
        return ""

    content = result.get("result", {}).get("content", [{}])[0].get("text", "")
    print(f"  ✓ Read {len(content)} characters")
    return content

def write_file(path: str, content: str) -> bool:
    """Write a file to the workspace"""
    print(f"\n✍️  Writing file: {path}")
    result = make_request("tools/call", {
        "name": "write_file",
        "arguments": {"path": path, "content": content}
    })

    if "error" in result:
        print(f"  ❌ Error: {result['error']}")
        # If this is a HITL approval request, we would handle it here
        if "requires_approval" in result:
            print(f"  ⏸️  Approval needed! Request ID: {result['request_id']}")
        return False

    print(f"  ✓ Wrote {len(content)} characters")
    return True

def run_cargo_check() -> bool:
    """Run cargo check on the workspace"""
    print(f"\n🔍 Running: cargo check")
    result = make_request("tools/call", {
        "name": "run_cargo_check",
        "arguments": {}
    })

    if "error" in result:
        output = result["error"]
        print(f"  ❌ Build failed:\n{output}")
        return False

    output = result.get("result", {}).get("content", [{}])[0].get("text", "")
    if "error" in output.lower():
        print(f"  ❌ Compilation errors found:\n{output}")
        return False

    print(f"  ✓ Build succeeded")
    return True

def search_codebase(pattern: str) -> list:
    """Search for patterns in the codebase"""
    print(f"\n🔎 Searching for: {pattern}")
    result = make_request("tools/call", {
        "name": "search_codebase",
        "arguments": {"pattern": pattern}
    })

    if "error" in result:
        print(f"  ❌ Error: {result['error']}")
        return []

    matches = result.get("result", {}).get("content", [{}])[0].get("matches", [])
    print(f"  ✓ Found {len(matches)} matches")
    for match in matches[:5]:  # Show first 5
        print(f"    - {match}")
    return matches

def main():
    """Main agent loop"""
    print("╔════════════════════════════════════════════════════════════════╗")
    print("║  🤖 Bonsai Python Agent                                        ║")
    print("║  Connected to Universal Agent Control System                   ║")
    print("╚════════════════════════════════════════════════════════════════╝")

    # Step 1: List available tools
    print("\n▶ Step 1: List available tools")
    list_tools()

    # Step 2: Read Cargo.toml to understand the workspace
    print("\n▶ Step 2: Read Cargo.toml")
    cargo_content = read_file("Cargo.toml")
    if cargo_content:
        lines = cargo_content.split('\n')[:10]
        print(f"  First 10 lines:")
        for line in lines:
            print(f"    {line}")

    # Step 3: Run cargo check
    print("\n▶ Step 3: Run cargo check --workspace")
    if not run_cargo_check():
        print("\n  Note: Fixing compilation errors would go here")
        print("  In this demo, we just report the issue")

    # Step 4: Search for todo!() macros
    print("\n▶ Step 4: Search for unimplemented features")
    todos = search_codebase(r"todo!\(\)|unimplemented!\(\)")
    if todos:
        print(f"  Found {len(todos)} instances of todo!/unimplemented!")

    # Step 5: Create a simple improvement
    print("\n▶ Step 5: Create an improvement file")
    improvement = """# Bonsai Self-Improvement Report

Generated by Python Agent

## Summary
This is an automated report from the Bonsai Python Agent demonstrating
integration with the Universal Agent Control System.

## Steps Completed
1. ✓ Listed available tools
2. ✓ Read workspace configuration (Cargo.toml)
3. ✓ Ran cargo check to verify build status
4. ✓ Searched for unimplemented features (todo!/unimplemented!)

## Observations
- The workspace structure is well-organized
- Multiple crates present: bonsai-mcp-server, bonsai-inference, etc.
- Build status verified

## Next Steps
- Implement missing features (identified by search)
- Add comprehensive tests
- Document public APIs
- Optimize performance hotspots

Generated at: {}
""".format(time.strftime("%Y-%m-%d %H:%M:%S"))

    report_path = "AGENT_REPORT.md"
    if write_file(report_path, improvement):
        print(f"  ✓ Report written to {report_path}")

    print("\n╔════════════════════════════════════════════════════════════════╗")
    print("║ ✅ Agent execution complete                                    ║")
    print("║                                                                ║")
    print("║ All actions were visible on the UACS dashboard:               ║")
    print("║ http://localhost:5173                                         ║")
    print("║                                                                ║")
    print("║ You could approve/deny any file writes or deployments.        ║")
    print("╚════════════════════════════════════════════════════════════════╝")

if __name__ == "__main__":
    try:
        main()
    except Exception as e:
        print(f"\n❌ Error: {e}")
        sys.exit(1)
