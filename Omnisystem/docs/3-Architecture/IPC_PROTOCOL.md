# OmniOS Runtime IPC Protocol

## Overview

The VS Code extension host communicates with the OmniOS runtime process (`omnicc runtime --ipc`) using **JSON-RPC 2.0** over stdin/stdout, framed with Content-Length headers identical to the Language Server Protocol. This ensures the protocol is self-delimiting, binary-safe, and compatible with any language or tooling that speaks LSP framing.

---

## Wire Format

Every message is a UTF-8 JSON object preceded by an HTTP-style header block:

```
Content-Length: <byte count of JSON body>\r\n
\r\n
<JSON body>
```

Example request:
```
Content-Length: 89\r\n
\r\n
{"jsonrpc":"2.0","id":1,"method":"fs.readDir","params":{"path":"/projects/myapp"}}
```

Example response:
```
Content-Length: 142\r\n
\r\n
{"jsonrpc":"2.0","id":1,"result":{"entries":[{"name":"main.titan","type":"file"},{"name":"src","type":"directory"}]}}
```

Example notification (no `id` field — no response expected):
```
Content-Length: 78\r\n
\r\n
{"jsonrpc":"2.0","method":"build.line","params":{"phase":"parse","text":"Parsing main.titan...","level":"info"}}
```

---

## Message Types

### Requests (Extension Host → Runtime)
Have an `id` field. The runtime must respond with either `result` or `error`.

### Responses (Runtime → Extension Host)
Match the `id` of the originating request.

### Notifications (Runtime → Extension Host)
No `id`. Used for streaming events: build output lines, terminal output, ML epoch metrics. The extension host registers listeners per event type.

---

## File System Commands (`fs.*`)

### `fs.readDir`
List the contents of a directory.

**Request:**
```json
{"jsonrpc":"2.0","id":1,"method":"fs.readDir","params":{"path":"/projects/myapp"}}
```

**Response:**
```json
{"jsonrpc":"2.0","id":1,"result":{
  "path": "/projects/myapp",
  "entries": [
    {"name":"src","type":"directory","modified":1719619200},
    {"name":"main.titan","type":"file","size":2048,"modified":1719619100},
    {"name":"BUILD.omnisystem","type":"file","size":512,"modified":1719619050}
  ]
}}
```

---

### `fs.readFile`
Read the contents of a file.

**Request:**
```json
{"jsonrpc":"2.0","id":2,"method":"fs.readFile","params":{"path":"/projects/myapp/main.titan"}}
```

**Response:**
```json
{"jsonrpc":"2.0","id":2,"result":{"content":"module Main;\n\nfn main() -> i32 {\n    return 0;\n}\n","encoding":"utf8"}}
```

---

### `fs.writeFile`
Write file contents atomically (write to temp, then rename).

**Request:**
```json
{"jsonrpc":"2.0","id":3,"method":"fs.writeFile","params":{"path":"/projects/myapp/main.titan","content":"module Main;\n\nfn main() -> i32 {\n    log(\"Hello, OmniOS!\");\n    return 0;\n}\n"}}
```

**Response:**
```json
{"jsonrpc":"2.0","id":3,"result":{"bytesWritten":64,"path":"/projects/myapp/main.titan"}}
```

---

### `fs.delete`
Delete a file or empty directory.

**Request:**
```json
{"jsonrpc":"2.0","id":4,"method":"fs.delete","params":{"path":"/projects/myapp/old.titan","recursive":false}}
```

**Response:**
```json
{"jsonrpc":"2.0","id":4,"result":{"deleted":true}}
```

---

### `fs.move`
Move or rename a file or directory.

**Request:**
```json
{"jsonrpc":"2.0","id":5,"method":"fs.move","params":{"src":"/projects/myapp/utils.titan","dst":"/projects/myapp/src/utils.titan"}}
```

**Response:**
```json
{"jsonrpc":"2.0","id":5,"result":{"moved":true}}
```

---

### `fs.search`
Search files by name glob and/or content pattern.

**Request:**
```json
{"jsonrpc":"2.0","id":6,"method":"fs.search","params":{"root":"/projects/myapp","nameGlob":"*.titan","contentPattern":"fn main"}}
```

**Response:**
```json
{"jsonrpc":"2.0","id":6,"result":{"matches":[
  {"path":"/projects/myapp/main.titan","line":3,"col":1,"preview":"fn main() -> i32 {"}
]}}
```

