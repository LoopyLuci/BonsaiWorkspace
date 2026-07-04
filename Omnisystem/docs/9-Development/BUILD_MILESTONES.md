# OmniOS Build Milestones — Sequenced Implementation Plan

## Guiding Principle

Build in order of user-visible value. Every milestone must ship something real that users can actually use. No internal-only milestones. No "infrastructure" that isn't immediately exercised by a real user action.

---

## Milestone 1 — Real IPC Foundation
**Goal:** Every click in the desktop does something real. No simulated responses.

### Deliverables

**1.1 — `src/runtime/RuntimeProcess.ts`**
- Spawns `omnicc runtime --ipc` with correct cwd and env
- Monitors health via 5-second heartbeat ping
- Auto-restarts on crash: 500ms → 1s → 2s → 4s backoff, max 3 attempts
- Emits `runtimeReady`, `runtimeCrashed`, `runtimeRestarted` events
- Graceful shutdown on VS Code deactivate

**1.2 — `src/runtime/RuntimeClient.ts`**
- Bidirectional JSON-RPC 2.0 channel over stdin/stdout with Content-Length framing
- Pending-request map: `requestId → {resolve, reject, timeoutHandle}`
- Per-event-type listener registry for notifications (buildLine, termOutput, etc.)
- All requests default to 30-second timeout
- `sendRequest(method, params)` → `Promise<result>`
- `onNotification(method, handler)` → listener registration

**1.3 — `src/runtime/RuntimeProtocol.ts`**
- TypeScript interfaces for every IPC message defined in `IPC_PROTOCOL.md`
- Type-safe wrappers: `runtimeClient.fs.readDir(path)`, `runtimeClient.build.start(opts)`, etc.

**1.4 — `omnicc runtime --ipc` mode in `bin/omnicc.js`**
- Reads and parses Content-Length framed JSON-RPC 2.0 from stdin
- Dispatches to method handlers
- Implements: `fs.readDir`, `fs.readFile`, `fs.writeFile`, `fs.delete`, `fs.move`, `fs.search`, `fs.watch`, `ping`
- `build.start`: spawns `omnicc build` as subprocess, pipes output as `build.line` notifications, sends response on exit
- All fs operations use atomic writes (temp file + rename)

**1.5 — `OmniOSDesktop.ts` refactor**
- Replace all `case 'getFiles':` / `case 'runBuild':` direct handlers with calls to `RuntimeClient`
- Files app now reads real workspace data
- OmniCC Build app now streams real build output

**Quality check:** Open Files app — real workspace directories appear. Click Build — real omnicc output streams.

---

## Milestone 2 — Real Terminal
**Goal:** Terminal app is a real terminal emulator connected to a real shell.

### Deliverables

**2.1 — Add `node-pty` to extension dependencies**
```json
"dependencies": {
  "node-pty": "^1.0.0",
  "xterm": "^5.3.0",
  "xterm-addon-fit": "^0.8.0"
}
```
Note: `node-pty` is a native module — must be built for the correct Electron version using `electron-rebuild`.

**2.2 — `term.*` IPC handlers in runtime**
- `term.create`: calls `pty.spawn(shell, args, {cols, rows, cwd, env})`, stores in a `terms` Map
- `term.input` notification: calls `pty.write(data)` on the identified PTY
- `term.resize` notification: calls `pty.resize(cols, rows)`
- `term.kill`: calls `pty.kill(signal)`, removes from Map
- PTY data events: sends `term.output` notifications to extension host

**2.3 — xterm.js in the webview**
- Terminal app body renders an `<div id="xterm-{id}"></div>` container
- Extension creates an `xterm.Terminal` instance attached to the container
- Incoming `term.output` messages call `terminal.write(data)`
- Keyboard input sends `term.input` notifications
- Window resize triggers `term.resize` via the FitAddon

