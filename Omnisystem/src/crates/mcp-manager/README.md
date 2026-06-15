# 🔌 Bonsai MCP Manager

**Complete MCP server configuration, client management, and tool registry for the Bonsai Ecosystem.**

Manage MCP server settings, connected clients, external MCP servers, and enable/disable tools — all from one beautiful, intuitive interface.

---

## ✨ Features

- **⚙️ Server Configuration** — Set host, port, authentication mode, max clients, rate limiting
- **👥 Client Management** — View connected clients, revoke access, inspect logs per client
- **🔗 External MCP Servers** — Add, test, and manage connections to external MCP servers
- **🛠️ Tool Registry** — Enable/disable tools, view schemas, manage MCP tool exposure
- **📊 Live Status** — Real-time client connections, request counts, tool usage
- **🔐 Security** — Auth modes (token, certificate, none), rate limiting, client revocation

---

## 🚀 Getting Started

### Prerequisites

- Rust 1.70+
- Tokio runtime
- MCP Server instance (mcp-server)

### Build

```bash
cd crates/bonsai-mcp-manager
cargo build --release
```

### Run

```bash
cargo run --release
# Or
bonsai-mcp-manager
```

The server starts on `http://127.0.0.1:4201`

---

## 📡 API Endpoints

### Server Configuration

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/mcp/config` | Get current config |
| PUT | `/api/mcp/config` | Update configuration |

**Example: Get Config**
```bash
curl http://127.0.0.1:4201/api/mcp/config
```

**Response:**
```json
{
  "host": "127.0.0.1",
  "port": 7780,
  "auth_mode": "token",
  "max_clients": 100,
  "rate_limit_per_minute": 60
}
```

**Example: Update Config**
```bash
curl -X PUT http://127.0.0.1:4201/api/mcp/config \
  -H "Content-Type: application/json" \
  -d '{
    "host": "0.0.0.0",
    "port": 7780,
    "auth_mode": "certificate",
    "max_clients": 200,
    "rate_limit_per_minute": 120
  }'
```

### Connected Clients

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/mcp/clients` | List all connected clients |
| POST | `/api/mcp/clients/:id/revoke` | Revoke client access |
| GET | `/api/mcp/clients/:id/logs` | Get client activity logs |

**Example: List Clients**
```bash
curl http://127.0.0.1:4201/api/mcp/clients
```

**Response:**
```json
[
  {
    "client_id": "claude-desktop-001",
    "ip_address": "192.168.1.5",
    "connected_since": "2026-06-03T08:00:00Z",
    "tools_accessed": ["docker_list_containers", "kdb_search"],
    "status": "active",
    "request_count": 245
  },
  {
    "client_id": "copilot-web-002",
    "ip_address": "10.0.0.3",
    "connected_since": "2026-06-03T09:30:00Z",
    "tools_accessed": ["kdb_search"],
    "status": "active",
    "request_count": 52
  }
]
```

**Example: Revoke Client**
```bash
curl -X POST http://127.0.0.1:4201/api/mcp/clients/claude-desktop-001/revoke
```

**Example: Get Client Logs**
```bash
curl http://127.0.0.1:4201/api/mcp/clients/claude-desktop-001/logs
```

### External MCP Servers

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/mcp/servers` | List external servers |
| POST | `/api/mcp/servers` | Add external server |
| POST | `/api/mcp/servers/:name/test` | Test connection |
| DELETE | `/api/mcp/servers/:name` | Remove server |

**Example: List External Servers**
```bash
curl http://127.0.0.1:4201/api/mcp/servers
```

**Response:**
```json
[
  {
    "name": "Claude Desktop",
    "url": "https://api.anthropic.com",
    "status": "connected",
    "last_checked": "2026-06-03T10:00:00Z"
  },
  {
    "name": "OpenAI GPT API",
    "url": "https://api.openai.com/v1/mcp",
    "status": "connected",
    "last_checked": "2026-06-03T09:55:00Z"
  }
]
```

**Example: Add External Server**
```bash
curl -X POST http://127.0.0.1:4201/api/mcp/servers \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Custom Research MCP",
    "url": "ws://research-server:7780"
  }'
```

**Example: Test Connection**
```bash
curl -X POST http://127.0.0.1:4201/api/mcp/servers/Custom%20Research%20MCP/test
```

**Response:**
```json
{
  "server": "Custom Research MCP",
  "url": "ws://research-server:7780",
  "status": "connected",
  "latency_ms": 12,
  "message": "Connection successful"
}
```

**Example: Remove Server**
```bash
curl -X DELETE http://127.0.0.1:4201/api/mcp/servers/Custom%20Research%20MCP
```

### Tool Registry

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/mcp/tools` | List all tools |
| POST | `/api/mcp/tools/:name/enable` | Enable a tool |
| POST | `/api/mcp/tools/:name/disable` | Disable a tool |

**Example: List Tools**
```bash
curl http://127.0.0.1:4201/api/mcp/tools
```

**Response:**
```json
[
  {
    "name": "docker_list_containers",
    "description": "List all Docker containers",
    "category": "Infrastructure",
    "enabled": true,
    "schema": { ... }
  },
  {
    "name": "kdb_search",
    "description": "Search the Knowledge Database",
    "category": "Knowledge",
    "enabled": true,
    "schema": { ... }
  },
  {
    "name": "send_slack_message",
    "description": "Send a message to Slack",
    "category": "Communication",
    "enabled": false,
    "schema": { ... }
  }
]
```

