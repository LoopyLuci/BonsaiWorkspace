#!/bin/bash
# START_UACS.sh - Launch Universal Agent Control System with Visual Mode + HITL

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║   🧠 Universal Agent Control System (Visual + HITL Mode)       ║"
echo "╚════════════════════════════════════════════════════════════════╝"

BONSAI_DIR="$(pwd)"
echo ""
echo "📍 Working Directory: $BONSAI_DIR"

# Check if cargo exists
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: cargo not found. Please ensure Rust is installed."
    exit 1
fi

echo ""
echo "🔨 Building mcp-server..."
cargo build -p mcp-server --release 2>&1 | tail -10

if [ ${PIPESTATUS[0]} -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

echo ""
echo "✅ Build successful!"

echo ""
echo "📂 Opening three terminals for:"
echo "   1️⃣  Terminal 1: UACS Server (Visual Mode + HITL)"
echo "   2️⃣  Terminal 2: UACS Dashboard (Svelte)"
echo "   3️⃣  Terminal 3: Browser (http://localhost:5173)"

# Terminal 1: UACS Server
echo ""
echo "[Terminal 1] Starting UACS Server..."
gnome-terminal -- bash -c "cd '$BONSAI_DIR'; cargo run -p mcp-server -- visual --hitl-categories destructive,network --port 11426; exec bash" 2>/dev/null || \
xterm -hold -e "cd '$BONSAI_DIR'; cargo run -p mcp-server -- visual --hitl-categories destructive,network --port 11426" &

# Give the server a moment to start
sleep 3

# Terminal 2: Dashboard
echo "[Terminal 2] Starting Dashboard..."
gnome-terminal -- bash -c "cd '$BONSAI_DIR/uacs-dashboard'; npm run dev; exec bash" 2>/dev/null || \
xterm -hold -e "cd '$BONSAI_DIR/uacs-dashboard'; npm run dev" &

# Terminal 3: Open browser
echo "[Terminal 3] Opening dashboard in browser..."
sleep 5
xdg-open "http://localhost:5173" 2>/dev/null || \
open "http://localhost:5173" 2>/dev/null || \
echo "📖 Open http://localhost:5173 in your browser"

echo ""
echo "╔════════════════════════════════════════════════════════════════╗"
echo "║ 🚀 UACS is starting!                                           ║"
echo "╠════════════════════════════════════════════════════════════════╣"
echo "║                                                                ║"
echo "║  Server:    http://127.0.0.1:11426                            ║"
echo "║  Dashboard: http://localhost:5173                             ║"
echo "║                                                                ║"
echo "║  Configuration:                                                ║"
echo "║  - Mode: Visual Agent Control                                 ║"
echo "║  - HITL: ENABLED (destructive, network)                       ║"
echo "║  - Status: Ready to connect agents                            ║"
echo "║                                                                ║"
echo "╠════════════════════════════════════════════════════════════════╣"
echo "║ Next: Configure Claude and give it the self-improvement task  ║"
echo "║ See: CLAUDE_SELF_IMPROVEMENT.md                               ║"
echo "╚════════════════════════════════════════════════════════════════╝"

echo ""
echo "⏳ Keep all terminals running. Press Ctrl+C to stop."