**2.4 — Multiple terminal tabs**
- Tab bar with `+` button (sends `term.create`), `×` (sends `term.kill`)
- Active tab tracks which `termId` keyboard input routes to
- Tab title updates from the terminal's title escape sequence (`\x1b]0;{title}\x07`)

**Quality check:** Open Terminal app — real PowerShell/bash prompt. Type `dir` or `ls` — real output. Run `omnicc build` — real compiler output with colors.

---

## Milestone 3 — State Persistence
**Goal:** Desktop state survives close and reopen. Nothing resets.

### Deliverables

**3.1 — `~/.omnisystem/state/` directory**
- Created on first run if it doesn't exist
- Contains: `desktop.json`, `files.json`, `terminal.json`, `studio.json`, `build.json`, `ml.json`
- `~/.omnisystem/recent.json`: last 20 opened files with timestamps

**3.2 — Desktop state load on startup**
- Extension host reads `desktop.json` before the webview renders
- Sends initial state to webview in the first `setInitialState` message
- Webview restores all windows at their saved positions, z-order, and minimized/maximized state

**3.3 — Per-app state save**
- Each app calls `saveState(appId, stateObject)` on every meaningful change
- Extension host writes to the appropriate `~/.omnisystem/state/{app}.json`
- Debounced: at most one write per 500ms per app (prevents write storms during dragging)

**3.4 — `system.getState` / `system.setState` IPC commands**
- Runtime provides read/write access to the state files via IPC

**Quality check:** Open Files, navigate to a subfolder. Close the desktop. Reopen — Files app is open at the same directory. Windows are in the same positions.

---

## Milestone 4 — Real Compiler Pipeline
**Goal:** OmniCC `build` produces a real WASM binary. Errors include real file locations.

### Deliverables

**4.1 — Real Titan Parser in `bin/omnicc.js`**
Hand-written recursive descent parser covering the full Titan language grammar:
- Lexer: keywords, identifiers, literals, operators, comments
- Parser: module declarations, function definitions, struct definitions, protocol definitions, impl blocks, let/const bindings, if/else/match/for/while expressions, function calls, binary/unary operators, field access, indexing
- AST output: JSON tree with source locations (file, line, col) on every node
- Error recovery: attempts to continue parsing after an error to report multiple errors in one run

**4.2 — Type Checker**
- Symbol table: built in a first pass over all module-level declarations
- Type inference: bottom-up inference for expressions
- Type errors: include file, line, col, expected type, actual type, and a suggested fix

**4.3 — IR Lowering**
- AST → IrInstruction sequence (Add, Sub, Mul, Div, Call, Return, Jump, JumpIf, Load, Store, Alloc)
- Each IR instruction tagged with its source AST node for error attribution

**4.4 — WASM Codegen via `binaryen.js`**
- IR → Binaryen module via the `binaryen` npm package's JavaScript API
- Outputs a `.wasm` binary file to `target/{target}/`
- Writes a build manifest JSON alongside the binary

**4.5 — Phase events**
- Each compilation phase emits `build.phase {status: 'start'}` before starting and `{status: 'done', duration}` when complete
- File count, error count, and warning count included in phase-done events

**4.6 — Error click-through**
- Error notifications include `{file, line, col}` fields
- `OmniOSDesktop._handleMessage` for `openErrorLocation` calls `vscode.window.showTextDocument` with a `vscode.Position`

**Quality check:** Create a Titan file with a type error. Click Build — errors appear in the output with correct line numbers. Click the error — VS Code opens the file at the exact line.

---

## Milestone 5 — Real Language Server
**Goal:** Hover, completion, and diagnostics work for all 7 Omni-Languages with real type information.

### Deliverables

**5.1 — Incremental parse cache**
- Per-file AST cache: `Map<filePath, {ast, version, symbols}>`
- On `textDocument/didChange`, re-parse only the changed file
- Cross-file symbol table updated incrementally