**Example: Enable Tool**
```bash
curl -X POST http://127.0.0.1:4201/api/mcp/tools/send_slack_message/enable
```

**Example: Disable Tool**
```bash
curl -X POST http://127.0.0.1:4201/api/mcp/tools/send_slack_message/disable
```

---

## 🏗️ Architecture

```
┌──────────────────────────────────────────┐
│       MCP Manager Server                 │
│  ┌────────────────────────────────────┐ │
│  │  server_config.rs  – Config CRUD   │ │
│  │  clients.rs        – Client mgmt   │ │
│  │  external_servers  – Server mgmt   │ │
│  │  tools.rs          – Tool registry │ │
│  └────────────────────────────────────┘ │
│                                          │
│  ┌────────────────────────────────────┐ │
│  │  Axum Router (HTTP API)            │ │
│  │  State: Clients, Servers, Tools    │ │
│  └────────────────────────────────────┘ │
└──────────────────────────────────────────┘
        ↓
    Port 4201
        ↓
  Web Frontend / CLI
```

---

## 📦 Data Structures

### McpServerConfig
```json
{
  "host": "127.0.0.1",
  "port": 7780,
  "auth_mode": "token",
  "max_clients": 100,
  "rate_limit_per_minute": 60
}
```

### McpClient
```json
{
  "client_id": "claude-desktop-001",
  "ip_address": "192.168.1.5",
  "connected_since": "2026-06-03T08:00:00Z",
  "tools_accessed": ["docker_list_containers", "kdb_search"],
  "status": "active",
  "request_count": 245
}
```

### ExternalMcpServer
```json
{
  "name": "Claude Desktop",
  "url": "https://api.anthropic.com",
  "status": "connected",
  "last_checked": "2026-06-03T10:00:00Z"
}
```

### ToolEntry
```json
{
  "name": "docker_list_containers",
  "description": "List all Docker containers",
  "category": "Infrastructure",
  "enabled": true,
  "schema": {
    "type": "object",
    "properties": { ... }
  }
}
```

---

## 🎯 Common Workflows

### 1. Update Server Settings
```bash
curl -X PUT http://127.0.0.1:4201/api/mcp/config \
  -H "Content-Type: application/json" \
  -d '{
    "auth_mode": "certificate",
    "max_clients": 200,
    "rate_limit_per_minute": 120
  }'
```

### 2. Monitor Connected Clients
```bash
curl http://127.0.0.1:4201/api/mcp/clients
```

### 3. Revoke Problematic Client
```bash
curl -X POST http://127.0.0.1:4201/api/mcp/clients/malicious-001/revoke
curl http://127.0.0.1:4201/api/mcp/clients/malicious-001/logs
```

### 4. Add New External MCP Server
```bash
curl -X POST http://127.0.0.1:4201/api/mcp/servers \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Research Assistant",
    "url": "ws://research-mcp:7780"
  }'

# Test the connection
curl -X POST http://127.0.0.1:4201/api/mcp/servers/Research%20Assistant/test
```

### 5. Disable Dangerous Tool
```bash
curl -X POST http://127.0.0.1:4201/api/mcp/tools/delete_database/disable
```

### 6. Enable Tool for All Clients
```bash
curl -X POST http://127.0.0.1:4201/api/mcp/tools/kdb_search/enable
```

---

## 🔐 Security Best Practices

### Authentication Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| `token` | Bearer token in Authorization header | Development, internal tools |
| `certificate` | TLS client certificate | Production, regulated environments |
| `none` | No authentication | Localhost-only testing |

### Rate Limiting

Default: 60 requests per minute per client.

Adjust via `/api/mcp/config`:
```bash
curl -X PUT http://127.0.0.1:4201/api/mcp/config \
  -d '{"rate_limit_per_minute": 100}'
```

### Client Revocation

Immediately revoke a client:
```bash
curl -X POST http://127.0.0.1:4201/api/mcp/clients/<client_id>/revoke
```

All subsequent requests from that client are rejected.

---

## 📊 Monitoring

### View All Metrics
```bash
curl http://127.0.0.1:4201/api/mcp/clients | jq 'length'  # Total clients
curl http://127.0.0.1:4201/api/mcp/tools | jq '[.[] | select(.enabled)] | length'  # Active tools
```

### Sample Monitoring Dashboard
```bash
#!/bin/bash
while true; do
  echo "=== MCP Manager Status ==="
  echo "Clients: $(curl -s http://127.0.0.1:4201/api/mcp/clients | jq 'length')"
  echo "Tools: $(curl -s http://127.0.0.1:4201/api/mcp/tools | jq 'length')"
  echo "External Servers: $(curl -s http://127.0.0.1:4201/api/mcp/servers | jq 'length')"
  sleep 5
done
```

---

## 🔗 Integration

Add to your Bonsai app's Cargo.toml:
```toml
[dependencies]
bonsai-mcp-manager = { path = "../bonsai-mcp-manager" }
```

Call from Tauri/Svelte:
```rust
#[tauri::command]
async fn open_mcp_manager() {
    // Open http://127.0.0.1:4201 in browser
}
```

---

## 🛠️ Development

Run tests:
```bash
cargo test --release
```

Check code:
```bash
cargo clippy --release
```

Format code:
```bash
cargo fmt
```

---

## 📝 License

Same as Bonsai Ecosystem (MIT/Apache-2.0)

---

**Made with 🔌 for the Bonsai Ecosystem**
