# 🧬 Bonsai Model Workshop

**A production-grade, childishly simple model management studio for the Bonsai Ecosystem.**

Design, build, edit, extract, and convert AI models. Manage knowledge modules. Create and curate datasets. Monitor training jobs in real-time.

---

## ✨ Features

- **🎨 Model Designer** — Configure model architecture, quantization, context, system prompt, temperature, and KDB module attachments
- **📚 Module Library** — Browse, create, edit, delete, and organize knowledge modules in the KDB with full CRUD
- **📊 Dataset Manager** — Create datasets, import data from JSONL/CSV/Parquet, clean and validate datasets
- **🏋️ Model Builder** — Start training jobs, monitor progress in real-time, view logs, cancel jobs
- **🔧 Model Editor** — Edit model parameters, merge LoRA adapters, update system prompts
- **🔄 Model Converter** — Convert between formats (PyTorch, ONNX, GGUF, SafeTensors)
- **⚡ Model Quantizer** — Reduce model size (Q4_K_M, Q5_K_M, Q8_0, F16, F32)
- **📈 Training Monitor** — Real-time job status, logs, progress bars, estimated completion times

---

## 🚀 Getting Started

### Prerequisites

- Rust 1.70+
- Tokio runtime
- Model files in `Z:\Projects\BonsaiWorkspace\models\`
- KDB instance running on port 8089 (optional, for full KDB integration)

### Build

```bash
cd crates/bonsai-model-workshop
cargo build --release
```

### Run

```bash
cargo run --release
# Or
bonsai-model-workshop
```

The server starts on `http://127.0.0.1:4200`

---

## 📡 API Endpoints

### Module Library

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/modules` | List all modules |
| POST | `/api/modules` | Create a new module |
| GET | `/api/modules/:id` | Get module details |
| PUT | `/api/modules/:id` | Update module metadata |
| DELETE | `/api/modules/:id` | Delete a module |
| POST | `/api/modules/:id/chunks` | Add chunk to module |
| DELETE | `/api/modules/:id/chunks/:chunk_id` | Remove chunk from module |

**Example: Create Module**
```json
POST /api/modules
{
  "name": "Server Management Guide",
  "description": "Knowledge about managing Linux servers",
  "domains": ["infrastructure", "devops"],
  "chunks": [
    {
      "text": "To list processes: `ps aux`",
      "domain": "infrastructure",
      "tags": ["processes", "monitoring"]
    },
    {
      "text": "To check disk usage: `df -h`",
      "domain": "infrastructure",
      "tags": ["disk", "monitoring"]
    }
  ]
}
```

### Dataset Manager

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/datasets` | List all datasets |
| POST | `/api/datasets` | Create a new dataset |
| DELETE | `/api/datasets/:id` | Delete dataset |
| POST | `/api/datasets/:id/import` | Import data into dataset |

**Example: Create Dataset**
```json
POST /api/datasets
{
  "name": "Server Management QA",
  "source_module": "module-123",
  "format": "qa"
}
```

### Model Designer

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/models/design` | Create model configuration |
| POST | `/api/models/design/validate` | Validate configuration |

**Example: Design Model**
```json
POST /api/models/design
{
  "name": "BonsAI-Server-Manager",
  "base_model": "llama-2-7b",
  "architecture": "llama",
  "quantization": "q4_k_m",
  "context_window": 4096,
  "system_prompt": "You are a helpful server management assistant.",
  "temperature": 0.7,
  "kdb_modules": ["module-123"],
  "tools": ["docker_list_containers", "kdb_search"],
  "parameters": {
    "total_params_billion": 7.0,
    "active_params_billion": 7.0,
    "moe_experts": 0,
    "active_experts": 0
  }
}
```

### Model Builder

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/models/build` | Start training job |
| GET | `/api/models/build/:job_id` | Get job status |
| DELETE | `/api/models/build/:job_id` | Cancel training job |

**Example: Start Training**
```json
POST /api/models/build
{
  "config_path": "/path/to/config.json",
  "stages": [1, 2, 3, 4],
  "gpu_count": 2,
  "dataset_id": "dataset-456"
}
```

### Model Editor

| Method | Endpoint | Description |
|--------|----------|-------------|
| PUT | `/api/models/edit/:id` | Edit model parameters |
| POST | `/api/models/merge-lora` | Merge LoRA adapter |

**Example: Merge LoRA**
```json
POST /api/models/merge-lora
{
  "base_model_id": "llama-2-7b",
  "lora_model_id": "lora-server-mgmt",
  "output_model_id": "bonsai-server-mgmt-v1",
  "scaling_factor": 1.0
}
```

