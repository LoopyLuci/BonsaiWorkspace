#!/usr/bin/env python3
"""Generate synthetic tool-use DPO data using the teacher as oracle.

The teacher (Qwen3-35B via llama-server on port 8080) is shown a natural language
request and asked to produce the correct JSON tool call. Each tool gets multiple
prompt variants covering different phrasings.

Run with teacher already loaded:
    Start-Process llama-server -ArgumentList "-m D:/Models/general/Qwen3-35B-A22B-Q4_K_M.gguf -ngl 99 --port 8080"
    python scripts/generate_tool_data.py

Offline-safe: no HuggingFace calls.
"""
import json
import pathlib
import urllib.request
import urllib.error
import time
import re
import argparse
import os

# ── Offline enforcement ──────────────────────────────────────────────────────
os.environ.setdefault("TRANSFORMERS_OFFLINE", "1")
os.environ.setdefault("HF_HUB_OFFLINE", "1")
os.environ.setdefault("HF_DATASETS_OFFLINE", "1")
os.environ.setdefault("HF_HUB_DISABLE_TELEMETRY", "1")

TEACHER_URL = "http://127.0.0.1:8080"

# Tool definitions: name → {description, example_prompts, schema}
# Each tool gets 6 prompt variants; teacher fills in the args.
TOOLS: dict[str, dict] = {
    "read_file": {
        "description": "Read the full contents of a file at a given path.",
        "schema": {"path": "string"},
        "prompts": [
            "Show me what's in src/main.rs",
            "What does config.yaml contain?",
            "Read the README file",
            "Print the contents of Cargo.toml",
            "Open bonsai-workspace/src-tauri/src/lib.rs and show me its contents",
            "What's in the justfile?",
        ],
    },
    "write_file": {
        "description": "Write or overwrite a file with the given content.",
        "schema": {"path": "string", "content": "string"},
        "prompts": [
            "Create a new file called hello.py that prints 'hello world'",
            "Save the current config to output/config.json",
            "Write a simple README.md for this project",
            "Create a .gitignore that excludes target/ and node_modules/",
            "Write an empty main.rs file at src/main.rs",
            "Save this JSON data to results.json",
        ],
    },
    "list_files": {
        "description": "List all files in a directory, optionally filtering by extension.",
        "schema": {"path": "string", "extension": "string (optional)"},
        "prompts": [
            "List all files in the src directory",
            "What Rust files are in bonsai-workspace/src-tauri/src?",
            "Show me the Python scripts in the scripts/ folder",
            "List everything in the crates directory",
            "What .svelte files exist in src/lib/components?",
            "Show files in the current directory",
        ],
    },
    "execute_code": {
        "description": "Run code in a sandboxed environment and return its output.",
        "schema": {"code": "string", "language": "python|rust|javascript"},
        "prompts": [
            "Run this Python: print(2 + 2)",
            "Execute the following and show me the result: import sys; print(sys.version)",
            "Run a quick JavaScript snippet: console.log('hello')",
            "Execute this Rust snippet and show stdout",
            "Run the unit tests for the math module",
            "Execute this code and tell me if it works",
        ],
    },
    "run_terminal_command": {
        "description": "Execute a shell command and return stdout/stderr.",
        "schema": {"command": "string"},
        "prompts": [
            "Run cargo build --release",
            "Check the current git status",
            "List the files changed in the last commit",
            "Run npm install in bonsai-workspace/src",
            "Show me the last 10 lines of the log file",
            "Run cargo check --workspace",
        ],
    },
    "web_search": {
        "description": "Search the web and return relevant results.",
        "schema": {"query": "string"},
        "prompts": [
            "Search for Rust serde JSON examples",
            "Look up how to use Tauri's app handle in async contexts",
            "Find documentation on Svelte writable stores",
            "Search for QLoRA training on CPU with transformers",
            "Look up the llama.cpp convert_hf_to_gguf script",
            "Search for tokio::task::block_in_place examples",
        ],
    },
    "browse_url": {
        "description": "Fetch and read the content of a specific URL.",
        "schema": {"url": "string"},
        "prompts": [
            "Fetch the docs at https://docs.rs/tauri/latest/tauri/",
            "Read the page at https://svelte.dev/docs/svelte-store",
            "Open https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html",
            "Get the content from https://crates.io/crates/serde",
            "Fetch https://python.org/docs/3/library/pathlib.html",
            "Read the README on https://github.com/ggerganov/llama.cpp",
        ],
    },
    "search_in_files": {
        "description": "Search for a regex pattern across files in the workspace.",
        "schema": {"pattern": "string", "path": "string (optional)", "file_glob": "string (optional)"},
        "prompts": [
            "Find all usages of AppState in the Rust source",
            "Search for 'bonsai_md' in all .rs files",
            "Find TODO comments in the codebase",
            "Where is MemoryNodeStore defined?",
            "Search for any file importing hot_reload",
            "Find all calls to invoke() in Svelte files",
        ],
    },
    "get_memory_nodes": {
        "description": "Retrieve recent memory nodes from the activity log.",
        "schema": {"limit": "number (optional)", "node_type": "string (optional)"},
        "prompts": [
            "Show me the last 10 memory nodes",
            "What tool calls have been recorded recently?",
            "Show me recent code edits from memory",
            "List the last 20 activity nodes",
            "What chat interactions are in my memory log?",
            "Show me the most recent terminal commands I ran",
        ],
    },
    "record_memory_node": {
        "description": "Record an activity event as a memory node.",
        "schema": {"node_type": "string", "source": "string", "content": "string", "tags": "array of strings"},
        "prompts": [
            "Remember that I fixed the cargo build error by adding the log crate",
            "Record that I added BONSAI.md injection to the submit_chat command",
            "Save a note that the hot reload watcher uses 2-second polling",
            "Record that DirectML cannot be used for training backward passes",
            "Remember this decision: DPO training uses CPU only on this machine",
            "Log that I deployed bonsai-latest.gguf at 14:30",
        ],
    },
    "get_bonsai_md": {
        "description": "Read the current BONSAI.md system prompt content.",
        "schema": {"workspace_path": "string (optional)"},
        "prompts": [
            "Show me the current BONSAI.md",
            "What does the system prompt say?",
            "Read the BONSAI.md file",
            "What's in my self-evolving system prompt?",
            "Display the current Bonsai context",
            "What guidelines am I operating under?",
        ],
    },
    "set_bonsai_md": {
        "description": "Update the BONSAI.md system prompt content.",
        "schema": {"workspace_path": "string", "content": "string"},
        "prompts": [
            "Update BONSAI.md to add a new rule about offline-only training",
            "Set the system prompt to include a reminder about AMD DirectML limitations",
            "Modify BONSAI.md to add my preferred code style",
            "Update the Bonsai context with today's learnings",
            "Add a new section to BONSAI.md about the training pipeline",
            "Write a new BONSAI.md that emphasises safety first",
        ],
    },
    "hot_reload_model": {
        "description": "Trigger a zero-downtime hot reload of the language model.",
        "schema": {"model_path": "string (optional)"},
        "prompts": [
            "Hot reload the model",
            "Swap in the new GGUF without restarting",
            "Reload the model after training",
            "Load the latest bonsai-latest.gguf",
            "Trigger a model swap",
            "Update the running model to the newly trained version",
        ],
    },
    "web_router_fetch": {
        "description": "Fetch a URL through the trusted web router (full content for whitelisted domains, summary otherwise).",
        "schema": {"url": "string"},
        "prompts": [
            "Fetch the Rust documentation through the web router",
            "Get the Tauri API docs via the trusted router",
            "Use the web router to fetch this page",
            "Route this URL through the trusted fetcher",
            "Get docs.rs/serde through the web router",
            "Fetch the MDN page for fetch() API",
        ],
    },
    "resolve_plan": {
        "description": "Approve or reject a pending high-risk plan gate request.",
        "schema": {"plan_id": "string", "approved": "boolean"},
        "prompts": [
            "Approve the pending plan request",
            "Reject the plan gate — don't do that",
            "Allow the queued high-risk operation",
            "Block the pending git push --force",
            "Approve plan-id abc123",
            "Deny the file deletion operation",
        ],
    },
}