---

### `fs.watch` / `fs.unwatch`
Subscribe to filesystem change events for a directory tree.

**Request:**
```json
{"jsonrpc":"2.0","id":7,"method":"fs.watch","params":{"path":"/projects/myapp","recursive":true}}
```

**Response:**
```json
{"jsonrpc":"2.0","id":7,"result":{"watchId":"watch-001"}}
```

**Notification stream (runtime → host, no id):**
```json
{"jsonrpc":"2.0","method":"fs.changed","params":{"watchId":"watch-001","type":"modified","path":"/projects/myapp/main.titan"}}
```

Change types: `"created"` / `"modified"` / `"deleted"` / `"renamed"` (includes `"oldPath"`).

---

## Build Commands (`build.*`)

### `build.start`
Begin a build. Streams `build.line` and `build.phase` notifications, then sends response when done.

**Request:**
```json
{"jsonrpc":"2.0","id":10,"method":"build.start","params":{
  "target": "x86_64-windows",
  "opt": "O2",
  "flags": ["--release"],
  "cwd": "/projects/myapp"
}}
```

**Notification stream (during build):**
```json
{"jsonrpc":"2.0","method":"build.phase","params":{"phase":"parse","status":"start","timestamp":1719619200000}}
{"jsonrpc":"2.0","method":"build.line","params":{"phase":"parse","text":"Parsing src/main.titan (142 LOC)","level":"info"}}
{"jsonrpc":"2.0","method":"build.line","params":{"phase":"parse","text":"Parsing src/utils.titan (87 LOC)","level":"info"}}
{"jsonrpc":"2.0","method":"build.phase","params":{"phase":"parse","status":"done","duration":234,"fileCount":8}}
{"jsonrpc":"2.0","method":"build.phase","params":{"phase":"resolve","status":"start","timestamp":1719619200234}}
...
{"jsonrpc":"2.0","method":"build.line","params":{"phase":"type","text":"src/main.titan:42:8 — type mismatch: expected i32, got str","level":"error","file":"src/main.titan","line":42,"col":8}}
```

**Response (on completion):**
```json
{"jsonrpc":"2.0","id":10,"result":{
  "code": 0,
  "duration": 847,
  "artifacts": [
    {"path":"target/x86_64-windows/myapp.exe","size":182400,"checksum":"sha256:abc123..."}
  ],
  "warnings": 2,
  "errors": 0
}}
```

Build phases (in order): `parse` → `resolve` → `type` → `lower` → `opt` → `codegen` → `link`

Error response on failure:
```json
{"jsonrpc":"2.0","id":10,"result":{"code":1,"errors":3,"warnings":1,"artifacts":[],"duration":412}}
```

---

### `build.cancel`
Cancel the in-progress build.

**Request:**
```json
{"jsonrpc":"2.0","id":11,"method":"build.cancel","params":{}}
```

**Response:**
```json
{"jsonrpc":"2.0","id":11,"result":{"cancelled":true}}
```

---

### `build.watch`
Start watch mode: rebuilds automatically on file save.

**Request:**
```json
{"jsonrpc":"2.0","id":12,"method":"build.watch","params":{"target":"x86_64-windows","opt":"O0","cwd":"/projects/myapp"}}
```

**Response:**
```json
{"jsonrpc":"2.0","id":12,"result":{"watchId":"build-watch-001"}}
```

Subsequent builds emit the same `build.phase` / `build.line` notifications, tagged with the `watchId`.

---

## Terminal Commands (`term.*`)

### `term.create`
Spawn a new PTY (pseudo-terminal) instance.

**Request:**
```json
{"jsonrpc":"2.0","id":20,"method":"term.create","params":{
  "id": "term-001",
  "cwd": "/projects/myapp",
  "shell": "powershell",
  "cols": 120,
  "rows": 40,
  "env": {"OMNICC_HOME":"/projects/myapp"}
}}
```

**Response:**
```json
{"jsonrpc":"2.0","id":20,"result":{"id":"term-001","pid":12345,"shell":"powershell"}}
```

---

### `term.input`
Send keystrokes to the PTY.

**Request (notification — no response):**
```json
{"jsonrpc":"2.0","method":"term.input","params":{"id":"term-001","data":"omnicc build\r"}}
```

