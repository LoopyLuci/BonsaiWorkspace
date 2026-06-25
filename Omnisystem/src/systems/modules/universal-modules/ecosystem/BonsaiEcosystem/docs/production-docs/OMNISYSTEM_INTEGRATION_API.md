# Omnisystem Integration API - Complete Reference

**Version**: 1.0.0  
**Status**: Production Ready  
**Last Updated**: 2026-06-07

---

## Table of Contents

1. [Overview](#overview)
2. [Authentication](#authentication)
3. [API Reference](#api-reference)
   - [Services API](#services-api)
   - [Environments API](#environments-api)
   - [Modules API](#modules-api)
   - [Assets API](#assets-api)
   - [Validation API](#validation-api)
   - [Workflows API](#workflows-api)
   - [HDE Management](#hde-management)
4. [Error Handling](#error-handling)
5. [Getting Started](#getting-started)
6. [Advanced Topics](#advanced-topics)

---

## Overview

The Omnisystem Integration API provides programmatic access to a four-wave distributed system architecture:

- **Wave 1**: Background Services with kernel-level snapshotting
- **Wave 2**: Clojure Integration with persistent data structures
- **Wave 3**: Hybrid Determinism Engine (HDE) for AI-optional optimization
- **Wave 4**: Bonsai Buddy distributed agent system

### API Base URLs

```
OpenAI-compatible: http://localhost:3000/v1
Native API: http://localhost:3000/api
Services API: http://localhost:8080/services
Environments API: http://localhost:8080/environments
Modules API: http://localhost:8080/modules
HDE API: http://localhost:8080/hde
```

### Key Features

- **Demand-Activated Services**: Services spawn only when requested
- **Snapshotable State**: Full memory/register preservation
- **Persistent Data Structures**: O(log₃₂ n) operations
- **AI-Optional Optimization**: Safety-guaranteed enhancements
- **Offline-First**: Eventually-consistent CRDT synchronization
- **Hot-Reloadable**: Atomic service binary updates

---

## Authentication

### Bearer Token Authentication

All API endpoints (except `/health` and `/status`) require authentication via Bearer token:

```bash
curl -H "Authorization: Bearer YOUR_API_TOKEN" \
  http://localhost:3000/v1/chat/completions
```

### API Token Lifecycle

1. **Generate Token**
   ```bash
   curl -X POST http://localhost:8080/auth/tokens \
     -H "Content-Type: application/json" \
     -d '{
       "username": "user@example.com",
       "password": "secure_password",
       "expiration_days": 30
     }'
   ```

2. **Response**
   ```json
   {
     "token": "eyJhbGciOiJIUzI1NiIs...",
     "expires_at": "2026-07-07T10:30:00Z",
     "scopes": ["read", "write", "admin"]
   }
   ```

3. **Revoke Token**
   ```bash
   curl -X DELETE http://localhost:8080/auth/tokens/CURRENT_TOKEN \
     -H "Authorization: Bearer YOUR_API_TOKEN"
   ```

### Permission Scopes

| Scope | Description |
|-------|-------------|
| `read` | Read-only access to all resources |
| `write` | Create and modify resources |
| `admin` | Full administrative access |
| `services:spawn` | Permission to spawn new services |
| `services:pause` | Permission to pause services |
| `hde:optimize` | Permission to enable HDE optimization |
| `validation:matrix` | Permission to run validation matrices |

---

# API Reference

## Services API

### 1. List Services

Returns all active and paused services.

```
GET /services
```

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `state` | string | Filter by state: `running`, `paused`, `archived`, `failed` |
| `include_archived` | boolean | Include archived services (default: false) |
| `limit` | integer | Max results (1-1000, default: 100) |
| `offset` | integer | Pagination offset (default: 0) |

**Example Request:**
```bash
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/services?state=running&limit=50"
```

**Response (200 OK):**
```json
{
  "services": [
    {
      "instance_id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "fax-service",
      "version": "1.2.3",
      "state": "running",
      "vault_id": 42,
      "uptime_seconds": 3600,
      "last_access": "2026-06-07T10:30:45Z",
      "resource_usage": {
        "memory_mb": 256,
        "cpu_percent": 15,
        "iops_current": 5
      },
      "health_status": {
        "healthy": true,
        "message": "All systems operational"
      }
    }
  ],
  "total": 42,
  "limit": 50,
  "offset": 0
}
```

**Error Responses:**
- `400 Bad Request`: Invalid query parameters
- `401 Unauthorized`: Missing/invalid authentication token
- `403 Forbidden`: Insufficient permissions

---

### 2. Get Service Details

Get detailed information about a specific service instance.

```
GET /services/{instance_id}
```

**Path Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `instance_id` | string (UUID) | Service instance ID |

**Example Request:**
```bash
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/services/550e8400-e29b-41d4-a716-446655440000
```

**Response (200 OK):**
```json
{
  "instance_id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "fax-service",
  "version": "1.2.3",
  "state": "running",
  "vault_id": 42,
  "binary_hash": "blake3:abc123def456...",
  "capabilities_required": [
    "hardware:usb",
    "hardware:network",
    "filesystem:read",
    "filesystem:write"
  ],
  "resource_quota": {
    "memory_mb": 512,
    "cpu_cores": 1.0,
    "cpu_percent_max": 50,
    "iops_limit": 1000,
    "max_snapshots": 5,
    "max_snapshot_size_mb": 256
  },
  "resource_usage": {
    "memory_used_mb": 256,
    "cpu_percent": 15,
    "iops_current": 5,
    "measured_at": 1717761045
  },
  "latest_snapshot": {
    "hash": "blake3:snapshot123...",
    "size_bytes": 262144,
    "created_at": "2026-06-07T10:15:30Z",
    "archived": false
  },
  "snapshots": [
    {
      "hash": "blake3:snapshot123...",
      "size_bytes": 262144,
      "created_at": "2026-06-07T10:15:30Z",
      "archived": false
    }
  ],
  "health_status": {
    "healthy": true,
    "message": "All systems operational"
  },
  "uptime_seconds": 3600,
  "consecutive_failures": 0
}
```

---

### 3. Spawn Service

Create and start a new service instance.

```
POST /services/spawn
```

**Request Body:**
```json
{
  "manifest": {
    "name": "scanner-service",
    "version": "2.1.0",
    "binary_hash": "blake3:abc123def456...",
    "capabilities_required": ["hardware:scanner", "filesystem:write"],
    "resource_quota": {
      "memory_mb": 1024,
      "cpu_cores": 2.0,
      "cpu_percent_max": 75,
      "iops_limit": 5000,
      "max_snapshots": 10,
      "max_snapshot_size_mb": 512
    },
    "idle_timeout_secs": 600,
    "archive_after_hours": 48,
    "heartbeat_interval_secs": 30,
    "heartbeat_timeout_secs": 60,
    "signature": "council_signature_here"
  }
}
```

**Example Request:**
```bash
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d @service_manifest.json \
  http://localhost:8080/services/spawn
```

**Response (201 Created):**
```json
{
  "instance_id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "scanner-service",
  "version": "2.1.0",
  "state": "running",
  "vault_id": 42,
  "message": "Service spawned successfully"
}
```

**Error Responses:**
- `400 Bad Request`: Invalid manifest format
- `409 Conflict`: Service already running
- `507 Insufficient Storage`: Not enough memory/disk

---

### 4. Pause Service

Pause a running service and create a snapshot.

```
POST /services/{instance_id}/pause
```

**Request Body:**
```json
{
  "timeout_ms": 5000,
  "force": false
}
```

**Example Request:**
```bash
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"timeout_ms": 5000, "force": false}' \
  http://localhost:8080/services/550e8400-e29b-41d4-a716-446655440000/pause
```

**Response (200 OK):**
```json
{
  "instance_id": "550e8400-e29b-41d4-a716-446655440000",
  "state": "paused",
  "snapshot": {
    "hash": "blake3:snap_abc123...",
    "size_bytes": 262144,
    "created_at": "2026-06-07T10:30:45Z"
  },
  "message": "Service paused and snapshot created"
}
```

**Error Responses:**
- `400 Bad Request`: Timeout value out of range
- `409 Conflict`: Service not running
- `504 Gateway Timeout`: Snapshot operation timed out

---

### 5. Resume Service

Restore a paused or archived service from snapshot.

```
POST /services/{instance_id}/resume
```

**Request Body:**
```json
{
  "snapshot_hash": "blake3:snap_abc123...",
  "timeout_ms": 10000
}
```

**Example Request:**
```bash
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"timeout_ms": 10000}' \
  http://localhost:8080/services/550e8400-e29b-41d4-a716-446655440000/resume
```

**Response (200 OK):**
```json
{
  "instance_id": "550e8400-e29b-41d4-a716-446655440000",
  "state": "running",
  "vault_id": 43,
  "message": "Service resumed from snapshot"
}
```

---

### 6. Terminate Service

Permanently terminate a service.

```
DELETE /services/{instance_id}
```

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `force` | boolean | Force termination without cleanup (default: false) |

**Example Request:**
```bash
curl -X DELETE -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/services/550e8400-e29b-41d4-a716-446655440000?force=false
```

**Response (204 No Content):**
No body, status indicates success.

---

### 7. Service Health Check

Get current health status of a service.

```
GET /services/{instance_id}/health
```

**Example Request:**
```bash
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/services/550e8400-e29b-41d4-a716-446655440000/health
```

**Response (200 OK):**
```json
{
  "healthy": true,
  "memory_used_mb": 256,
  "cpu_percent": 15,
  "message": "Service operational",
  "checked_at": "2026-06-07T10:30:45Z"
}
```

---

### 8. Service Audit Log

Get audit events for a service.

```
GET /services/{instance_id}/audit
```

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `event_type` | string | Filter by event type: `spawned`, `paused`, `resumed`, `failed`, `recovered` |
| `limit` | integer | Max results (1-1000, default: 100) |
| `offset` | integer | Pagination offset (default: 0) |

**Example Request:**
```bash
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/services/550e8400-e29b-41d4-a716-446655440000/audit?event_type=paused&limit=50"
```

**Response (200 OK):**
```json
{
  "events": [
    {
      "id": "660e8400-e29b-41d4-a716-446655440001",
      "event_type": "paused",
      "service_name": "fax-service",
      "instance_id": "550e8400-e29b-41d4-a716-446655440000",
      "timestamp": "2026-06-07T10:15:30Z",
      "details": {
        "snapshot_hash": "blake3:snap_abc123...",
        "snapshot_size_bytes": 262144,
        "reason": "idle_timeout"
      }
    }
  ],
  "total": 15,
  "limit": 50,
  "offset": 0
}
```

---

## Environments API

### 1. List Environments

List all execution environments.

```
GET /environments
```

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `type` | string | Filter by type: `dev`, `staging`, `prod`, `test` |
| `limit` | integer | Max results (1-1000, default: 100) |

**Example Request:**
```bash
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/environments?type=prod"
```

**Response (200 OK):**
```json
{
  "environments": [
    {
      "id": "env_prod_001",
      "name": "Production",
      "type": "prod",
      "region": "us-east-1",
      "status": "active",
      "created_at": "2026-01-01T00:00:00Z",
      "spec": {
        "services": {
          "max_instances": 1000,
          "memory_quota_mb": 1048576,
          "auto_scaling": true
        },
        "persistent_storage": {
          "type": "cas_backend",
          "total_capacity_gb": 10240,
          "replication_factor": 3
        },
        "security": {
          "tls_enabled": true,
          "certificate_path": "/etc/certs/prod.pem",
          "encryption_key_id": "key_prod_001"
        }
      }
    }
  ],
  "total": 5,
  "limit": 100
}
```

---

### 2. Get Environment Details

Get detailed information about a specific environment.

```
GET /environments/{env_id}
```

**Example Request:**
```bash
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/environments/env_prod_001
```

**Response (200 OK):**
```json
{
  "id": "env_prod_001",
  "name": "Production",
  "type": "prod",
  "region": "us-east-1",
  "status": "active",
  "created_at": "2026-01-01T00:00:00Z",
  "updated_at": "2026-06-07T10:30:00Z",
  "spec": {
    "services": {
      "max_instances": 1000,
      "memory_quota_mb": 1048576,
      "cpu_quota_cores": 512,
      "auto_scaling": true,
      "scaling_min_instances": 10,
      "scaling_max_instances": 500,
      "scaling_target_cpu_percent": 70
    },
    "persistent_storage": {
      "type": "cas_backend",
      "total_capacity_gb": 10240,
      "replication_factor": 3,
      "backup_enabled": true,
      "backup_retention_days": 30
    },
    "networking": {
      "vpc_id": "vpc_prod_001",
      "subnets": ["subnet_a", "subnet_b", "subnet_c"],
      "security_groups": ["sg_prod_001"]
    },
    "security": {
      "tls_enabled": true,
      "certificate_path": "/etc/certs/prod.pem",
      "encryption_key_id": "key_prod_001",
      "audit_logging_enabled": true
    }
  },
  "metrics": {
    "active_services": 145,
    "memory_used_mb": 524288,
    "cpu_used_cores": 256,
    "storage_used_gb": 2048
  }
}
```

---

### 3. Create Environment

Create a new execution environment.

```
POST /environments
```

**Request Body:**
```json
{
  "name": "Staging Environment",
  "type": "staging",
  "region": "us-west-2",
  "spec": {
    "services": {
      "max_instances": 500,
      "memory_quota_mb": 524288,
      "auto_scaling": true,
      "scaling_min_instances": 5,
      "scaling_max_instances": 250
    },
    "persistent_storage": {
      "type": "cas_backend",
      "total_capacity_gb": 5120,
      "replication_factor": 2
    },
    "security": {
      "tls_enabled": true,
      "certificate_path": "/etc/certs/staging.pem"
    }
  }
}
```

**Example Request:**
```bash
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d @env_spec.json \
  http://localhost:8080/environments
```

**Response (201 Created):**
```json
{
  "id": "env_staging_002",
  "name": "Staging Environment",
  "type": "staging",
  "status": "provisioning",
  "message": "Environment provisioning initiated"
}
```

---

### 4. Update Environment

Update an existing environment's specification.

```
PUT /environments/{env_id}
```

**Request Body:**
```json
{
  "spec": {
    "services": {
      "memory_quota_mb": 1048576
    },
    "persistent_storage": {
      "total_capacity_gb": 20480
    }
  }
}
```

**Example Request:**
```bash
curl -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d @env_update.json \
  http://localhost:8080/environments/env_staging_002
```

**Response (200 OK):**
```json
{
  "id": "env_staging_002",
  "status": "active",
  "message": "Environment updated successfully"
}
```

---

### 5. Snapshot Environment

Create a point-in-time snapshot of an environment's state.

```
POST /environments/{env_id}/snapshot
```

**Request Body:**
```json
{
  "label": "pre-deployment",
  "include_service_snapshots": true,
  "description": "Snapshot before deployment"
}
```

**Example Request:**
```bash
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"label": "pre-deployment", "include_service_snapshots": true}' \
  http://localhost:8080/environments/env_prod_001/snapshot
```

**Response (201 Created):**
```json
{
  "snapshot_id": "snap_env_001",
  "env_id": "env_prod_001",
  "label": "pre-deployment",
  "timestamp": "2026-06-07T10:30:45Z",
  "state": "running",
  "service_count": 145,
  "total_size_bytes": 1099511627776,
  "status": "completed"
}
```

---

### 6. Restore Environment

Restore an environment from a snapshot.

```
POST /environments/{env_id}/restore
```

**Request Body:**
```json
{
  "snapshot_id": "snap_env_001",
  "timeout_ms": 300000
}
```

**Example Request:**
```bash
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"snapshot_id": "snap_env_001", "timeout_ms": 300000}' \
  http://localhost:8080/environments/env_prod_001/restore
```

**Response (200 OK):**
```json
{
  "env_id": "env_prod_001",
  "snapshot_id": "snap_env_001",
  "status": "restored",
  "services_restored": 145,
  "message": "Environment restored from snapshot"
}
```

---

### 7. Environment Migrations

Migrate resources between environments.

```
POST /environments/migrate
```

**Request Body:**
```json
{
  "source_env_id": "env_staging_002",
  "target_env_id": "env_prod_001",
  "resources": {
    "service_ids": ["550e8400-e29b-41d4-a716-446655440000"],
    "include_snapshots": true,
    "validation_mode": "dry_run"
  },
  "timeout_ms": 600000
}
```

**Example Request:**
```bash
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d @migration_spec.json \
  http://localhost:8080/environments/migrate
```

**Response (202 Accepted):**
```json
{
  "migration_id": "migration_001",
  "source_env_id": "env_staging_002",
  "target_env_id": "env_prod_001",
  "status": "in_progress",
  "progress": {
    "total_resources": 1,
    "completed": 0,
    "failed": 0
  }
}
```

---

## Modules API

### 1. Search Modules

Search for available modules in the registry.

```
GET /modules/search
```

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `query` | string | Search term (name, description) |
| `category` | string | Filter by category: `service`, `library`, `utility`, `extension` |
| `language` | string | Filter by language: `rust`, `clojure`, `python`, etc. |
| `version` | string | Exact version or semver range |
| `limit` | integer | Max results (1-1000, default: 50) |

**Example Request:**
```bash
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/modules/search?query=fax&category=service&limit=25"
```

**Response (200 OK):**
```json
{
  "modules": [
    {
      "id": "mod_fax_001",
      "name": "fax-service",
      "version": "1.2.3",
      "category": "service",
      "language": "rust",
      "description": "High-performance fax processing service",
      "binary_hash": "blake3:abc123def456...",
      "author": "bonsai-team",
      "published_at": "2026-04-15T10:00:00Z",
      "downloads": 15240,
      "signatures": ["bonsai-council"],
      "dependencies": [
        {
          "module_id": "mod_pdf_001",
          "version": "^2.0.0"
        }
      ]
    }
  ],
  "total": 42,
  "limit": 25
}
```

---

### 2. Get Module Details

Get detailed information about a module.

```
GET /modules/{module_id}
```

**Example Request:**
```bash
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/modules/mod_fax_001
```

**Response (200 OK):**
```json
{
  "id": "mod_fax_001",
  "name": "fax-service",
  "version": "1.2.3",
  "category": "service",
  "language": "rust",
  "description": "High-performance fax processing service with OCR",
  "long_description": "...",
  "binary_hash": "blake3:abc123def456...",
  "binary_size_bytes": 45678901,
  "author": "bonsai-team",
  "published_at": "2026-04-15T10:00:00Z",
  "downloads": 15240,
  "signatures": ["bonsai-council"],
  "signature_verification": "valid",
  "dependencies": [
    {
      "module_id": "mod_pdf_001",
      "name": "pdf-processor",
      "version": "^2.0.0",
      "required": true
    },
    {
      "module_id": "mod_ocr_001",
      "name": "ocr-engine",
      "version": "^1.5.0",
      "required": false
    }
  ],
  "capabilities": ["hardware:scanner", "filesystem:write"],
  "resource_requirements": {
    "memory_mb": 512,
    "cpu_cores": 1.0,
    "disk_mb": 256
  },
  "api": {
    "endpoints": [
      {
        "method": "POST",
        "path": "/scan",
        "description": "Submit fax for processing"
      },
      {
        "method": "GET",
        "path": "/status/{job_id}",
        "description": "Get job status"
      }
    ]
  }
}
```

---

### 3. List Module Versions

List all versions of a module.

```
GET /modules/{module_name}/versions
```

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `limit` | integer | Max results (default: 50) |

**Example Request:**
```bash
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/modules/fax-service/versions?limit=20"
```

**Response (200 OK):**
```json
{
  "name": "fax-service",
  "versions": [
    {
      "version": "1.2.3",
      "binary_hash": "blake3:abc123...",
      "published_at": "2026-06-05T00:00:00Z",
      "deprecated": false,
      "prerelease": false
    },
    {
      "version": "1.2.2",
      "binary_hash": "blake3:def456...",
      "published_at": "2026-05-20T00:00:00Z",
      "deprecated": false,
      "prerelease": false
    }
  ],
  "total": 15
}
```

---

### 4. Resolve Module Dependencies

Resolve all transitive dependencies for a module.

```
POST /modules/resolve-dependencies
```

**Request Body:**
```json
{
  "modules": [
    {
      "id": "mod_fax_001",
      "version": "1.2.3"
    }
  ]
}
```

**Example Request:**
```bash
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"modules": [{"id": "mod_fax_001", "version": "1.2.3"}]}' \
  http://localhost:8080/modules/resolve-dependencies
```

**Response (200 OK):**
```json
{
  "resolved": [
    {
      "id": "mod_fax_001",
      "name": "fax-service",
      "version": "1.2.3",
      "depth": 0
    },
    {
      "id": "mod_pdf_001",
      "name": "pdf-processor",
      "version": "2.1.0",
      "depth": 1
    },
    {
      "id": "mod_ocr_001",
      "name": "ocr-engine",
      "version": "1.5.2",
      "depth": 1
    }
  ],
  "conflict_free": true,
  "total_size_bytes": 127890123
}
```

---

### 5. Verify Module Signature

Verify the cryptographic signature of a module.

```
POST /modules/{module_id}/verify-signature
```

**Request Body:**
```json
{
  "version": "1.2.3",
  "signature": "council_signature_data"
}
```

**Example Request:**
```bash
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"version": "1.2.3", "signature": "signature_data"}' \
  http://localhost:8080/modules/mod_fax_001/verify-signature
```

**Response (200 OK):**
```json
{
  "valid": true,
  "signer": "bonsai-council",
  "timestamp": "2026-06-05T00:00:00Z",
  "message": "Signature verified successfully"
}
```

---

## Assets API

### 1. Generate Assets

Generate deployment artifacts for a service.

```
POST /assets/generate
```

**Request Body:**
```json
{
  "module_id": "mod_fax_001",
  "version": "1.2.3",
  "environment": "prod",
  "format": "container",
  "include_dependencies": true,
  "optimization_level": "aggressive"
}
```

**Example Request:**
```bash
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d @asset_request.json \
  http://localhost:8080/assets/generate
```

**Response (202 Accepted):**
```json
{
  "job_id": "job_asset_001",
  "status": "queued",
  "estimated_duration_seconds": 120,
  "progress": {
    "stage": "queued",
    "percentage": 0
  }
}
```

---

### 2. Get Asset Generation Status

Get status of an asset generation job.

```
GET /assets/jobs/{job_id}
```

**Example Request:**
```bash
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/assets/jobs/job_asset_001
```

**Response (200 OK):**
```json
{
  "job_id": "job_asset_001",
  "status": "in_progress",
  "created_at": "2026-06-07T10:00:00Z",
  "progress": {
    "stage": "compiling",
    "percentage": 45,
    "message": "Compiling module dependencies"
  },
  "estimated_completion": "2026-06-07T10:02:00Z"
}
```

---

### 3. Stream Asset Generation Progress

Stream progress updates as assets are generated.

```
GET /assets/jobs/{job_id}/stream
```

**Example Request:**
```bash
curl -H "Authorization: Bearer $TOKEN" \
  --stream http://localhost:8080/assets/jobs/job_asset_001/stream
```

**Streaming Response (text/event-stream):**
```
data: {"stage": "compiling", "percentage": 45, "message": "Compiling module dependencies"}

data: {"stage": "linking", "percentage": 67, "message": "Linking object files"}

data: {"stage": "packaging", "percentage": 89, "message": "Creating container image"}

data: {"status": "completed", "percentage": 100, "artifact_id": "artifact_001"}
```

---

### 4. Download Generated Asset

Download a completed asset.

```
GET /assets/{artifact_id}/download
```

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `format` | string | Output format: `tar.gz`, `docker`, `oci` |

**Example Request:**
```bash
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/assets/artifact_001/download?format=docker" \
  -o fax-service-1.2.3.tar
```

---

### 5. Batch Generate Assets

Generate multiple assets in a single request.

```
POST /assets/generate/batch
```

**Request Body:**
```json
{
  "jobs": [
    {
      "module_id": "mod_fax_001",
      "version": "1.2.3",
      "environment": "prod"
    },
    {
      "module_id": "mod_scanner_001",
      "version": "2.0.0",
      "environment": "prod"
    }
  ],
  "parallel": true,
  "max_concurrent": 4
}
```

**Example Request:**
```bash
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d @batch_request.json \
  http://localhost:8080/assets/generate/batch
```

**Response (202 Accepted):**
```json
{
  "batch_id": "batch_asset_001",
  "total_jobs": 2,
  "job_ids": ["job_asset_002", "job_asset_003"],
  "status": "queued"
}
```

---

## Validation API

### 1. Create Validation Matrix

Define and execute a validation matrix for comprehensive testing.

```
POST /validation/matrix
```

**Request Body:**
```json
{
  "name": "fax-service-validation",
  "module_id": "mod_fax_001",
  "version": "1.2.3",
  "matrix": {
    "environments": ["dev", "staging", "prod"],
    "configurations": [
      {
        "name": "standard",
        "resource_quota": {
          "memory_mb": 512,
          "cpu_cores": 1.0
        }
      },
      {
        "name": "high-performance",
        "resource_quota": {
          "memory_mb": 2048,
          "cpu_cores": 4.0
        }
      }
    ],
    "test_suites": ["unit", "integration", "performance"],
    "test_count": 750
  }
}
```

**Example Request:**
```bash
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d @validation_matrix.json \
  http://localhost:8080/validation/matrix
```

**Response (201 Created):**
```json
{
  "matrix_id": "matrix_001",
  "name": "fax-service-validation",
  "status": "running",
  "total_tests": 750,
  "completed": 0,
  "passed": 0,
  "failed": 0,
  "estimated_duration_seconds": 3600
}
```

---

### 2. Get Validation Results

Get results from a validation matrix run.

```
GET /validation/matrix/{matrix_id}
```

**Example Request:**
```bash
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/validation/matrix/matrix_001
```

**Response (200 OK):**
```json
{
  "matrix_id": "matrix_001",
  "status": "completed",
  "total_tests": 750,
  "completed": 750,
  "passed": 748,
  "failed": 2,
  "skipped": 0,
  "pass_rate": 0.997,
  "started_at": "2026-06-07T10:00:00Z",
  "completed_at": "2026-06-07T11:00:00Z",
  "duration_seconds": 3600,
  "results_by_environment": {
    "dev": {
      "total": 250,
      "passed": 250,
      "failed": 0
    },
    "staging": {
      "total": 250,
      "passed": 250,
      "failed": 0
    },
    "prod": {
      "total": 250,
      "passed": 248,
      "failed": 2
    }
  },
  "failed_tests": [
    {
      "test_id": "test_001",
      "name": "high_load_stress_test",
      "environment": "prod",
      "error": "Service timeout at 95% CPU utilization",
      "stack_trace": "..."
    }
  ]
}
```

---

### 3. Stream Validation Progress

Stream validation progress updates.

```
GET /validation/matrix/{matrix_id}/stream
```

**Streaming Response (text/event-stream):**
```
data: {"status": "running", "completed": 100, "passed": 100, "failed": 0, "percentage": 13}

data: {"status": "running", "completed": 250, "passed": 248, "failed": 2, "percentage": 33}

data: {"status": "completed", "completed": 750, "passed": 748, "failed": 2, "percentage": 100}
```

---

### 4. Deterministic Replay

Replay a specific test execution with identical conditions.

```
POST /validation/replay
```

**Request Body:**
```json
{
  "test_id": "test_001",
  "matrix_id": "matrix_001",
  "environment": "prod",
  "configuration": "high-performance",
  "seed": 12345,
  "record_execution": true
}
```

**Example Request:**
```bash
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d @replay_request.json \
  http://localhost:8080/validation/replay
```

**Response (202 Accepted):**
```json
{
  "replay_id": "replay_001",
  "test_id": "test_001",
  "status": "running",
  "seed": 12345
}
```

---

## Workflows API

### 1. Create Workflow

Define a workflow DAG for orchestrating complex operations.

```
POST /workflows
```

**Request Body:**
```json
{
  "name": "deploy-pipeline",
  "description": "Deploy service and run validation",
  "dag": {
    "nodes": [
      {
        "id": "build",
        "type": "asset_generate",
        "config": {
          "module_id": "mod_fax_001",
          "version": "1.2.3"
        }
      },
      {
        "id": "validate",
        "type": "validation_matrix",
        "depends_on": ["build"],
        "config": {
          "module_id": "mod_fax_001",
          "test_count": 100
        }
      },
      {
        "id": "deploy",
        "type": "service_spawn",
        "depends_on": ["validate"],
        "config": {
          "manifest": {
            "name": "fax-service",
            "version": "1.2.3"
          }
        }
      }
    ]
  },
  "parameters": {
    "environment": {
      "type": "string",
      "default": "staging"
    },
    "test_count": {
      "type": "integer",
      "default": 100
    }
  }
}
```

**Example Request:**
```bash
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d @workflow.json \
  http://localhost:8080/workflows
```

**Response (201 Created):**
```json
{
  "workflow_id": "wf_001",
  "name": "deploy-pipeline",
  "status": "created",
  "version": 1
}
```

---

### 2. Execute Workflow

Execute a workflow with provided parameters.

```
POST /workflows/{workflow_id}/execute
```

**Request Body:**
```json
{
  "parameters": {
    "environment": "prod",
    "test_count": 500
  },
  "timeout_seconds": 7200,
  "notifications": {
    "email": "ops@example.com",
    "on_events": ["completed", "failed"]
  }
}
```

**Example Request:**
```bash
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d @execution_params.json \
  http://localhost:8080/workflows/wf_001/execute
```

**Response (202 Accepted):**
```json
{
  "execution_id": "exec_001",
  "workflow_id": "wf_001",
  "status": "running",
  "started_at": "2026-06-07T10:00:00Z",
  "estimated_completion": "2026-06-07T12:00:00Z"
}
```

---

### 3. Get Workflow Execution Status

Get detailed execution status.

```
GET /workflows/{workflow_id}/executions/{execution_id}
```

**Example Request:**
```bash
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/workflows/wf_001/executions/exec_001
```

**Response (200 OK):**
```json
{
  "execution_id": "exec_001",
  "workflow_id": "wf_001",
  "status": "completed",
  "started_at": "2026-06-07T10:00:00Z",
  "completed_at": "2026-06-07T11:30:00Z",
  "duration_seconds": 5400,
  "nodes": [
    {
      "node_id": "build",
      "status": "completed",
      "started_at": "2026-06-07T10:00:00Z",
      "completed_at": "2026-06-07T10:15:00Z",
      "output": {
        "artifact_id": "artifact_001"
      }
    },
    {
      "node_id": "validate",
      "status": "completed",
      "started_at": "2026-06-07T10:15:00Z",
      "completed_at": "2026-06-07T11:15:00Z",
      "output": {
        "matrix_id": "matrix_001",
        "pass_rate": 0.997
      }
    },
    {
      "node_id": "deploy",
      "status": "completed",
      "started_at": "2026-06-07T11:15:00Z",
      "completed_at": "2026-06-07T11:30:00Z",
      "output": {
        "instance_id": "550e8400-e29b-41d4-a716-446655440000"
      }
    }
  ],
  "overall_status": "success"
}
```

---

## HDE Management

### 1. Enable HDE Optimization

Enable Hybrid Determinism Engine optimization for a service.

```
POST /hde/enable
```

**Request Body:**
```json
{
  "instance_id": "550e8400-e29b-41d4-a716-446655440000",
  "safety_level": "strict",
  "latency_constraint_ms": 100,
  "memory_constraint_mb": 512,
  "models": ["speed", "efficiency"]
}
```

**Example Request:**
```bash
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d @hde_enable.json \
  http://localhost:8080/hde/enable
```

**Response (200 OK):**
```json
{
  "instance_id": "550e8400-e29b-41d4-a716-446655440000",
  "hde_enabled": true,
  "status": "operational",
  "models_loaded": ["speed", "efficiency"],
  "safety_envelope": {
    "latency_constraint_ms": 100,
    "memory_constraint_mb": 512,
    "enforcement": "strict"
  }
}
```

---

### 2. Get HDE Status

Get current HDE status for a service.

```
GET /hde/status/{instance_id}
```

**Example Request:**
```bash
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/hde/status/550e8400-e29b-41d4-a716-446655440000
```

**Response (200 OK):**
```json
{
  "instance_id": "550e8400-e29b-41d4-a716-446655440000",
  "hde_enabled": true,
  "status": "operational",
  "shadow_mode": false,
  "safety_envelope": {
    "latency_constraint_ms": 100,
    "memory_constraint_mb": 512,
    "enforcement": "strict",
    "violations": 0
  },
  "model_status": {
    "speed": {
      "loaded": true,
      "accuracy": 0.987,
      "last_updated": "2026-06-01T00:00:00Z"
    },
    "efficiency": {
      "loaded": true,
      "accuracy": 0.945,
      "last_updated": "2026-06-05T00:00:00Z"
    }
  },
  "metrics": {
    "optimizations_applied": 1245,
    "optimizations_bypassed": 3,
    "total_latency_ms": 850,
    "average_speedup": 1.23
  }
}
```

---

### 3. Enable Shadow Mode

Enable shadow mode for safe testing of optimizations.

```
POST /hde/{instance_id}/shadow-mode
```

**Request Body:**
```json
{
  "enabled": true,
  "compare_threshold": 0.01,
  "record_comparisons": true
}
```

**Example Request:**
```bash
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"enabled": true, "compare_threshold": 0.01, "record_comparisons": true}' \
  http://localhost:8080/hde/550e8400-e29b-41d4-a716-446655440000/shadow-mode
```

**Response (200 OK):**
```json
{
  "instance_id": "550e8400-e29b-41d4-a716-446655440000",
  "shadow_mode": true,
  "status": "active",
  "message": "Shadow mode enabled. Optimizations will be validated before commitment."
}
```

---

### 4. Get Shadow Mode Comparisons

Get comparison data from shadow mode executions.

```
GET /hde/{instance_id}/shadow-comparisons
```

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `limit` | integer | Max results (default: 100) |
| `variance_filter` | float | Show only comparisons with variance > threshold |

**Example Request:**
```bash
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/hde/550e8400-e29b-41d4-a716-446655440000/shadow-comparisons?limit=50"
```

**Response (200 OK):**
```json
{
  "instance_id": "550e8400-e29b-41d4-a716-446655440000",
  "comparisons": [
    {
      "execution_id": "exec_001",
      "baseline_result": 123.45,
      "optimized_result": 124.12,
      "variance_percent": 0.54,
      "status": "validated",
      "timestamp": "2026-06-07T10:00:00Z"
    }
  ],
  "total_comparisons": 1245,
  "validation_pass_rate": 0.998
}
```

---

### 5. Update HDE Model

Update or replace an HDE optimization model.

```
PUT /hde/models/{model_name}
```

**Request Body:**
```json
{
  "model_data": "base64_encoded_model_binary",
  "version": "2.1.0",
  "accuracy": 0.992,
  "rollback_on_failure": true
}
```

**Example Request:**
```bash
curl -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d @model_update.json \
  http://localhost:8080/hde/models/speed
```

**Response (200 OK):**
```json
{
  "model_name": "speed",
  "version": "2.1.0",
  "status": "active",
  "deployed_at": "2026-06-07T10:30:45Z",
  "instances_updated": 142
}
```

---

# Error Handling

All API responses use standard HTTP status codes and include detailed error information.

## Error Response Format

```json
{
  "error": {
    "code": "SERVICE_NOT_FOUND",
    "message": "Service instance not found",
    "details": {
      "instance_id": "550e8400-e29b-41d4-a716-446655440000"
    },
    "trace_id": "trace_abc123def456"
  }
}
```

## Status Codes

| Code | Meaning | Example |
|------|---------|---------|
| `200` | Success | Request completed successfully |
| `201` | Created | Resource created successfully |
| `202` | Accepted | Async operation accepted |
| `204` | No Content | Successful delete operation |
| `400` | Bad Request | Invalid request parameters |
| `401` | Unauthorized | Missing/invalid authentication |
| `403` | Forbidden | Insufficient permissions |
| `404` | Not Found | Resource doesn't exist |
| `409` | Conflict | Service already running |
| `429` | Rate Limited | Too many requests |
| `500` | Server Error | Internal server error |
| `503` | Unavailable | Service temporarily unavailable |
| `504` | Timeout | Operation exceeded timeout |
| `507` | Insufficient Storage | Not enough disk/memory |

## Common Error Codes

| Code | HTTP | Description |
|------|------|-------------|
| `AUTH_INVALID_TOKEN` | 401 | Token is invalid or expired |
| `AUTH_INSUFFICIENT_SCOPES` | 403 | Token lacks required scopes |
| `SERVICE_NOT_FOUND` | 404 | Service instance not found |
| `SERVICE_NOT_RUNNING` | 409 | Service is not in running state |
| `SERVICE_ALREADY_RUNNING` | 409 | Service already running |
| `VAULT_CREATION_FAILED` | 500 | Failed to create kernel vault |
| `SNAPSHOT_FAILED` | 500 | Snapshot operation failed |
| `RESOURCE_QUOTA_EXCEEDED` | 507 | Insufficient resources |
| `VALIDATION_FAILED` | 400 | Invalid manifest or configuration |
| `NETWORK_TIMEOUT` | 504 | Operation exceeded timeout |

---

# Getting Started

## 1. Initial Setup

### Install CLI Tools

```bash
# Using cargo
cargo install bonsai-cli

# Or download binary
curl https://releases.bonsai.io/cli/latest -o bonsai
chmod +x bonsai
```

### Generate API Token

```bash
bonsai auth login
# Follow interactive prompt to authenticate
```

### Verify Installation

```bash
curl -H "Authorization: Bearer $BONSAI_TOKEN" \
  http://localhost:8080/services | jq '.total'
```

---

## 2. Quick Start Example: Deploy a Service

### Step 1: Find Available Module

```bash
curl -H "Authorization: Bearer $BONSAI_TOKEN" \
  "http://localhost:8080/modules/search?query=fax" | jq '.'
```

### Step 2: Spawn Service Instance

```bash
curl -X POST -H "Authorization: Bearer $BONSAI_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "manifest": {
      "name": "fax-service",
      "version": "1.2.3",
      "binary_hash": "blake3:abc123...",
      "capabilities_required": ["hardware:usb"],
      "resource_quota": {
        "memory_mb": 512,
        "cpu_cores": 1.0,
        "cpu_percent_max": 50,
        "iops_limit": 1000,
        "max_snapshots": 5,
        "max_snapshot_size_mb": 256
      },
      "idle_timeout_secs": 300,
      "archive_after_hours": 24,
      "heartbeat_interval_secs": 30,
      "heartbeat_timeout_secs": 60,
      "signature": "council_signature"
    }
  }' \
  http://localhost:8080/services/spawn
```

### Step 3: Check Service Status

```bash
SERVICE_ID="550e8400-e29b-41d4-a716-446655440000"
curl -H "Authorization: Bearer $BONSAI_TOKEN" \
  http://localhost:8080/services/$SERVICE_ID/health
```

### Step 4: Pause and Snapshot

```bash
curl -X POST -H "Authorization: Bearer $BONSAI_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"timeout_ms": 5000}' \
  http://localhost:8080/services/$SERVICE_ID/pause
```

---

## 3. Authentication Setup Script

```bash
#!/bin/bash

# Set up environment
export BONSAI_API_HOST="http://localhost:8080"
export BONSAI_API_PORT="8080"

# Generate token
TOKEN=$(curl -X POST "$BONSAI_API_HOST/auth/tokens" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "password",
    "expiration_days": 30
  }' | jq -r '.token')

export BONSAI_TOKEN=$TOKEN

# Verify
curl -H "Authorization: Bearer $BONSAI_TOKEN" \
  "$BONSAI_API_HOST/services" | jq '.total'
```

---

## 4. Common Workflow Patterns

### Pattern 1: Deploy, Validate, and Monitor

```bash
#!/bin/bash

# 1. Create validation matrix
MATRIX=$(curl -X POST -H "Authorization: Bearer $BONSAI_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "deploy-validation",
    "module_id": "mod_fax_001",
    "matrix": {
      "environments": ["staging", "prod"],
      "test_suites": ["unit", "integration"],
      "test_count": 500
    }
  }' http://localhost:8080/validation/matrix)

MATRIX_ID=$(echo $MATRIX | jq -r '.matrix_id')
echo "Matrix ID: $MATRIX_ID"

# 2. Wait for completion
while true; do
  RESULT=$(curl -H "Authorization: Bearer $BONSAI_TOKEN" \
    http://localhost:8080/validation/matrix/$MATRIX_ID)
  STATUS=$(echo $RESULT | jq -r '.status')
  PASS_RATE=$(echo $RESULT | jq -r '.pass_rate // 0')
  
  echo "Status: $STATUS, Pass Rate: $PASS_RATE"
  
  if [ "$STATUS" = "completed" ]; then
    break
  fi
  
  sleep 10
done

# 3. Deploy if validation passed
if [ $(echo "$PASS_RATE > 0.95" | bc) -eq 1 ]; then
  curl -X POST -H "Authorization: Bearer $BONSAI_TOKEN" \
    http://localhost:8080/workflows/wf_001/execute
fi
```

### Pattern 2: Environment Migration

```bash
#!/bin/bash

# Migrate service from staging to production
curl -X POST -H "Authorization: Bearer $BONSAI_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_env_id": "env_staging_001",
    "target_env_id": "env_prod_001",
    "resources": {
      "service_ids": ["550e8400-e29b-41d4-a716-446655440000"],
      "include_snapshots": true,
      "validation_mode": "dry_run"
    },
    "timeout_ms": 600000
  }' \
  http://localhost:8080/environments/migrate
```

---

# Advanced Topics

## 1. Capability System Deep Dive

### Capability-Based Security Model

The Omnisystem uses a capability-based security model where every service request must carry valid capabilities.

**Capability Structure:**
```json
{
  "capability_token": "cap_abc123...",
  "grant_id": "grant_001",
  "service_id": "550e8400-e29b-41d4-a716-446655440000",
  "capabilities": [
    {
      "resource": "hardware:usb",
      "permission": "read-write",
      "device_id": "device_123",
      "expires_at": "2026-06-14T10:30:00Z"
    }
  ]
}
```

### Request with Capabilities

```bash
curl -H "Authorization: Bearer $BONSAI_TOKEN" \
  -H "X-Capabilities: cap_abc123..." \
  -H "Content-Type: application/json" \
  -d '{"command": "scan"}' \
  http://localhost:3000/api/scanner/execute
```

---

## 2. Offline Queue Operation

The Bonsai Buddy agent maintains an offline-first operation queue.

### Queued Operations

```json
{
  "queue_id": "queue_001",
  "instance_id": "buddy_001",
  "operations": [
    {
      "operation_id": "op_001",
      "type": "service_spawn",
      "target": "fax-service",
      "payload": {},
      "status": "pending",
      "enqueued_at": "2026-06-07T10:00:00Z",
      "retry_count": 0
    }
  ],
  "sync_status": "waiting"
}
```

### Sync When Online

```bash
curl -X POST -H "Authorization: Bearer $BONSAI_TOKEN" \
  http://localhost:8080/buddy/sync \
  -H "Content-Type: application/json" \
  -d '{"queue_id": "queue_001"}'
```

---

## 3. CRDT Synchronization

The Bonsai Buddy system uses CRDTs (Conflict-free Replicated Data Types) for distributed state merging.

### Snapshot Merge Operation

```json
{
  "local_snapshot": {
    "vector_clock": {"node_a": 10, "node_b": 5},
    "state": {...}
  },
  "remote_snapshot": {
    "vector_clock": {"node_a": 8, "node_b": 12},
    "state": {...}
  }
}
```

### Merge Request

```bash
curl -X POST -H "Authorization: Bearer $BONSAI_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"local": {...}, "remote": {...}}' \
  http://localhost:8080/crdt/merge
```

---

## 4. WebSocket Subscriptions

Subscribe to real-time events via WebSocket.

### Connect to WebSocket

```bash
wscat -c "ws://localhost:8080/ws?token=$BONSAI_TOKEN"
```

### Subscribe to Events

```json
{
  "action": "subscribe",
  "channel": "services:550e8400-e29b-41d4-a716-446655440000:status"
}
```

### Receive Events

```json
{
  "type": "service_status_changed",
  "instance_id": "550e8400-e29b-41d4-a716-446655440000",
  "new_state": "paused",
  "timestamp": "2026-06-07T10:30:45Z"
}
```

---

## 5. Error Recovery Patterns

### Circuit Breaker Pattern

```rust
pub struct CircuitBreaker {
    failure_threshold: u32,
    consecutive_failures: u32,
    state: CircuitState,
}

impl CircuitBreaker {
    pub async fn execute<F>(&mut self, f: F) -> Result<T>
    where
        F: Fn() -> Result<T>,
    {
        match self.state {
            CircuitState::Closed => {
                match f() {
                    Ok(result) => {
                        self.consecutive_failures = 0;
                        Ok(result)
                    }
                    Err(e) => {
                        self.consecutive_failures += 1;
                        if self.consecutive_failures >= self.failure_threshold {
                            self.state = CircuitState::Open;
                        }
                        Err(e)
                    }
                }
            }
            CircuitState::Open => {
                Err(Error::CircuitBreakerOpen)
            }
            CircuitState::HalfOpen => {
                // Try one request
                f()
            }
        }
    }
}
```

### Automatic Retry with Exponential Backoff

```bash
#!/bin/bash

max_retries=5
retry_count=0
backoff=1

while [ $retry_count -lt $max_retries ]; do
  curl -H "Authorization: Bearer $BONSAI_TOKEN" \
    http://localhost:8080/services/$SERVICE_ID && break
  
  retry_count=$((retry_count + 1))
  echo "Retry $retry_count/$max_retries in ${backoff}s..."
  sleep $backoff
  backoff=$((backoff * 2))
done
```

### Graceful Degradation

```json
{
  "execution_mode": "degraded",
  "reason": "HDE_UNAVAILABLE",
  "fallback_config": {
    "hde_enabled": false,
    "use_baseline_execution": true
  },
  "affected_capabilities": ["optimization:speed", "optimization:efficiency"]
}
```

---

## 6. Performance Tuning

### Connection Pooling

```bash
# Configure connection pool size
export BONSAI_POOL_SIZE=100
export BONSAI_POOL_TIMEOUT_SECS=30
```

### Request Batching

```bash
curl -X POST -H "Authorization: Bearer $BONSAI_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "requests": [
      {
        "method": "GET",
        "path": "/services",
        "query": {"limit": 10}
      },
      {
        "method": "GET",
        "path": "/modules/search",
        "query": {"query": "fax"}
      }
    ]
  }' \
  http://localhost:8080/batch
```

### Caching Strategies

```bash
# Enable client-side caching
curl -H "Authorization: Bearer $BONSAI_TOKEN" \
  -H "Cache-Control: max-age=300" \
  http://localhost:8080/modules/mod_fax_001
```

---

## 7. Monitoring and Observability

### Health Check Endpoint

```bash
curl http://localhost:8080/health
```

**Response:**
```json
{
  "status": "healthy",
  "version": "1.0.0",
  "components": {
    "api_gateway": "healthy",
    "service_manager": "healthy",
    "cas_backend": "healthy",
    "clojure_runtime": "healthy"
  }
}
```

### Prometheus Metrics

```bash
curl http://localhost:9090/metrics
```

### OpenTelemetry Tracing

```bash
# Configure OTLP exporter
export OTEL_EXPORTER_OTLP_ENDPOINT="http://localhost:4317"
export OTEL_SERVICE_NAME="omnisystem-api"
```

---

## 8. Security Best Practices

### Token Rotation

```bash
# Rotate token every 30 days
bonsai auth rotate --expiration-days 30
```

### Rate Limiting

Default limits:
- 1000 requests/minute per token
- 100 concurrent requests per token
- 10 GB/day data transfer per token

### IP Whitelisting

```bash
curl -X POST -H "Authorization: Bearer $BONSAI_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "token": "YOUR_API_TOKEN",
    "whitelist": ["192.168.1.0/24", "10.0.0.0/8"]
  }' \
  http://localhost:8080/auth/tokens/whitelist
```

---

## 9. Version Compatibility

### API Versioning

The API uses URL-based versioning:
- `/v1/` - Current stable version
- `/v2/` - Next major version (beta)
- `/experimental/` - Experimental features

### Version Migration

```bash
# Old endpoint (v1)
curl -H "Authorization: Bearer $BONSAI_TOKEN" \
  http://localhost:8080/v1/services

# New endpoint (v2)
curl -H "Authorization: Bearer $BONSAI_TOKEN" \
  http://localhost:8080/v2/services
```

---

## 10. Troubleshooting

### Common Issues

**Issue**: "Service not found"
```bash
# Verify service exists
curl -H "Authorization: Bearer $BONSAI_TOKEN" \
  http://localhost:8080/services | jq '.services[] | select(.name == "fax-service")'
```

**Issue**: "Token expired"
```bash
# Regenerate token
bonsai auth login
export BONSAI_TOKEN=$(bonsai auth token)
```

**Issue**: "Resource quota exceeded"
```bash
# Check resource usage
curl -H "Authorization: Bearer $BONSAI_TOKEN" \
  http://localhost:8080/environments/env_prod_001 | jq '.metrics'
```

### Debug Mode

```bash
# Enable debug logging
export RUST_LOG=debug
bonsai start --log-level debug
```

---

**Version**: 1.0.0  
**Last Updated**: 2026-06-07  
**Status**: Production Ready

For additional support, visit: https://docs.omnisystem.io/api
