#!/usr/bin/env bash
# ====== Tie - Build & Launch ======
# Usage:  ./build.sh            (build + launch)
#         ./build.sh --skip     (launch only)

set -euo pipefail
cd "$(dirname "$0")")

APP_NAME="tie"
SKIP_BUILD=false

if [[ "${1:-}" == "--skip" ]]; then
    SKIP_BUILD=true
fi

EXE_PATH="./src-tauri/target/release/$APP_NAME"

echo ""
echo "========== Tie =========="

# 1. Kill old process
echo ""
echo "[1/4] Stopping old process..."
if pkill -x "$APP_NAME" 2>/dev/null; then
    echo "  Stopped old process"
    sleep 2
else
    echo "  No running process"
fi

# 2. Delete old binary
echo ""
echo "[2/4] Cleaning old build..."
if [[ -f "$EXE_PATH" ]]; then
    rm -f "$EXE_PATH"
    echo "  Deleted old binary"
else
    echo "  No old binary"
fi

# 3. Build
if [[ "$SKIP_BUILD" == false ]]; then
    echo ""
    echo "[3/4] Building (may take 8-15 min)..."
    if npx tauri build --no-bundle 2>&1 | grep -E "Built application|error|Error|Finished|Compiling"; then
        echo "  Build OK"
    else
        echo ""
        echo "  BUILD FAILED!"
        echo "  Run manually: npx tauri build --no-bundle"
        exit 1
    fi
else
    echo ""
    echo "[3/4] Skipped (--skip)"
fi

# 4. Launch
echo ""
echo "[4/4] Launching..."
if [[ -f "$EXE_PATH" ]]; then
    "$EXE_PATH" &
    echo "  Started: $EXE_PATH"
else
    echo "  Binary not found, build first"
    exit 1
fi

echo ""
echo "========== Done =========="
echo "Tip: launch only -> ./build.sh --skip"
echo ""
