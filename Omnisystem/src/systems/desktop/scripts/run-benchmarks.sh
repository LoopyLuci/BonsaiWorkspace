#!/bin/bash
# Run performance benchmarks for mobile remote desktop

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
RESULTS_DIR="$PROJECT_ROOT/benchmark-results"

echo "=========================================="
echo "Bonsai Remote Desktop Benchmarks"
echo "=========================================="
echo ""

# Create results directory
mkdir -p "$RESULTS_DIR"

# Check prerequisites
check_prerequisites() {
    if ! command -v adb &> /dev/null; then
        echo "ERROR: adb not found. Install Android SDK Platform Tools"
        exit 1
    fi

    if ! command -v cargo &> /dev/null; then
        echo "ERROR: cargo not found. Install Rust from https://rustup.rs"
        exit 1
    fi

    echo "✓ Prerequisites met"
}

# Check device connection
check_device() {
    echo ""
    echo "Checking for connected device..."

    DEVICES=$(adb devices | grep -v "List of attached" | grep "device$" | wc -l)

    if [ "$DEVICES" -eq 0 ]; then
        echo "ERROR: No Android devices connected"
        exit 1
    fi

    DEVICE_MODEL=$(adb shell getprop ro.product.model)
    DEVICE_API=$(adb shell getprop ro.build.version.sdk)

    echo "✓ Device connected: $DEVICE_MODEL (API $DEVICE_API)"
}

# Benchmark: Video decode latency
benchmark_decode_latency() {
    echo ""
    echo "Benchmark 1/5: Video Decode Latency"
    echo "  Testing H.264 decode performance..."

    RESULTS_FILE="$RESULTS_DIR/decode-latency.json"

    adb shell am instrument -w \
        com.bonsai.remote_desktop.test/androidx.test.runner.AndroidJUnitRunner \
        -e class com.bonsai.remote_desktop.tests.VideoDecodeBenchmark \
        2>&1 | tee "$RESULTS_FILE"

    if [ -f "$RESULTS_DIR/decode-latency.json" ]; then
        DECODE_P95=$(grep "decode_latency_p95" "$RESULTS_FILE" | cut -d: -f2 | tr -d ' ,')
        echo "  ✓ Decode latency P95: ${DECODE_P95}ms"
    fi
}

# Benchmark: Touch input latency
benchmark_input_latency() {
    echo ""
    echo "Benchmark 2/5: Touch Input Latency"
    echo "  Testing touch-to-visible latency..."

    RESULTS_FILE="$RESULTS_DIR/input-latency.json"

    adb shell am instrument -w \
        com.bonsai.remote_desktop.test/androidx.test.runner.AndroidJUnitRunner \
        -e class com.bonsai.remote_desktop.tests.InputLatencyBenchmark \
        2>&1 | tee "$RESULTS_FILE"

    if [ -f "$RESULTS_FILE" ]; then
        INPUT_P95=$(grep "input_latency_p95" "$RESULTS_FILE" | cut -d: -f2 | tr -d ' ,')
        echo "  ✓ Input latency P95: ${INPUT_P95}ms"
    fi
}

# Benchmark: Memory usage
benchmark_memory() {
    echo ""
    echo "Benchmark 3/5: Memory Usage"
    echo "  Measuring heap and native memory..."

    RESULTS_FILE="$RESULTS_DIR/memory-usage.txt"

    # Clear cache first
    adb shell pm clear com.bonsai.remote_desktop

    # Start recording memory
    echo "Session Duration,Heap (MB),Native (MB),Total (MB)" > "$RESULTS_FILE"

    for i in {0..10}; do
        sleep 6

        MEMINFO=$(adb shell dumpsys meminfo com.bonsai.remote_desktop)
        HEAP=$(echo "$MEMINFO" | grep "TOTAL" | awk '{print $2}')
        NATIVE=$(echo "$MEMINFO" | grep "Native Heap" | head -1 | awk '{print $4}')
        TOTAL=$(echo "$MEMINFO" | grep "TOTAL" | awk '{print $2}')

        echo "$((i * 6)),${HEAP},${NATIVE},${TOTAL}" >> "$RESULTS_FILE"
    done

    PEAK_HEAP=$(tail -n +2 "$RESULTS_FILE" | cut -d, -f2 | sort -n | tail -1)
    echo "  ✓ Peak heap: ${PEAK_HEAP} MB"
}

