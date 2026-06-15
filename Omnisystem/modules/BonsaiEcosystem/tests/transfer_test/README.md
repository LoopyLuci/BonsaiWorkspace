# TransferDaemon P2P Real Internet Transfer Test

**Production-Grade Real Internet File Transfer Test** for the TransferDaemon Protocol and FTDaemon system.

## Overview

This test framework creates **two independent real nodes** (Sanctum vaults) that communicate over the **actual internet** to perform file transfers using TransferDaemon's multi-path bonding, NAT traversal, and P2P protocols.

**Key Features**:
- ✅ Real nodes, real internet (NO simulation, NO Docker)
- ✅ Multi-path bonding with per-lane metrics
- ✅ Sanctum vault isolation (security & reproducibility)
- ✅ Comprehensive metrics collection (throughput, latency, packet loss)
- ✅ File integrity verification (BLAKE3 hashing)
- ✅ Full end-to-end testing of TransferDaemon protocol layers
- ✅ Aether & Titan implementation (Omni-languages)
- ✅ Automatic report generation (JSON + HTML)

## Architecture

```
┌────────────────────────────────────────────────────────────┐
│          Test Orchestrator (Aether Actor)                 │
│   • Manages test lifecycle                                 │
│   • Coordinates nodes                                      │
│   • Collects metrics                                       │
└───────────────────────┬────────────────────────────────────┘
                        │
        ┌───────────────┴────────────────┐
        │                                │
┌───────▼───────┐                ┌───────▼───────┐
│   Node A      │                │   Node B      │
│ (Sanctum)     │◄─ Internet ───►│ (Sanctum)     │
│               │                │               │
│ • TransferD.  │                │ • TransferD.  │
│ • FTDaemon    │                │ • FTDaemon    │
│ • BLAKE3 hash │                │ • BLAKE3 hash │
└───────────────┘                └───────────────┘

Real network: TCP, QUIC, WebRTC, Direct lanes
Multi-path: Simultaneous transfers over multiple routes
Encryption: Post-quantum hybrid (default)
```

## Directory Structure

```
transfer_test/
├── types.ti                 # Type definitions (Titan)
├── orchestrator.ae          # Main orchestrator (Aether)
├── node_controller.ae       # Individual node control (Aether)
├── metrics_collector.ae     # Metrics aggregation (Aether)
├── config.ae                # Configuration management (Aether)
├── main.ae                  # Entry point (Aether)
├── run_test.py              # Python test runner
├── test_config.json         # Example configuration
├── README.md                # This file
└── INTEGRATION.md           # UVM integration guide
```

## Prerequisites

### System Requirements

- **Two machines** with public IP addresses and internet connectivity
  - Minimum: 2 vCPU, 2 GB RAM, 100 GB disk each
  - Recommended: Cloud VMs (AWS EC2, Google Cloud, Azure, etc.)

- **Omnisystem Environment**:
  - Omnisystem kernel installed
  - Omni compiler (omni build)
  - Aether runtime
  - Titan compiler

- **Services Running on Each Node**:
  - TransferDaemon (post-quantum crypto enabled)
  - FTDaemon core (with FFI bindings)
  - Sanctum vault (for isolation)

- **Network Setup**:
  - Port 8114+ open for FTDaemon API
  - Port 9050-9055 open for TransferDaemon P2P
  - Firewall rules allowing UDP/TCP for NAT traversal

### Build & Compilation

```bash
# Install Omni compiler
curl https://releases.omnisystem.io/omni-latest.tar.gz | tar xz
export PATH=$PATH:./omni/bin

# Verify installation
omni --version

# Build FTDaemon core
cd ../..
cargo build --release --package ftdaemon-core

# Ensure TransferDaemon is running on both nodes
# (verify with: curl http://node-address:8114/health)
```

## Configuration

### Environment Variables

```bash
# Node A Configuration
export FTDAEMON_NODE_A_ID="node-a"
export FTDAEMON_NODE_A_ADDR="54.123.45.67"          # Public IP
export FTDAEMON_NODE_A_PORT="8114"
export FTDAEMON_NODE_A_VAULT_ID="vault-a"
export FTDAEMON_NODE_A_DATA_DIR="/data/node-a"

# Node B Configuration
export FTDAEMON_NODE_B_ID="node-b"
export FTDAEMON_NODE_B_ADDR="34.234.56.78"          # Public IP
export FTDAEMON_NODE_B_PORT="8114"
export FTDAEMON_NODE_B_VAULT_ID="vault-b"
export FTDAEMON_NODE_B_DATA_DIR="/data/node-b"

# Test Parameters
export FTDAEMON_TEST_FILE_SIZE_MB="50"              # 50 MB test file
export FTDAEMON_TEST_FILE_COUNT="1"
export FTDAEMON_TEST_TIMEOUT="300"                  # 5 minute timeout

# Feature Flags
export FTDAEMON_MULTI_PATH="true"                   # Enable multi-path bonding
export FTDAEMON_ENCRYPTION="true"                   # Enable AES-256+post-quantum
export FTDAEMON_COMPRESSION="false"
export FTDAEMON_VERIFY_INTEGRITY="true"             # BLAKE3 verification

# Monitoring
export FTDAEMON_MONITOR_METRICS="true"
export FTDAEMON_SAVE_RESULTS="true"
export FTDAEMON_RESULTS_PATH="/tmp/ftdaemon_test_results.json"

# Authentication
export FTDAEMON_CAPABILITY_TOKEN="<capability-token>"
```