**5.2 — Real hover** (`textDocument/hover`)
- Find the AST node at the cursor position using the source location tags
- Look up the node's inferred type from the type checker
- Return: `{contents: {kind: 'markdown', value: '```titan\nfn foo(x: i32) -> str\n```\n\nConverts an integer to a string.'}, range: ...}`

**5.3 — Real completion** (`textDocument/completion`)
- Collect all symbols visible at the cursor's scope depth from the symbol table
- Filter by the prefix the user has typed
- Return: `{isIncomplete: false, items: [{label, kind, detail, documentation}]}`
- `kind` maps: function → `Function (3)`, struct → `Struct (22)`, field → `Field (5)`, local → `Variable (6)`

**5.4 — Real diagnostics** (`textDocument/publishDiagnostics`)
- After each `didChange`, wait 300ms (debounce), then re-typecheck the changed file
- Publish all errors and warnings with precise source ranges
- Include a `code` field with the error code and a `codeDescription.href` pointing to the language spec

**5.5 — Go to definition** (`textDocument/definition`)
- Resolve the symbol under the cursor to its declaration location
- Cross-file: walks the symbol table to find the source module and line

**5.6 — Workspace symbol search** (`workspace/symbol`)
- Search all symbols across all files matching the query string
- Used by Code Studio's Symbol Search feature

**Quality check:** Type `fn foo(x: ` in a Titan file — completion shows available types. Hover over a function call — see its signature. Introduce a type error — red squiggle appears within 300ms.

---

## Milestone 6 — Package Manager
**Goal:** `omnicc pm` commands connect to a real registry and install real packages.

### Deliverables

**6.1 — OmniPM Registry Endpoint**
- REST API: `GET /packages?q={query}`, `GET /packages/{name}`, `GET /packages/{name}/{version}/download`
- Package format: `.omnipkg` — ZIP archive containing `manifest.json` + compiled IR + source files
- Hosted at: `https://registry.omnisystem.dev`

**6.2 — `pm.*` IPC handlers in runtime**
- `pm.search`: HTTP GET to registry, returns results
- `pm.install`: HTTP download → SHA-256 verify → extract to `~/.omnisystem/packages/{name}/` → update `omnisystem.lock`
- `pm.list`: reads `~/.omnisystem/packages/` directory
- `pm.remove`: deletes package directory, updates lock file
- `pm.update`: for each installed package, checks registry for newer version
- `pm.audit`: HTTP GET to `https://registry.omnisystem.dev/audit` with installed package list

**6.3 — OmniPM app integration**
- All buttons connect to real IPC commands
- Progress stream updates the progress bar in real time
- Install/remove operations update the installed list immediately

**Quality check:** Search for "omni-http" in OmniPM. Click Install. Watch the real download progress. Check `~/.omnisystem/packages/` — the package directory exists.

---

## Milestone 7 — App Converter
**Goal:** Convert a real JS/Python/Rust file to Omni-Languages with real semantic analysis.

### Deliverables

**7.1 — Tree-sitter grammar bindings**
- Install `tree-sitter` + grammars for JavaScript, TypeScript, Python, Rust, Java, C# as npm deps
- `convert.analyze` IPC handler: parse source file using the appropriate grammar, extract semantic structure

**7.2 — Transformation rules engine**
- Rule definitions: `{pattern: TreeSitterQuery, transform: (node, context) => OmniAst}`
- Rules for each source→target language pair (see `OMNIOS_APPS.md` App Converter section)
- Output is a valid Omni-Language AST, serialized to source text via a pretty-printer

**7.3 — Converter UI — 4-stage flow**
- Stage 1 Analyze: file picker → call analyze → show semantic map
- Stage 2 Plan: editable table of transformations with override dropdowns
- Stage 3 Execute: call execute → progress stream → write output files
- Stage 4 Verify: auto-run `build.start --check-only` → show type errors

**Quality check:** Select a real `server.js` Express app. Analyze — see semantic map with async functions, route handlers. Execute — converted `.aether` and `.titan` files appear in the output directory. Verify — most items typecheck; manual review items are marked.

