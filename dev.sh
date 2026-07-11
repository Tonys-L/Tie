#!/usr/bin/env bash
# ====== Tie - Dev Mode (Fast) ======
# Usage: ./dev.sh

set -euo pipefail
cd "$(dirname "$0")"

APP_NAME="tie"

echo ""
echo "========== Tie - Dev Mode =========="

# 1. Kill old process
echo ""
echo "[1/2] Stopping old process..."
if pkill -x "$APP_NAME" 2>/dev/null; then
    echo "  Stopped old process"
    sleep 1
else
    echo "  No running process"
fi

# 2. Run tauri dev
echo ""
echo "[2/2] Starting tauri dev..."
echo "  First run: ~3 min (compiles debug binaries)"
echo "  After that: ~10-30s (incremental)"
echo ""

npm run tauri dev