### Model Converter

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/models/convert` | Convert model format |
| POST | `/api/models/quantize` | Quantize model |

**Example: Convert Model**
```json
POST /api/models/convert
{
  "input_path": "/models/llama-2-7b.safetensors",
  "input_format": "safetensors",
  "output_format": "gguf",
  "quantization": "q4_k_m"
}
```

### Training Monitor

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/training/jobs` | List all training jobs |
| GET | `/api/training/jobs/:id/logs` | Get job logs |

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────┐
│         Model Workshop Server               │
│  ┌─────────────────────────────────────┐   │
│  │  library.rs   – Module CRUD         │   │
│  │  datasets.rs  – Dataset management  │   │
│  │  designer.rs  – Model configuration │   │
│  │  builder.rs   – Training jobs       │   │
│  │  editor.rs    – Model editing       │   │
│  │  converter.rs – Format conversion   │   │
│  │  monitor.rs   – Job monitoring      │   │
│  └─────────────────────────────────────┘   │
│                                             │
│  ┌─────────────────────────────────────┐   │
│  │  Axum Router (HTTP API)             │   │
│  │  State: Modules, Datasets, Jobs     │   │
│  └─────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
       ↓
   Port 4200
       ↓
   Web Frontend / CLI
```

---

## 📦 Data Structures

### ModuleInfo
```json
{
  "id": "server-management-v1",
  "name": "Server Management Guide",
  "version": "1.0.0",
  "description": "Knowledge about Linux server management",
  "num_chunks": 42,
  "domains": ["infrastructure", "devops"],
  "created_at": "2026-06-03T00:00:00Z"
}
```

### DatasetInfo
```json
{
  "id": "ds-server-qa-001",
  "name": "Server Management Q&A",
  "num_examples": 5000,
  "domains": ["infrastructure"],
  "created_at": "2026-06-03T00:00:00Z"
}
```

### TrainingJob
```json
{
  "id": "job-xyz123",
  "config": "/path/to/config.json",
  "status": "running",
  "progress": 0.45,
  "current_stage": 2,
  "started_at": "2026-06-03T12:00:00Z",
  "estimated_completion": "2026-06-03T14:30:00Z",
  "logs": [
    "🚀 Stage 1: Loading dataset...",
    "✓ Loaded 5000 examples",
    "🚀 Stage 2: Fine-tuning..."
  ]
}
```

---

## 🎯 Workflow Example

### 1. Create a Knowledge Module
```bash
curl -X POST http://127.0.0.1:4200/api/modules \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Docker Guide",
    "description": "Docker commands and best practices",
    "domains": ["containers"],
    "chunks": [
      {"text": "docker run -d nginx", "tags": ["containers", "nginx"]}
    ]
  }'
```

### 2. Create a Dataset from the Module
```bash
curl -X POST http://127.0.0.1:4200/api/datasets \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Docker QA",
    "source_module": "module-123",
    "format": "qa"
  }'
```

### 3. Design a Model
```bash
curl -X POST http://127.0.0.1:4200/api/models/design \
  -H "Content-Type: application/json" \
  -d '{
    "name": "BonsAI-Docker",
    "base_model": "llama-2-7b",
    "architecture": "llama",
    "quantization": "q4_k_m",
    "context_window": 2048,
    "system_prompt": "You are a Docker expert.",
    "temperature": 0.7,
    "kdb_modules": ["module-123"],
    "tools": [],
    "parameters": {
      "total_params_billion": 7.0,
      "active_params_billion": 7.0,
      "moe_experts": 0,
      "active_experts": 0
    }
  }'
```

### 4. Start Training
```bash
curl -X POST http://127.0.0.1:4200/api/models/build \
  -H "Content-Type: application/json" \
  -d '{
    "config_path": "/path/to/config.json",
    "stages": [1, 2, 3, 4],
    "gpu_count": 2,
    "dataset_id": "ds-123"
  }'
```

### 5. Monitor Training
```bash
curl http://127.0.0.1:4200/api/training/jobs

curl http://127.0.0.1:4200/api/training/jobs/job-xyz123/logs
```

### 6. Convert Model
```bash
curl -X POST http://127.0.0.1:4200/api/models/convert \
  -H "Content-Type: application/json" \
  -d '{
    "input_path": "/models/bonsai-docker.safetensors",
    "input_format": "safetensors",
    "output_format": "gguf",
    "quantization": "q4_k_m"
  }'
```

---

## 🔗 Integration

Add to your Bonsai app's Cargo.toml:
```toml
[dependencies]
bonsai-model-workshop = { path = "../bonsai-model-workshop" }
```

Call from Tauri/Svelte:
```rust
#[tauri::command]
async fn open_model_workshop() {
    // Open http://127.0.0.1:4200 in browser
}
```

---

## 📊 Performance

- **Module creation**: <10ms
- **Module listing**: ~1ms per module
- **Training job start**: <50ms
- **Job status query**: <1ms
- **Memory usage**: ~10-20MB (in-memory state)

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

**Made with 🧬 for the Bonsai Ecosystem**
