# OmniOS Quality Standards

Every component of OmniOS must meet the following standards before being considered production-ready. These are not aspirational — they are pass/fail gates.

---

## Performance Standards

### IPC Response Times
| Operation | Maximum Time | Measured From |
|---|---|---|
| `fs.readDir` (< 1000 files) | 50ms | Request sent to first byte of response |
| `fs.readFile` (< 1MB) | 30ms | Request sent to complete response |
| `fs.writeFile` | 50ms | Request sent to complete response |
| `build.start` first output | 200ms | Button click to first `build.line` notification |
| `pm.search` | 500ms | Request sent to results (includes network) |
| `term.create` | 300ms | Request sent to PTY ready |
| LSP hover | 100ms | Cursor stop to hover shown |
| LSP completion | 50ms | Trigger character to items shown |
| LSP diagnostics | 300ms | Last keystroke to squiggles shown |
| Desktop startup | 2000ms | Command executed to desktop interactive |

### Resource Limits
| Resource | Limit | How to Measure |
|---|---|---|
| Extension host memory | < 200MB RSS | Task Manager / `process.memoryUsage()` |
| Runtime process memory | < 500MB RSS | Task Manager / `system.stats` IPC |
| CPU idle | < 2% | Task Manager when desktop open, no operation |
| CPU during build | < 100% × cores | Expected to saturate all cores; should not exceed |
| Startup CPU spike | < 5 seconds | Time from activation to CPU returning to idle |

---

## Reliability Standards

### Runtime Resilience
The VS Code extension host must survive a crashed runtime process without crashing or requiring a reload.

**Test procedure:**
1. Open the OmniOS Desktop
2. Kill the `omnicc runtime --ipc` process manually (Task Manager → End Task)
3. Expected: Desktop shows "Runtime disconnected — reconnecting..." status
4. Expected: Runtime restarts within 2 seconds
5. Expected: Desktop resumes normal operation without user action
6. Expected: No VS Code "Extension crashed" notification

**Acceptance:** 3/3 kill-and-recover cycles succeed without user intervention.

### File Write Safety
All file writes must be atomic. A crash mid-write must not corrupt the file.

**Test procedure:**
1. Write a large file (> 10MB) via `fs.writeFile`
2. Kill the runtime process during the write
3. Check the file: it must either be the complete new content or the complete original content — never a partial write

**Implementation requirement:** All `fs.writeFile` must:
1. Write to a temp file at `{path}.omni-tmp-{uuid}`
2. Sync the temp file to disk
3. Atomic rename the temp file to `{path}`

### No Frozen UI
The webview must never become unresponsive due to an IPC operation.

**Test procedure:**
1. Introduce an artificial 10-second delay in the runtime's `build.start` handler
2. Click Build in the OmniCC Build app
3. Expected: UI remains responsive — windows can be dragged, other apps can be opened
4. Expected: Build output area shows a "Waiting for response..." indicator

**Implementation requirement:** All IPC calls in `OmniOSDesktop._handleMessage` must be `await`ed without blocking the message handler. The message handler itself must be `async` and must not perform synchronous I/O.

---

## Correctness Standards

### No Console Errors
The webview browser console must show zero errors or warnings during normal operation.

**Test procedure:**
1. Open VS Code DevTools for the OmniOS webview (Help → Toggle Developer Tools → select the webview frame)
2. Open the OmniOS Desktop
3. Open every app
4. Perform the primary action in each app (build, navigate files, run terminal command, etc.)
5. Check the Console tab

**Acceptance:** Zero errors, zero warnings. `console.log` statements used during development must be removed before shipping.

### Type Safety (TypeScript)
The extension TypeScript must compile with `strict: true` and zero errors.

```json
{
  "compilerOptions": {
    "strict": true,
    "noImplicitAny": true,
    "strictNullChecks": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true
  }
}
```

**Test procedure:** `npm run compile` must exit with code 0.

### Content Security Policy
The webview CSP must be maximally restrictive:

```
default-src 'none';
script-src 'nonce-{nonce}';
style-src 'nonce-{nonce}';
img-src data: vscode-resource:;
font-src data:;
```

No `unsafe-inline`, no `unsafe-eval`, no external origins. All scripts and styles must use the per-render nonce.

**Test procedure:** Open webview DevTools → Security tab. Zero CSP violations.

---

## User Experience Standards

### Accessibility
All interactive elements must be keyboard-navigable and screen-reader-compatible.