# Benchmark: Battery drain
benchmark_battery() {
    echo ""
    echo "Benchmark 4/5: Battery Drain"
    echo "  Measuring battery drain rate (this takes 10 minutes)..."

    RESULTS_FILE="$RESULTS_DIR/battery-drain.txt"

    # Get initial battery level
    adb shell dumpsys battery > /tmp/battery-start.txt
    INITIAL_LEVEL=$(grep "level:" /tmp/battery-start.txt | awk '{print $2}')

    echo "Initial battery level: ${INITIAL_LEVEL}%"
    echo "Time (min),Battery (%)" > "$RESULTS_FILE"

    # Run session for 10 minutes, sample every minute
    for i in {1..10}; do
        sleep 60

        CURRENT_LEVEL=$(adb shell dumpsys battery | grep "level:" | awk '{print $2}')
        echo "$i,${CURRENT_LEVEL}" >> "$RESULTS_FILE"

        DRAIN=$((INITIAL_LEVEL - CURRENT_LEVEL))
        DRAIN_PER_HOUR=$((DRAIN * 60 / i))

        echo "  Time: ${i}min | Level: ${CURRENT_LEVEL}% | Drain rate: ~${DRAIN_PER_HOUR}%/hour"
    done

    FINAL_LEVEL=$(adb shell dumpsys battery | grep "level:" | awk '{print $2}')
    TOTAL_DRAIN=$((INITIAL_LEVEL - FINAL_LEVEL))
    DRAIN_PER_HOUR=$((TOTAL_DRAIN * 6))

    echo "  ✓ Battery drain: ${TOTAL_DRAIN}% in 10 minutes (~${DRAIN_PER_HOUR}%/hour)"
}

# Benchmark: Network performance
benchmark_network() {
    echo ""
    echo "Benchmark 5/5: Network Performance"
    echo "  Testing bitrate, latency, and stability..."

    RESULTS_FILE="$RESULTS_DIR/network-performance.json"

    cargo test --release -p mcp-server --test integration_tests \
        -- --nocapture 2>&1 | tee "$RESULTS_FILE"

    echo "  ✓ Network tests completed"
}

