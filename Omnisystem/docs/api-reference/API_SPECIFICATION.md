# Omni-Bot API Specification - Phase 1

Complete REST API specification for Validation (UVM), Driver Converter, and HDE Management systems.

## Base URL
```
http://localhost:3000/api
```

---

## Validation (UVM) API

### 1. POST /validation/run
Execute a test suite with matrix configuration and parallelism settings.

**Request Body:**
```json
{
  "name": "test_suite_v1",
  "description": "Comprehensive matrix validation",
  "matrix": {
    "axes": [
      {
        "name": "platform",
        "values": ["linux", "windows", "macos"]
      },
      {
        "name": "optimization",
        "values": ["debug", "release"]
      }
    ],
    "total_combinations": 6
  },
  "parallelism": {
    "max_parallel_tests": 8,
    "worker_pool_size": 4,
    "queue_depth": 64
  },
  "timeout": {
    "per_test_secs": 300,
    "total_run_secs": 3600,
    "warmup_secs": 60
  },
  "metadata": {
    "branch": "main",
    "commit": "abc123def"
  }
}
```

**Response (202 Accepted):**
```json
{
  "run_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "status": "queued",
  "total_combinations": 6,
  "timestamp": "2026-06-07T10:30:00Z"
}
```

---

### 2. GET /validation/results/{id}
Retrieve validation run results.

**Path Parameters:**
- `id`: ValidationRunId (UUID)

**Response (200 OK):**
```json
{
  "run_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "name": "test_suite_v1",
  "status": "COMPLETED",
  "started_at": "2026-06-07T10:30:00Z",
  "completed_at": "2026-06-07T10:45:30Z",
  "total_tests": 6,
  "passed": 5,
  "failed": 1,
  "skipped": 0,
  "timeout": 0,
  "results": [
    {
      "test_id": "test_0000",
      "status": "PASSED",
      "duration_ms": 245,
      "timestamp": "2026-06-07T10:30:15Z",
      "error": null,
      "metrics": {
        "memory_peak_mb": 512,
        "cpu_avg_percent": 45,
        "iops": 1000,
        "cache_hits": 1000,
        "cache_misses": 100
      }
    }
  ],
  "summary_metrics": {
    "total_duration_ms": 930000,
    "avg_test_duration_ms": 155000,
    "peak_memory_mb": 768,
    "avg_cpu_percent": 42,
    "success_rate_percent": 83.33
  }
}
```

---

### 3. GET /validation/heatmap
Retrieve visual heatmap of validation results.

**Query Parameters:**
- `run_id`: ValidationRunId (UUID)
- `x_axis`: Optional axis label (default: "X")
- `y_axis`: Optional axis label (default: "Y")

**Response (200 OK):**
```json
{
  "run_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "axes": ["X", "Y"],
  "cells": [
    {
      "coordinates": [0, 0],
      "value": 1.0,
      "status": "PASSED",
      "label": "test_00"
    },
    {
      "coordinates": [0, 1],
      "value": 0.0,
      "status": "FAILED",
      "label": "test_01"
    }
  ],
  "legend": {
    "min_value": 0.0,
    "max_value": 1.0,
    "color_scale": "viridis"
  }
}
```

---

### 4. POST /validation/replay
Deterministically replay a validation run.

**Request Body:**
```json
{
  "original_run_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "specific_tests": ["test_0000", "test_0001"]
}
```

**Response (202 Accepted):**
```json
{
  "new_run_id": "a1b2c3d4-e5f6-47g8-h9i0-j1k2l3m4n5o6",
  "original_run_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "status": "replaying",
  "timestamp": "2026-06-07T11:00:00Z"
}
```

---

### 5. GET /validation/results/{id}/trace
Retrieve execution trace for a validation run.

**Path Parameters:**
- `id`: ValidationRunId (UUID)

**Response (200 OK):**
```json
{
  "run_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "events": [
    {
      "timestamp": "2026-06-07T10:30:00Z",
      "event_type": "validation_started",
      "details": {
        "name": "test_suite_v1",
        "matrix_combinations": 6
      }
    },
    {
      "timestamp": "2026-06-07T10:30:15Z",
      "event_type": "test_completed",
      "details": {
        "test_id": "test_0000",
        "status": "PASSED",
        "duration_ms": 245
      }
    },
    {
      "timestamp": "2026-06-07T10:45:30Z",
      "event_type": "validation_completed",
      "details": {
        "passed": 5,
        "failed": 1,
        "total": 6
      }
    }
  ],
  "total_events": 8
}
```