| Requirement | Standard |
|---|---|
| All buttons | Have `aria-label` if icon-only |
| All inputs | Have associated `<label>` or `aria-label` |
| Focus visible | All focusable elements show a visible focus ring |
| Tab order | Logical tab order within each app window |
| Keyboard desktop | All app windows can be opened, closed, minimized via keyboard |
| Reduced motion | All animations respect `prefers-reduced-motion: reduce` |

**Test procedure:** Navigate the entire desktop using only the Tab, Enter, Space, Escape, and arrow keys. Every function must be reachable.

### Error Message Quality
Every error message shown to the user must include:
1. **What went wrong** — specific, not generic ("File 'main.titan' not found" not "Error")
2. **Where it happened** — file path and line number when applicable
3. **What to do** — a concrete next step ("Set the omnicc path in Settings → OmniCC" not "Configure omnicc")

Bad: `"Build failed"`
Good: `"Build failed: 3 type errors in src/main.titan. Click the error to see the location."`

### Zero-Knowledge Accessibility
A user with zero prior knowledge of OmniOS must be able to:
1. Open the extension
2. See the welcome screen
3. Choose a project type
4. Have a project scaffolded
5. Build it successfully
6. See the output of running it

...without reading any documentation, without configuring any settings, within 60 seconds.

**Test procedure:** Time a fresh install from extension activation to successful first build with a user who has never seen OmniOS before.

**Acceptance:** < 60 seconds, zero documentation consulted.

---

## Security Standards

### No Secret Exfiltration
The webview must never send user data to external servers.

**Requirement:** CSP `default-src 'none'` enforces this at the browser level. Additionally:
- The runtime process must not make outbound network connections to any URL not explicitly initiated by the user (e.g., OmniPM install, OmniPM search)
- Package downloads must use HTTPS with TLS certificate validation
- No telemetry collected without explicit opt-in

### Package Verification
Every OmniPM package installation must:
1. Download over HTTPS
2. Verify the SHA-256 checksum against the registry's published checksum
3. Reject the package if the checksum does not match
4. Extract to a user-owned directory (never to system directories)
5. Run the package's code only inside the OmniCC VM (not as a native process) unless the user explicitly grants native execution permission

### File System Boundaries
The runtime process must not read or write files outside of:
- The current workspace directory
- `~/.omnisystem/` (OmniOS user data)
- System-level reads necessary for shell operation (PATH, environment variables)

**Dangerous operations (delete, overwrite, move)** must show a VS Code modal confirmation dialog before proceeding. The webview's "Delete" button sends a `confirm` IPC command; the extension host shows `vscode.window.showWarningMessage` with Cancel/Delete options; only on "Delete" does the runtime execute the operation.

---

## Versioning Standards

### Extension Version
Follows semantic versioning:
- Major: breaking changes to the IPC protocol or extension API
- Minor: new features, new apps, new IPC commands
- Patch: bug fixes, performance improvements

### IPC Protocol Version
The first message the extension host sends after connecting is:
```json
{"jsonrpc":"2.0","id":0,"method":"runtime.hello","params":{"clientVersion":"2.0.0","protocolVersion":1}}
```

The runtime responds with its version. If the protocol versions are incompatible, the runtime sends an error and the extension shows a "Please update OmniCC" notification.

### OmniCC Version Pinning
The extension's `package.json` specifies a minimum OmniCC version:
```json
"omnisystem.minimumOmniccVersion": "1.0.0"
```

On startup, the extension runs `omnicc --version` and compares against this minimum. If the found version is older, the user sees a notification with a link to the update.

---

## Testing Standards

### Automated Tests Required For
- All IPC message handlers (unit tests with mock stdin/stdout)
- All TypeScript extension modules (unit tests with `@vscode/test-electron`)
- All state persistence logic (round-trip tests: save, reload, compare)
- All file system operations (integration tests against a temp directory)
- The build pipeline (integration test: compile a known Titan file, verify output)
- The LSP server (integration test: send didOpen + hover request, verify response)
- The package manager (integration test against a local mock registry)

### Test Naming Convention
```
describe('RuntimeClient', () => {
  describe('sendRequest', () => {
    it('resolves when runtime responds with result', async () => { ... });
    it('rejects when runtime responds with error', async () => { ... });
    it('rejects with timeout error after 30 seconds', async () => { ... });
    it('rejects all pending requests when runtime crashes', async () => { ... });
  });
});
```

### Coverage Target
- Extension host code: > 80% line coverage
- IPC message handlers: 100% handler coverage (every method has at least one test)
- Critical paths (file write atomicity, error recovery): 100% branch coverage
