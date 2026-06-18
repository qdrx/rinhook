#!/usr/bin/env bash
# Post-install for rinhook on Linux Wayland.
# Called by the Electron app on first launch.
#
# What this does (zero-sudo after the one-time group add):
#   1. Check user is in 'input' group (needed for libinput).
#      If not → print the one-time sudo command and exit 2.
#   2. Install bridge binary  → ~/.local/bin/rinhook-bridge
#   3. Install systemd unit   → ~/.config/systemd/user/rinhook.service
#   4. Enable + start the unit.
#
# Usage:
#   install-bridge.sh                  # auto-detect binary next to this script
#   install-bridge.sh /path/to/binary  # explicit binary path
#   install-bridge.sh --download       # download latest from GitHub releases
set -euo pipefail

REPO="qdrx/rinhook"
BIN_DST="$HOME/.local/bin/rinhook-bridge"
UNIT_DST="$HOME/.config/systemd/user/rinhook.service"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ── helpers ───────────────────────────────────────────────────────────────────

die()  { echo "ERROR: $*" >&2; exit 1; }
info() { echo "  → $*"; }

check_input_group() {
    if ! groups | grep -qw input; then
        echo ""
        echo "  rinhook needs access to /dev/input/* (libinput)."
        echo "  You must be in the 'input' group. Run this once, then re-login:"
        echo ""
        echo "      sudo usermod -aG input $USER"
        echo ""
        echo "  After re-login, re-run this installer."
        echo ""
        exit 2
    fi
}

detect_arch() {
    case "$(uname -m)" in
        x86_64)  echo "x86_64" ;;
        aarch64) echo "aarch64" ;;
        *) die "Unsupported architecture: $(uname -m)" ;;
    esac
}

latest_tag() {
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' \
        | sed -E 's/.*"([^"]+)".*/\1/'
}

# ── step 1: check group ───────────────────────────────────────────────────────

echo "Checking 'input' group membership …"
check_input_group
info "OK — user is in the input group"

# ── step 2: get binary ────────────────────────────────────────────────────────

mkdir -p "$HOME/.local/bin"

MODE="${1:-auto}"

if [[ "$MODE" == "--download" ]]; then
    ARCH=$(detect_arch)
    TAG=$(latest_tag)
    [[ -n "$TAG" ]] || die "Could not determine latest release tag"
    URL="https://github.com/${REPO}/releases/download/${TAG}/rinhook-bridge-linux-${ARCH}"
    echo "Downloading rinhook-bridge ${TAG} (${ARCH}) …"
    curl -fsSL "$URL" -o "$BIN_DST"
    chmod 755 "$BIN_DST"
    info "Downloaded to $BIN_DST"
elif [[ "$MODE" != "auto" && -f "$MODE" ]]; then
    # Explicit path passed (Electron app extracts binary from AppImage and passes path)
    install -m 755 "$MODE" "$BIN_DST"
    info "Installed binary from $MODE"
else
    # Auto: look for binary next to this script (bundled in AppImage resources)
    BUNDLED="$SCRIPT_DIR/rinhook-bridge"
    if [[ -f "$BUNDLED" ]]; then
        install -m 755 "$BUNDLED" "$BIN_DST"
        info "Installed bundled binary"
    else
        die "No binary found. Pass a path or use --download"
    fi
fi

# ── step 3: install systemd user unit ────────────────────────────────────────

mkdir -p "$(dirname "$UNIT_DST")"

cat > "$UNIT_DST" <<'EOF'
[Unit]
Description=rinhook input event bridge
After=default.target

[Service]
Type=simple
Environment=RINHOOK_SOCKET=%t/rinhook.sock
ExecStart=%h/.local/bin/rinhook-bridge
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
EOF

info "Installed systemd user unit to $UNIT_DST"

# ── step 4: enable + start ────────────────────────────────────────────────────

systemctl --user daemon-reload
systemctl --user enable --now rinhook
info "Service enabled and started"

# ── done ──────────────────────────────────────────────────────────────────────

SOCK_PATH="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/rinhook.sock"
echo ""
echo "Done! Socket: $SOCK_PATH"
echo "Status: systemctl --user status rinhook"
echo "Logs:   journalctl --user -u rinhook -f"