### Configuration File (test_config.json)

```json
{
  "node_a": {
    "id": "node-a",
    "name": "Node A",
    "address": "54.123.45.67",
    "port": 8114,
    "sanctum_vault_id": "vault-a",
    "data_directory": "/data/node-a"
  },
  "node_b": {
    "id": "node-b",
    "name": "Node B",
    "address": "34.234.56.78",
    "port": 8114,
    "sanctum_vault_id": "vault-b",
    "data_directory": "/data/node-b"
  },
  "file_size_mb": 50,
  "file_count": 1,
  "timeout_seconds": 300,
  "multi_path_enabled": true,
  "encryption_enabled": true,
  "compression_enabled": false,
  "verify_integrity": true,
  "monitor_system_metrics": true,
  "save_results_to_file": true,
  "results_output_path": "/tmp/ftdaemon_test_results.json"
}
```

## Running the Test

### Quick Start (Using Python Runner)

```bash
# Set environment variables
source node_config.env

# Run test with default configuration
python3 run_test.py

# Run with custom config file
python3 run_test.py --config test_config.json

# Run with custom file size and timeout
python3 run_test.py --file-size 100 --timeout 600

# Run without encryption (test baseline)
python3 run_test.py --no-encryption

# Run without multi-path (single path test)
python3 run_test.py --no-multi-path
```

### Direct Compilation & Execution

```bash
# Compile Omni-language modules
omni build -m transfer_test::main --release

# Run orchestrator
omni run transfer_test \
  --env FTDAEMON_NODE_A_ADDR=54.123.45.67 \
  --env FTDAEMON_NODE_B_ADDR=34.234.56.78 \
  --env FTDAEMON_TEST_FILE_SIZE_MB=50

# Monitor progress
tail -f /tmp/ftdaemon_test_$(date +%s).log
```

## Test Workflow

### Phase 1: Initialization (1-2 min)
```
✓ Load configuration
✓ Verify node connectivity
✓ Establish RemoteCall channels
✓ Derive capability tokens
```

### Phase 2: Peer Discovery (30-60 sec)
```
✓ Get peer IDs from TransferDaemon
✓ Initiate DHT lookup
✓ NAT traversal (DCUtR)
✓ Verify bidirectional connectivity
```

### Phase 3: Test Preparation (10-30 sec)
```
✓ Generate random test file (50 MB default)
✓ Compute source BLAKE3 hash
✓ Create destination directory
```

### Phase 4: Multi-Path Transfer (variable)
```
✓ Negotiate encryption (post-quantum hybrid)
✓ Establish lanes:
  - TCP direct
  - QUIC direct
  - WebRTC datachannel
  - Relay (if direct fails)
✓ Distribute file across lanes
✓ Monitor per-lane throughput
✓ Automatic lane failover if needed
```

### Phase 5: Verification (10-30 sec)
```
✓ Compute destination BLAKE3 hash
✓ Compare hashes
✓ Collect per-lane statistics
```

### Phase 6: Cleanup & Reporting (10 sec)
```
✓ Delete test files
✓ Close connections
✓ Generate JSON/HTML report
✓ Upload metrics to UVM
```

## Expected Results

### Successful Transfer (Multi-Path)

```
═══════════════════════════════════════════════════════════════
TEST RESULTS
═══════════════════════════════════════════════════════════════

✅ TEST PASSED

Test Duration: 45,230 ms
Total Bytes Transferred: 52,428,800 bytes (50 MB)
Average Throughput: 92.5 Mbps
Peak Throughput: 215.3 Mbps
Average Latency: 18.4 ms
Packet Loss: 0.02%
Lanes Used: tcp, quic, webrtc

Transfer Details:
  ✅ Source: node-a (/tmp/test_file_50mb.bin)
  ✅ Destination: node-b (/tmp/test_file_received.bin)
  ✅ Hash verified: BLAKE3 match
  ✅ Duration: 45 seconds
  ✅ Throughput: 92.5 Mbps
  
Lane Statistics:
  TCP (Direct):     34 MB/s, latency: 12ms, loss: 0%
  QUIC (Direct):    38 MB/s, latency: 15ms, loss: 0.01%
  WebRTC:           20 MB/s, latency: 28ms, loss: 0.1%

═══════════════════════════════════════════════════════════════
```

### Performance Metrics