# Generate HTML report
generate_report() {
    echo ""
    echo "Generating benchmark report..."

    REPORT_FILE="$RESULTS_DIR/benchmark-report.html"

    cat > "$REPORT_FILE" << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>Bonsai Remote Desktop Benchmarks</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; background: #f5f5f5; }
        .container { max-width: 1200px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; }
        h1 { color: #333; border-bottom: 3px solid #4caf50; padding-bottom: 10px; }
        .benchmark { margin: 20px 0; padding: 15px; background: #f9f9f9; border-left: 4px solid #4caf50; }
        .benchmark h2 { color: #4caf50; margin-top: 0; }
        .metric { display: inline-block; margin-right: 30px; }
        .metric-label { color: #666; font-size: 0.9em; }
        .metric-value { font-size: 1.5em; font-weight: bold; color: #333; }
        .good { color: #4caf50; }
        .warning { color: #ff9800; }
        .bad { color: #f44336; }
        table { width: 100%; border-collapse: collapse; margin: 10px 0; }
        th, td { padding: 10px; text-align: left; border-bottom: 1px solid #ddd; }
        th { background: #f0f0f0; font-weight: bold; }
        .timestamp { color: #999; font-size: 0.9em; }
    </style>
</head>
<body>
    <div class="container">
        <h1>Bonsai Remote Desktop Benchmark Report</h1>
        <p class="timestamp" id="timestamp"></p>

        <div class="benchmark">
            <h2>1. Video Decode Latency</h2>
            <p>Measures time from frame received to displayed on screen.</p>
            <div class="metric">
                <div class="metric-label">P50 Latency</div>
                <div class="metric-value"><span id="decode-p50" class="good">—</span> ms</div>
            </div>
            <div class="metric">
                <div class="metric-label">P95 Latency</div>
                <div class="metric-value"><span id="decode-p95">—</span> ms</div>
            </div>
            <div class="metric">
                <div class="metric-label">P99 Latency</div>
                <div class="metric-value"><span id="decode-p99">—</span> ms</div>
            </div>
        </div>

        <div class="benchmark">
            <h2>2. Touch Input Latency</h2>
            <p>Measures end-to-end latency from touch input to visible response.</p>
            <div class="metric">
                <div class="metric-label">P50 Latency</div>
                <div class="metric-value"><span id="input-p50" class="good">—</span> ms</div>
            </div>
            <div class="metric">
                <div class="metric-label">P95 Latency</div>
                <div class="metric-value"><span id="input-p95">—</span> ms</div>
            </div>
        </div>

        <div class="benchmark">
            <h2>3. Memory Usage</h2>
            <p>Heap memory usage during sustained session.</p>
            <div class="metric">
                <div class="metric-label">Peak Heap</div>
                <div class="metric-value"><span id="memory-peak" class="good">—</span> MB</div>
            </div>
            <div class="metric">
                <div class="metric-label">Memory Leak Rate</div>
                <div class="metric-value"><span id="memory-leak" class="good">—</span> MB/min</div>
            </div>
        </div>

        <div class="benchmark">
            <h2>4. Battery Drain</h2>
            <p>Battery consumption during active remote session.</p>
            <div class="metric">
                <div class="metric-label">Drain Rate</div>
                <div class="metric-value"><span id="battery-drain" class="good">—</span> %/hour</div>
            </div>
            <div class="metric">
                <div class="metric-label">Expected Duration</div>
                <div class="metric-value"><span id="battery-duration" class="good">—</span> hours</div>
            </div>
        </div>

        <div class="benchmark">
            <h2>5. Network Performance</h2>
            <p>Network stability and adaptation metrics.</p>
            <table>
                <tr>
                    <th>Metric</th>
                    <th>Value</th>
                    <th>Target</th>
                </tr>
                <tr>
                    <td>Packet Loss</td>
                    <td><span id="network-pkt-loss">—</span> %</td>
                    <td>&lt; 0.5%</td>
                </tr>
                <tr>
                    <td>RTT Latency</td>
                    <td><span id="network-rtt">—</span> ms</td>
                    <td>&lt; 10ms</td>
                </tr>
                <tr>
                    <td>Jitter</td>
                    <td><span id="network-jitter">—</span> ms</td>
                    <td>&lt; 5ms</td>
                </tr>
                <tr>
                    <td>Bitrate Stability</td>
                    <td><span id="network-stability">—</span> %</td>
                    <td>&gt; 95%</td>
                </tr>
            </table>
        </div>
    </div>

    <script>
        document.getElementById('timestamp').textContent = 'Generated: ' + new Date().toISOString();
    </script>
</body>
</html>
EOF

    echo "✓ Report saved to: $REPORT_FILE"
}

# Show summary
show_summary() {
    echo ""
    echo "=========================================="
    echo "Benchmark Complete!"
    echo "=========================================="
    echo ""
    echo "Results saved to: $RESULTS_DIR/"
    echo ""
    ls -lh "$RESULTS_DIR/"
    echo ""
    echo "View HTML report:"
    echo "  open $RESULTS_DIR/benchmark-report.html"
    echo ""
}

# Main execution
main() {
    check_prerequisites
    check_device

    benchmark_decode_latency
    benchmark_input_latency
    benchmark_memory
    benchmark_battery
    benchmark_network

    generate_report
    show_summary
}

main