---

## Milestone 8 — Zero-Knowledge User Experience
**Goal:** A user with no knowledge of OmniOS can open the extension and be building real software in 60 seconds.

### Deliverables

**8.1 — Welcome screen**
- Shown on first launch (detected via absence of `~/.omnisystem/state/welcomed.flag`)
- Four large cards: Desktop App / Web Server / ML Pipeline / Convert Existing Code / Explore OmniOS
- Selecting a card scaffolds the appropriate project template and opens the desktop with the relevant app focused

**8.2 — Guided tour overlay**
- Per-app tours with step-by-step callout bubbles pointing at specific UI elements
- Tour state persisted: shows once, can be re-launched from the `?` button in each app
- Tours are interruptible and resumable

**8.3 — Progressive disclosure**
- Each app has a mode selector: `[Beginner] [Standard] [Expert]`
- Beginner: large buttons, sensible defaults hidden, natural language labels
- Standard: dropdowns visible, documentation links shown
- Expert: raw flags input, all IPC parameters exposed
- Mode persisted per app in state

**8.4 — "Fix it" and "Explain it" on diagnostics**
- Error items in OmniCC Build and Code Studio have two icon buttons: 🔧 Fix it, ℹ️ Explain it
- Fix it: OmniCC synthesizes the correction using type information, shows a diff, applies on confirm
- Explain it: renders a plain-English explanation of the error with a link to the relevant spec section

**8.5 — Contextual help**
- Press `?` in any app → slide-in help panel for that specific app
- Keyboard shortcut reference for the desktop (press `?` on the desktop background)

**Quality check:** On a fresh install, open the extension. Welcome screen appears. Click "Web Server". A project is scaffolded, Code Studio opens. Click Build — the starter project builds successfully. Total time: under 60 seconds.

---

## Milestone 9 — Deployment Packaging
**Goal:** OmniOS ships in 5 forms from the same codebase.

**9.1 — VS Code Extension VSIX** (already done, continuously updated)

**9.2 — Standalone Tauri App**
- `apps/standalone/` directory: Tauri app shell embedding the OmniOS runtime
- Vera renderer replaces the webview layer
- Build script: `npm run build:standalone` → produces `.exe` / `.app` / `.deb`

**9.3 — Container Image**
- `Dockerfile` at repo root: `FROM node:22-slim`, copies runtime, exposes port 7878 (IPC over TCP)
- `docker-compose.yml` with a companion web UI container
- Published to `ghcr.io/omnisystem/omnios:latest`

**9.4 — WASM Browser Build**
- OmniCC runtime compiled to WASM via Binaryen
- Browser entry point at `web/index.html`
- OPFS used for filesystem persistence
- Published to `omnisystem.dev/desktop`

**9.5 — VM Image**
- QEMU image built via GitHub Actions
- GRUB + OmniOS init script + runtime pre-installed
- Published as `.qcow2` download from releases page

---

## Quality Standards (Apply to Every Milestone)

| Standard | Requirement | How to Verify |
|---|---|---|
| IPC response time | < 100ms for all requests (or streaming starts within 100ms) | Add timing logs to RuntimeClient |
| Runtime resilience | Extension host survives runtime crash, auto-restarts, desktop stays open | Kill the runtime process manually |
| Atomic file writes | File writes use temp+rename, never truncate then write | Check with a write that crashes halfway |
| No frozen UI | All IPC calls are async, webview never blocks | Add a 5-second IPC delay artificially |
| No console errors | Zero browser console errors or warnings in normal operation | Open DevTools in the webview |
| Memory | Extension host < 200MB, runtime < 500MB | Check Task Manager during use |
| Startup | Desktop fully interactive within 2 seconds | Time from command to first click working |
| Accessibility | All interactive elements have aria-label, keyboard navigable | Use Tab key only to navigate desktop |
| Error messages | Include: what went wrong, which file/line, what to do | Intentionally trigger every error type |