---

### `term.output` (notification stream)
PTY output streamed to the extension host.

```json
{"jsonrpc":"2.0","method":"term.output","params":{"id":"term-001","data":"\x1b[32mBuilding...\x1b[0m\r\n"}}
```

Data is raw VT100/xterm-256 encoded bytes (base64 if binary, UTF-8 string if text).

---

### `term.resize`
Resize the PTY window dimensions.

**Request (notification):**
```json
{"jsonrpc":"2.0","method":"term.resize","params":{"id":"term-001","cols":160,"rows":48}}
```

---

### `term.kill`
Terminate a PTY and its running process.

**Request:**
```json
{"jsonrpc":"2.0","id":21,"method":"term.kill","params":{"id":"term-001","signal":"SIGTERM"}}
```

**Response:**
```json
{"jsonrpc":"2.0","id":21,"result":{"killed":true,"exitCode":0}}
```

---

## Package Manager Commands (`pm.*`)

### `pm.list`
List all installed packages.

**Response result:**
```json
{"packages":[
  {"name":"omni-http","version":"1.2.0","description":"HTTP client and server","size":84200,"installed":"2026-06-01"},
  {"name":"omni-json","version":"2.0.1","description":"JSON parsing and serialization","size":41600,"installed":"2026-06-01"}
]}
```

---

### `pm.search`
Search the OmniPM registry.

**Request params:** `{"query":"http server"}`

**Response result:**
```json
{"results":[
  {"name":"omni-http","version":"1.2.0","description":"HTTP client and server","downloads":48200,"verified":true},
  {"name":"omni-ws","version":"0.8.1","description":"WebSocket server","downloads":12400,"verified":true}
]}
```

---

### `pm.install`
Install a package. Streams `pm.progress` notifications.

**Request params:** `{"pkg":"omni-http","version":"1.2.0"}`

**Notification stream:**
```json
{"jsonrpc":"2.0","method":"pm.progress","params":{"pkg":"omni-http","phase":"download","percent":45,"bytesReceived":38200,"totalBytes":84200}}
{"jsonrpc":"2.0","method":"pm.progress","params":{"pkg":"omni-http","phase":"verify","percent":90}}
{"jsonrpc":"2.0","method":"pm.progress","params":{"pkg":"omni-http","phase":"extract","percent":100}}
```

**Response result:**
```json
{"installed":true,"name":"omni-http","version":"1.2.0","path":"~/.omnisystem/packages/omni-http"}
```

---

### `pm.remove`
Remove an installed package.

**Request params:** `{"pkg":"omni-http"}`

**Response result:**
```json
{"removed":true,"name":"omni-http","freedBytes":84200}
```

---

### `pm.update`
Update all installed packages (or a specific one).

**Request params:** `{"pkg":"omni-http"}` or `{}` for all.

**Response result:**
```json
{"updated":[{"name":"omni-http","from":"1.1.0","to":"1.2.0"}],"upToDate":["omni-json"]}
```

---

### `pm.audit`
Check installed packages for known vulnerabilities.

**Response result:**
```json
{"vulnerabilities":[],"scanned":4,"clean":4,"registryChecked":"2026-06-29T10:00:00Z"}
```

---

## ML Training Commands (`ml.*`)

### `ml.train`
Start training a Sylva model.

**Request params:**
```json
{
  "modelPath": "/projects/myapp/model.sylva",
  "datasetPath": "/projects/myapp/data/train.csv",
  "config": {"epochs":50,"batchSize":32,"learningRate":0.001,"optimizer":"adam"}
}
```

**Notification stream:**
```json
{"jsonrpc":"2.0","method":"ml.epoch","params":{"epoch":1,"epochs":50,"loss":0.8432,"accuracy":0.6120,"valLoss":0.8901,"valAccuracy":0.5980,"duration":2340}}
{"jsonrpc":"2.0","method":"ml.epoch","params":{"epoch":2,"epochs":50,"loss":0.7211,"accuracy":0.6890,"valLoss":0.7830,"valAccuracy":0.6440,"duration":2287}}
```

**Response result on completion:**
```json
{"finalLoss":0.0234,"finalAccuracy":0.9870,"epochs":50,"duration":118000,"modelOutput":"/projects/myapp/trained/model.sylva-model"}
```

---