---

### 6. GET /validation/history
Retrieve historical validation runs with pagination.

**Query Parameters:**
- `page`: Page number (0-indexed, default: 0)
- `per_page`: Results per page (default: 20, max: 100)

**Response (200 OK):**
```json
{
  "items": [
    {
      "run_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
      "name": "test_suite_v1",
      "timestamp": "2026-06-07T10:30:00Z",
      "status": "COMPLETED",
      "passed": 5,
      "failed": 1,
      "total_tests": 6,
      "duration_ms": 930000
    }
  ],
  "total": 42,
  "page": 0,
  "per_page": 20,
  "total_pages": 3
}
```

---

### WebSocket: /validation/progress/{id}
Stream real-time progress updates for a validation run.

**Connection:**
```
ws://localhost:3000/api/validation/progress/f47ac10b-58cc-4372-a567-0e02b2c3d479
```

**Message Format (sent every 500ms):**
```json
{
  "run_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "status": "RUNNING",
  "progress": 45,
  "passed": 3,
  "failed": 1,
  "total": 6,
  "timestamp": "2026-06-07T10:35:00Z"
}
```

---

## Driver Converter API

### 1. POST /driver/convert
Convert DIS to driver with optimization flags.

**Request Body:**
```json
{
  "dis_content": "// DIS code here",
  "dis_name": "video_driver",
  "target_platform": "linux",
  "optimization": {
    "enable_lto": true,
    "codegen_units": 16,
    "vectorization": true,
    "inline_threshold": 100
  },
  "background": true
}
```

**Response (202 Accepted):**
```json
{
  "job_id": "d47ac10b-58cc-4372-a567-0e02b2c3d480",
  "status": "queued",
  "background": true,
  "timestamp": "2026-06-07T11:15:00Z"
}
```

---

### 2. GET /driver/results/{id}
Retrieve conversion status and results.

**Path Parameters:**
- `id`: ConversionJobId (UUID)

**Response (200 OK):**
```json
{
  "job_id": "d47ac10b-58cc-4372-a567-0e02b2c3d480",
  "dis_name": "video_driver",
  "target_platform": "linux",
  "status": "COMPLETED",
  "started_at": "2026-06-07T11:15:00Z",
  "completed_at": "2026-06-07T11:16:45Z",
  "driver_binary": "base64_encoded_binary_data",
  "driver_checksum": "blake3_hash_value",
  "compilation_log": "DIS to IR conversion started\n...",
  "warnings": ["Warning: Unsafe memory access in function X"],
  "errors": []
}
```

---

### 3. POST /driver/{id}/install
Install converted driver to UMS.

**Path Parameters:**
- `id`: ConversionJobId (UUID)

**Request Body:**
```json
{
  "version": "1.0.0",
  "auto_activate": true,
  "rollback_on_error": true
}
```

**Response (202 Accepted):**
```json
{
  "installation_id": "i47ac10b-58cc-4372-a567-0e02b2c3d481",
  "driver_name": "video_driver",
  "version": "1.0.0",
  "status": "installing",
  "auto_activate": true,
  "rollback_on_error": true,
  "timestamp": "2026-06-07T11:17:00Z"
}
```

---

## HDE Management API

### 1. GET /hde/models
List all AI models with deployment states.

**Response (200 OK):**
```json
{
  "models": [
    {
      "id": "claude-v3.1",
      "name": "Claude v3.1",
      "version": "3.1.0",
      "state": "active",
      "created_at": "2026-06-01T00:00:00Z",
      "last_updated": "2026-06-07T10:00:00Z",
      "metrics": {
        "accuracy": 0.98,
        "latency_ms": 45.0,
        "throughput_rps": 1200.0,
        "error_rate": 0.001,
        "resource_efficiency": 0.92
      },
      "safety_envelope": {
        "max_context_length": 200000,
        "allowed_operations": [
          "text_generation",
          "code_analysis",
          "safe_transformations"
        ],
        "resource_limits": {
          "memory_mb": 2048,
          "cpu_percent": 50,
          "max_tokens": 4096,
          "timeout_secs": 300
        },
        "validation_required": true
      }
    }
  ],
  "total": 2,
  "timestamp": "2026-06-07T11:20:00Z"
}
```

---

### 2. POST /hde/models/{name}/promote
Promote shadow model to active.

