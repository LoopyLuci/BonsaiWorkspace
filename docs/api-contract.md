# Bonsai Buddy API Contract

**Version**: 1  
**Server**: `http://127.0.0.1:11420` (default; port may shift +1 through +4 on conflict)  
**Auth**: None required (loopback-only binding; see Security below)  
**Protocol**: HTTP/1.1; JSON request and response bodies; SSE for streaming

---

## Endpoints

### `GET /health`

Liveness probe. Returns server status and the active port.

**Response** `200 OK`:
```json
{
  "status": "ok",
  "port": 11420
}
```

---

### `GET /v1/models`

Returns the static model list in OpenAI format.

**Response** `200 OK`:
```json
{
  "object": "list",
  "data": [
    {
      "id": "bonsai-buddy",
      "object": "model",
      "owned_by": "bonsai"
    }
  ]
}
```

---

### `POST /v1/chat/completions`

Main inference endpoint. Accepts an OpenAI-compatible chat request and returns a response (non-streaming) or an SSE stream (streaming).

#### Request

```json
{
  "model":      "bonsai-buddy",
  "messages":   [ { "role": "user", "content": "Hello" } ],
  "stream":     false,
  "max_tokens": 2048
}
```

| Field        | Type              | Required | Default        | Description |
|--------------|-------------------|----------|----------------|-------------|
| `model`      | string            | No       | active profile | Model hint. Accepts `"bonsai-buddy"`, `"auto"`, or a specific model ID. Ignored if no loaded slot matches. |
| `messages`   | array of objects  | Yes      | —              | Standard OpenAI message array: `{ "role": "system" \| "user" \| "assistant", "content": string }` |
| `stream`     | boolean           | No       | `false`        | If `true`, response is delivered as SSE. |
| `max_tokens` | integer           | No       | `2048`         | Maximum tokens for the completion. |