### `ml.stop`
Stop in-progress training.

**Request params:** `{}`
**Response result:** `{"stopped":true,"epochsCompleted":23}`

---

## App Conversion Commands (`convert.*`)

### `convert.analyze`
Parse a source file and produce a semantic map.

**Request params:**
```json
{"path":"/projects/old/server.js","srcLang":"javascript","targetLang":"aether"}
```

**Response result:**
```json
{"analysis":{
  "functions":12,
  "classes":3,
  "asyncFunctions":8,
  "uiComponents":0,
  "imports":["express","fs","path"],
  "difficulty":"medium",
  "plan":[
    {"src":"async function handleRequest(req,res)","target":"handler HttpRequest(req) in actor WebServer","confidence":0.95},
    {"src":"class UserService","target":"struct UserService + impl block","confidence":0.88},
    {"src":"app.listen(3000)","target":"actor WebServer { message Start{} handler Start(m){ bind(\"0.0.0.0\",3000) } }","confidence":0.91}
  ]
}}
```

---

### `convert.execute`
Execute a conversion plan and write output files.

**Request params:**
```json
{"sourcePath":"/projects/old/server.js","outputDir":"/projects/converted","plan":[...]}
```

**Notification stream:**
```json
{"jsonrpc":"2.0","method":"convert.progress","params":{"file":"server.js","item":"handleRequest","status":"converted","outputFile":"converted/WebServer.aether"}}
{"jsonrpc":"2.0","method":"convert.progress","params":{"file":"server.js","item":"UserService","status":"converted","outputFile":"converted/UserService.titan"}}
{"jsonrpc":"2.0","method":"convert.progress","params":{"file":"server.js","item":"app.use(errorMiddleware)","status":"manual","reason":"Middleware pattern requires manual review","outputFile":"converted/WebServer.aether","line":47}}
```

**Response result:**
```json
{"filesWritten":3,"manualItems":2,"outputDir":"/projects/converted","nextStep":"run `omnicc check` in /projects/converted to see type errors"}
```

---

## System Monitoring Commands (`system.*`)

### `system.stats`
Get current runtime resource usage.

**Response result:**
```json
{"runtime":{
  "heapUsedMB":84.2,
  "heapTotalMB":128.0,
  "rss":210.4,
  "cpuPercent":12.3,
  "activeActors":7,
  "messageQueueDepth":0,
  "gcPauseLastMs":2.1,
  "uptimeSeconds":3842
},"host":{
  "platform":"win32",
  "cpuCores":8,
  "totalMemoryGB":16.0,
  "freeMemoryGB":8.4
}}
```

---

### `system.health`
Check health of all OmniOS services.

**Response result:**
```json
{"services":[
  {"name":"Compiler","status":"ok","latencyMs":12},
  {"name":"LSP","status":"ok","latencyMs":8},
  {"name":"Runtime","status":"ok","latencyMs":3},
  {"name":"OmniPM","status":"ok","latencyMs":145},
  {"name":"Bonsai Launcher","status":"stopped","latencyMs":null},
  {"name":"Bonsai Control Panel","status":"ok","latencyMs":22}
]}
```

---

## Error Handling

All errors follow JSON-RPC 2.0 error format:

```json
{"jsonrpc":"2.0","id":10,"error":{
  "code": -32001,
  "message": "File not found",
  "data": {"path":"/projects/myapp/missing.titan"}
}}
```

### Error Codes

| Code | Meaning |
|---|---|
| -32700 | Parse error (malformed JSON) |
| -32600 | Invalid request |
| -32601 | Method not found |
| -32602 | Invalid params |
| -32603 | Internal error |
| -32001 | File not found |
| -32002 | Permission denied |
| -32003 | Build already in progress |
| -32004 | Terminal session not found |
| -32005 | Package not found in registry |
| -32006 | Checksum verification failed |
| -32007 | Runtime not initialized |
| -32008 | Operation cancelled |

---

## Heartbeat

The extension host sends a heartbeat every 5 seconds. If the runtime misses 3 consecutive heartbeats, RuntimeProcess.ts marks the runtime as crashed and begins the restart sequence.

```json
{"jsonrpc":"2.0","id":999,"method":"ping","params":{}}
```

```json
{"jsonrpc":"2.0","id":999,"result":{"pong":true,"uptime":3842}}
```
