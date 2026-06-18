#!/usr/bin/env bash
# Test runner for rinhook-bridge.
#
# Modes:
#   mock     — Python mock server, no root/libinput needed     (default)
#   direct   — run bridge binary directly (needs 'input' group)
#   service  — install + start as user systemd unit
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
BRIDGE="$ROOT_DIR/target/release/rinhook-bridge"
MODE="${1:-mock}"

# ── build if needed ───────────────────────────────────────────────────────────

if [[ "$MODE" != "mock" && ! -x "$BRIDGE" ]]; then
    echo "Building rinhook-bridge …"
    cd "$ROOT_DIR"
    cargo build --release --features bridge --bin rinhook-bridge
fi

# ── mock: fake events, no libinput required ───────────────────────────────────

if [[ "$MODE" == "mock" ]]; then
    SOCK="/tmp/rinhook-test.sock"
    echo "=== MOCK MODE (no root / no libinput required) ==="
    echo ""
    python3 -u "$SCRIPT_DIR/mock_bridge.py" "$SOCK" &
    MOCK_PID=$!
    trap "kill $MOCK_PID 2>/dev/null; rm -f $SOCK" EXIT
    sleep 0.3
    python3 -u "$SCRIPT_DIR/read_socket.py" "$SOCK"
    exit 0
fi

# ── direct: run bridge binary in foreground ───────────────────────────────────

if [[ "$MODE" == "direct" ]]; then
    if ! groups | grep -qw input; then
        echo "ERROR: you must be in the 'input' group."
        echo "  sudo usermod -aG input $USER   (then re-login)"
        exit 1
    fi
    SOCK="${XDG_RUNTIME_DIR:-/tmp}/rinhook-test.sock"
    echo "=== DIRECT MODE (input group: OK) ==="
    echo "Socket: $SOCK"
    echo ""
    RINHOOK_SOCKET="$SOCK" "$BRIDGE" &
    BRIDGE_PID=$!
    trap "kill $BRIDGE_PID 2>/dev/null; rm -f $SOCK" EXIT
    sleep 0.3
    echo "Bridge running (PID $BRIDGE_PID). Move the mouse / press keys."
    echo ""
    python3 -u "$SCRIPT_DIR/read_socket.py" "$SOCK"
    exit 0
fi

# ── service: install as user systemd unit ────────────────────────────────────

if [[ "$MODE" == "service" ]]; then
    echo "=== SERVICE MODE ==="
    install -m 755 "$BRIDGE" "$HOME/.local/bin/rinhook-bridge"
    bash "$ROOT_DIR/packaging/install-bridge.sh" "$BRIDGE"
    SOCK="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/rinhook.sock"
    sleep 0.5
    python3 -u "$SCRIPT_DIR/read_socket.py" "$SOCK"
    exit 0
fi

echo "Usage: $0 [mock|direct|service]"
exit 1