SYSTEM_PROMPT = (
    "You are a tool-call assistant for the Bonsai AI workspace. "
    "Given a user request, respond ONLY with a valid JSON object representing the tool call. "
    "The JSON must have exactly two keys: \"tool\" (the tool name) and \"args\" (an object). "
    "Do not include any explanation, markdown, or text outside the JSON object."
)


def teacher_generate(user_prompt: str, tool_name: str, schema: dict, teacher_url: str) -> str | None:
    tool_hint = f"Use the tool \"{tool_name}\" with schema: {json.dumps(schema)}."
    payload = json.dumps({
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user",   "content": f"{tool_hint}\n\nUser request: {user_prompt}"},
        ],
        "max_tokens": 200,
        "temperature": 0.1,
        "stop": ["\n\n"],
    }).encode()
    req = urllib.request.Request(
        f"{teacher_url}/v1/chat/completions",
        data=payload,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            text = json.loads(r.read())["choices"][0]["message"]["content"].strip()
        # Extract first JSON object
        m = re.search(r"\{.*\}", text, re.DOTALL)
        if m:
            candidate = m.group(0)
            json.loads(candidate)  # validate
            return candidate
        return None
    except Exception as e:
        print(f"    [warn] generation failed: {e}")
        return None


def make_wrong_call(tool_name: str) -> str:
    """Generate a plausible wrong tool call (missing args or wrong tool name)."""
    return json.dumps({"tool": tool_name, "args": {}})


def teacher_available(teacher_url: str) -> bool:
    try:
        with urllib.request.urlopen(f"{teacher_url}/health", timeout=3):
            return True
    except Exception:
        return False


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", default=str(pathlib.Path.home() / ".bonsai/training_export/tool_use_synthetic.jsonl"))
    parser.add_argument("--teacher-url", default=TEACHER_URL)
    parser.add_argument("--offline", action="store_true",
                        help="Use rule-based examples only (no teacher calls)")
    args = parser.parse_args()

    output_path = pathlib.Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    use_teacher = not args.offline and teacher_available(args.teacher_url)
    if not use_teacher and not args.offline:
        print("[warn] Teacher not reachable. Using template-only examples (no teacher calls).")
        print("       Start teacher with: llama-server -m D:/Models/general/Qwen3-35B-A22B-Q4_K_M.gguf -ngl 99 --port 8080")

    pairs = []
    for tool_name, info in TOOLS.items():
        for prompt in info["prompts"]:
            if use_teacher:
                chosen = teacher_generate(prompt, tool_name, info["schema"], args.teacher_url)
                time.sleep(0.3)
            else:
                # Template fallback: build a syntactically valid example
                chosen = json.dumps({"tool": tool_name, "args": {"_template": "fill_in"}})

            if not chosen:
                # If teacher failed, skip rather than add bad data
                print(f"    [skip] {tool_name}: {prompt[:50]}")
                continue

            rejected = make_wrong_call(tool_name)
            pairs.append({"prompt": prompt, "chosen": chosen, "rejected": rejected})
            print(f"  [{len(pairs):3d}] {tool_name}: {prompt[:55]}")

    with open(output_path, "w", encoding="utf-8") as f:
        for pair in pairs:
            f.write(json.dumps(pair) + "\n")

    print(f"\nWrote {len(pairs)} tool-use pairs → {output_path}")


if __name__ == "__main__":
    main()