**Path Parameters:**
- `name`: Model name (e.g., "claude-v3.2-shadow")

**Request Body:**
```json
{
  "version": "3.2.0-rc1",
  "validation_passed": true,
  "rollout_percentage": 100
}
```

**Response (200 OK):**
```json
{
  "model": {
    "id": "claude-v3.2",
    "name": "Claude v3.2",
    "version": "3.2.0-rc1",
    "state": "active",
    "created_at": "2026-06-05T00:00:00Z",
    "last_updated": "2026-06-07T11:25:00Z",
    "metrics": {
      "accuracy": 0.985,
      "latency_ms": 42.0,
      "throughput_rps": 1300.0,
      "error_rate": 0.0008,
      "resource_efficiency": 0.94
    },
    "safety_envelope": {}
  },
  "previous_state": "shadow",
  "new_state": "active",
  "rollout_percentage": 100,
  "timestamp": "2026-06-07T11:25:00Z"
}
```

---

### 3. POST /hde/models/{name}/demote
Demote active model to shadow or archived.

**Path Parameters:**
- `name`: Model name

**Request Body:**
```json
{
  "reason": "Performance regression detected in shadow validation",
  "preserve_shadow": true
}
```

**Response (200 OK):**
```json
{
  "model": {
    "id": "claude-v3.1",
    "name": "Claude v3.1",
    "version": "3.1.0",
    "state": "shadow",
    "created_at": "2026-06-01T00:00:00Z",
    "last_updated": "2026-06-07T11:30:00Z",
    "metrics": {},
    "safety_envelope": {}
  },
  "previous_state": "active",
  "new_state": "shadow",
  "reason": "Performance regression detected in shadow validation",
  "preserve_shadow": true,
  "timestamp": "2026-06-07T11:30:00Z"
}
```

---

### 4. GET /hde/shadow-reports
Retrieve validation reports for shadow models.

**Query Parameters:**
- `model`: Optional model name filter

**Response (200 OK):**
```json
{
  "reports": [
    {
      "model_id": "claude-v3.2-shadow",
      "model_version": "3.2.0-rc1",
      "validation_timestamp": "2026-06-07T10:00:00Z",
      "tests_run": 50,
      "tests_passed": 45,
      "tests_failed": 5,
      "safety_violations": [
        {
          "severity": "warning",
          "description": "Bounds check failure in test 10",
          "context": {
            "test_id": "safety_test_010",
            "type": "bounds_check_failure"
          }
        }
      ],
      "performance_deltas": {
        "accuracy_delta": 0.005,
        "latency_delta_ms": -3.0,
        "throughput_delta_rps": 100.0,
        "error_rate_delta": -0.0002
      },
      "ready_for_promotion": true
    }
  ],
  "total_reports": 1,
  "total_violations": 5,
  "critical_violations": 0,
  "all_ready": true,
  "timestamp": "2026-06-07T11:35:00Z"
}
```

---

## Error Handling

All endpoints return structured error responses:

### Bad Request (400)
```json
{
  "error": "Invalid request: DIS content cannot be empty",
  "error_type": "InvalidRequest",
  "timestamp": "2026-06-07T11:40:00Z"
}
```

### Not Found (404)
```json
{
  "error": "Validation run not found: invalid-uuid",
  "error_type": "InvalidRequest",
  "timestamp": "2026-06-07T11:40:00Z"
}
```

### Conflict (409)
```json
{
  "error": "Model is not in shadow state: Active",
  "error_type": "InvalidRequest",
  "timestamp": "2026-06-07T11:40:00Z"
}
```

### Internal Server Error (500)
```json
{
  "error": "Conversion job not completed: Queued",
  "error_type": "ExecutionFailed",
  "timestamp": "2026-06-07T11:40:00Z"
}
```

---

## Status Codes

| Code | Status | Used For |
|------|--------|----------|
| 200 | OK | Successful GET/POST with immediate result |
| 202 | Accepted | Long-running operation (validation, conversion) |
| 400 | Bad Request | Invalid parameters or validation failure |
| 404 | Not Found | Resource not found |
| 409 | Conflict | State conflict (e.g., demotion of non-active) |
| 500 | Internal Error | Server error |

---

## Rate Limiting
Not yet implemented. Planned for Phase 2.

## Authentication
Not yet implemented. Planned for Phase 2.

## CORS
Enabled for all origins. Customize via `tower_http::cors::CorsLayer`.