**`bonsai_ext` extension on `messages`**: The last message in the array may carry an extension object for the confirmation protocol (see [Confirmation Flow](#confirmation-flow) below).

---

#### Response (non-streaming, `stream: false`)

**Success** `200 OK`:
```json
{
  "id":      "buddy-abc123",
  "object":  "chat.completion",
  "choices": [
    {
      "index":         0,
      "message":       { "role": "assistant", "content": "Hello! How can I help?" },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens":     0,
    "completion_tokens": 0
  }
}
```

| Field                     | Type   | Description |
|---------------------------|--------|-------------|
| `id`                      | string | Request identifier, prefixed `buddy-`. |
| `object`                  | string | Always `"chat.completion"`. |
| `choices[0].message`      | object | The assistant's reply. |
| `choices[0].finish_reason`| string | `"stop"` on normal completion. `"tool_calls_pending_approval"` when user confirmation is required (see below). |
| `usage`                   | object | Token counts (currently zeroed — placeholder for future billing). |
| `bonsai_ext`              | object | Present only when `finish_reason` is `"tool_calls_pending_approval"`. See [Confirmation Flow](#confirmation-flow). |

---

#### Response (streaming, `stream: true`)

Delivers tokens as [Server-Sent Events (SSE)](https://html.spec.whatwg.org/multipage/server-sent-events.html).

Each event:
```
data: {"id":"buddy-abc123","object":"chat.completion.chunk","choices":[{"delta":{"content":"Hello"},"index":0,"finish_reason":null}]}
```

Stream errors are delivered as a final data event:
```
data: {"error":{"type":"buddy_error","message":"No model slot ready."}}
```

The SSE stream ends when the underlying `mpsc` channel closes (the spawned inference task completes or errors).

---

## `bonsai_ext` Extension Fields

All `bonsai_ext` objects carry a `schema` version field for future compatibility. The current version is `1`.

### On response — `confirm_required`

When the assistant wants to call a tool that requires user confirmation (e.g., `run_command`), the response has `finish_reason: "tool_calls_pending_approval"` and:

```json
{
  "bonsai_ext": {
    "schema":     1,
    "type":       "confirm_required",
    "token":      "tok_a1b2c3",
    "tool":       "run_command",
    "args":       { "command": "ls -la" },
    "prompt":     "Run command: ls -la",
    "expires_at": 1746400800
  }
}
```

| Field        | Type    | Description |
|--------------|---------|-------------|
| `schema`     | integer | Always `1`. |
| `type`       | string  | Always `"confirm_required"`. |
| `token`      | string  | One-time confirmation token. Present the approval UI to the user, then POST back with this token. |
| `tool`       | string  | The tool name that requires approval. |
| `args`       | object  | The arguments the tool will be called with. |
| `prompt`     | string  | Human-readable description suitable for displaying in an approval dialog. |
| `expires_at` | integer | Unix timestamp (seconds). The token expires at this time; `confirm_response` with an expired token returns `confirm_expired`. |

### On request — `confirm_response`

To resolve a pending confirmation, the client appends a message with `bonsai_ext` to the messages array:

```json
{
  "role": "user",
  "content": "",
  "bonsai_ext": {
    "schema":   1,
    "type":     "confirm_response",
    "token":    "tok_a1b2c3",
    "approved": true
  }
}
```

| Field      | Type    | Description |
|------------|---------|-------------|
| `type`     | string  | Must be `"confirm_response"`. |
| `token`    | string  | The token from the `confirm_required` response. |
| `approved` | boolean | `true` = user approved; `false` = user denied. |

**Approved flow**: Server returns `confirm_ack`. Client should then resubmit the full conversation (without `bonsai_ext`) to trigger the approved tool call.

**Denied flow**: Server cancels the token and returns a standard `stop` response with content `"Confirmation denied. No action was taken."`.

### On response — `confirm_ack`

Receipt of a successful confirmation resolution:

```json
{
  "bonsai_ext": {
    "schema": 1,
    "type":   "confirm_ack",
    "token":  "tok_a1b2c3"
  }
}
```

---

## Error Responses

All errors use the envelope:
```json
{
  "error": {
    "type":    "<error_type>",
    "message": "<human-readable description>",
    "code":    <http_status_code>
  }
}
```

| HTTP Status | `type`              | When |
|-------------|---------------------|------|
| `400`       | `confirm_invalid`   | `confirm_response` has invalid schema or missing token |
| `400`       | `confirm_expired`   | Confirmation token has expired or was already consumed |
| `503`       | `buddy_error`       | No model slot ready, or inference error |

---

## Confirmation Flow — End-to-End

```
Client                              Server
  │                                   │
  │ POST /v1/chat/completions          │
  │ { messages: [...], stream: false } │
  │ ──────────────────────────────────►│
  │                                   │ (tool needs confirmation)
  │◄──────────────────────────────────│
  │ { finish_reason: "tool_calls_     │
  │   pending_approval",              │
  │   bonsai_ext: { type: "confirm_   │
  │   required", token: "tok_abc" } } │
  │                                   │
  │  [user sees approval UI]           │
  │                                   │
  │ POST /v1/chat/completions          │
  │ { messages: [...,                 │
  │     { role: "user", content: "",  │
  │       bonsai_ext: { type:         │
  │       "confirm_response",         │
  │       token: "tok_abc",           │
  │       approved: true } }] }       │
  │ ──────────────────────────────────►│
  │                                   │ (token consumed; returns ack)
  │◄──────────────────────────────────│
  │ { bonsai_ext: { type:             │
  │   "confirm_ack", token: "tok_abc" }│
  │                                   │
  │ POST /v1/chat/completions          │
  │ { messages: [...] }               │  ← resubmit without bonsai_ext
  │ ──────────────────────────────────►│
  │                                   │ (tool executes; normal reply)
  │◄──────────────────────────────────│
  │ { choices[0].finish_reason: "stop" }
```

---

## Security

- The server binds to `127.0.0.1` (loopback) only — it is **not accessible from the network**.
- No authentication is required because loopback-only access is the security boundary.
- CORS is configured via `tower_http::cors` — permitted origins should be restricted in production builds.
- The confirmation token is a one-time, expiring token. Replaying or guessing tokens returns `confirm_expired`.