| Scenario | Throughput | Latency | Packet Loss | Lanes |
|----------|-----------|---------|------------|-------|
| Multi-path (enabled) | 90-215 Mbps | 12-28 ms | <0.1% | 2-3 |
| Single path (TCP) | 40-80 Mbps | 12-20 ms | 0% | 1 |
| Relay (NAT blocked) | 10-30 Mbps | 50-200 ms | 0.5-2% | 1 |
| Over VPN | 20-100 Mbps | 30-100 ms | 0.1-1% | 1 |

## Output & Reporting

### Test Report Format

**JSON Report** (`/tmp/ftdaemon_test_results_TIMESTAMP.json`):
```json
{
  "test_id": "ftdaemon-p2p-2026-06-07T14:30:00",
  "timestamp": "2026-06-07T14:30:45Z",
  "status": "passed",
  "duration_seconds": 45.23,
  "configuration": {
    "file_size_mb": 50,
    "multi_path": true,
    "encryption": true,
    "compression": false
  },
  "summary": {
    "bytes_transferred": 52428800,
    "throughput_mbps": 92.5,
    "latency_ms": 18.4,
    "packet_loss_percent": 0.02,
    "hash_verified": true
  },
  "lanes": [
    {
      "lane_type": "tcp",
      "bytes_sent": 17809066,
      "latency_ms": 12,
      "bandwidth_mbps": 34.0,
      "reliability_percent": 100.0
    },
    {
      "lane_type": "quic",
      "bytes_sent": 19972122,
      "latency_ms": 15,
      "bandwidth_mbps": 38.0,
      "reliability_percent": 99.99
    },
    {
      "lane_type": "webrtc",
      "bytes_sent": 10471456,
      "latency_ms": 28,
      "bandwidth_mbps": 20.0,
      "reliability_percent": 99.9
    }
  ]
}
```

**HTML Report**:
- Browser-viewable performance charts
- Transfer timeline visualization
- Per-lane metrics breakdown
- System resource usage graphs
- Pass/fail summary

### Result File Locations

```bash
/tmp/ftdaemon_test_results_TIMESTAMP.json         # Raw results
/tmp/ftdaemon_test_report_TIMESTAMP.html          # Visual report
/tmp/ftdaemon_test_TIMESTAMP.log                  # Full execution log
```

## Troubleshooting

### Node Connectivity Issues

```bash
# Test basic connectivity
curl -v http://NODE_ADDRESS:8114/health

# Check TransferDaemon status
curl http://NODE_ADDRESS:8114/api/v1/transfer/status

# Verify network interface
ip addr show
ping -c 5 REMOTE_NODE_ADDRESS

# Check firewall rules
sudo iptables -L -n | grep 8114
sudo iptables -L -n | grep 9050
```

### Peer Discovery Failures

```bash
# Check DHT state
curl http://NODE_ADDRESS:8114/api/v1/dht/status

# Monitor DNS resolution
dig REMOTE_NODE_ADDRESS

# Test NAT traversal
curl http://NODE_ADDRESS:8114/api/v1/nat/status
```

### Transfer Timeout

```bash
# Increase timeout
export FTDAEMON_TEST_TIMEOUT=600  # 10 minutes

# Check system resources
free -m          # Memory
df -h /          # Disk space
iostat -x 1 5    # I/O stats

# Monitor transfer progress
curl http://NODE_A:8114/api/v1/transfer/TRANSFER_ID/status | jq .
```

## Integration with UVM

Register this test with the Universal Validation Mesh for continuous testing:

```bash
# Register test
python3 -c "
from transfer_test import main
asyncio.run(main.register_with_uvm(
    'https://uvm.omnisystem.io',
    capability
))
"

# Test runs nightly on dedicated infrastructure
# Results aggregated and compared against baseline
# Regression detected via SLO alerting
```

## Performance Tuning

### Optimize for Throughput

```bash
# Disable compression (unless network-bound)
export FTDAEMON_COMPRESSION=false

# Enable all lanes
export FTDAEMON_MULTI_PATH=true

# Use larger file (less overhead)
export FTDAEMON_TEST_FILE_SIZE_MB=500

# Test from data center (low latency)
# Use instances in same region/AZ
```

### Optimize for Reliability

```bash
# Enable compression (adds redundancy)
export FTDAEMON_COMPRESSION=true

# Force specific lane for baseline
# Modify transfer_test/orchestrator.ae

# Use encrypted channel
export FTDAEMON_ENCRYPTION=true
```

## References

- [TransferDaemon Protocol Spec](../../docs/TRANSFER_DAEMON.md)
- [FTDaemon API Reference](../../PHASE_4_INTEGRATION.md)
- [Sanctum Vault Documentation](../../docs/SANCTUM.md)
- [Omni-Language Specification](../../docs/OMNI_LANGUAGES.md)
- [UVM Integration Guide](./INTEGRATION.md)

## License

Part of the Bonsai Ecosystem. See LICENSE file.

---

**Status**: ✅ Production Ready for Real Internet Testing  
**Last Updated**: 2026-06-07  
**Maintainer**: BonsaiEcosystem Team
