[![Build](https://github.com/qdrx/rinhook/actions/workflows/build.yml/badge.svg)](https://github.com/qdrx/rinhook/actions/workflows/build.yml)
[![npm](https://img.shields.io/npm/v/rinhook)](https://www.npmjs.com/package/@qdrx/rinhook)

# rinhook

Cross-platform global input event listener for Electron — fork of [rdev](https://github.com/Narsil/rdev) with a Wayland bridge and napi-rs wrapper.

| Platform | Backend |
|---|---|
| Windows | rdev (WinAPI hooks) |
| macOS | rdev (CGEventTap) |
| Linux X11 / XWayland | rdev (libinput) |
| Linux Wayland | Unix socket → `rinhook-bridge` |

## Node.js / Electron usage

```bash
npm install rinhook
```

```typescript
import { startListening, stopListening, InputEvent } from 'rinhook'

startListening((event: InputEvent) => {
  console.log(event.type, event.key ?? event.button, event.timestamp)
})

// later
stopListening()
```

### InputEvent

```typescript
interface InputEvent {
  type: 'KeyDown' | 'KeyUp' | 'MouseMove' | 'MouseDown' | 'MouseUp' | 'Wheel'
  key?: string        // e.g. "KeyA", "Space", "ShiftLeft"
  button?: 'Left' | 'Right' | 'Middle'
  x?: number
  y?: number
  deltaX?: number
  deltaY?: number
  timestamp: number   // ms since Unix epoch
}
```

## Linux Wayland — bridge setup

On Wayland, global input capture requires a privileged bridge process.
One-time setup (no root after this):

```bash
# 1. Add yourself to the input group (once, then re-login)
sudo usermod -aG input $USER

# 2. Install bridge + systemd user unit (bundled in the AppImage, or from releases)
./install-bridge.sh
```

The bridge runs as a user systemd service, socket at `$XDG_RUNTIME_DIR/rinhook.sock`.

```bash
systemctl --user status rinhook
journalctl --user -u rinhook -f
```

The socket path can be overridden via `RINHOOK_SOCKET` env var (useful for testing).

## Building from source

```bash
# Bridge binary (Linux only, requires libinput-dev)
cargo build --release --features bridge --bin rinhook-bridge

# napi .node (requires @napi-rs/cli)
cd crates/napi && napi build --platform --release
```

## Testing locally

```bash
# Mock bridge — no root or libinput needed
./test/test_bridge.sh mock

# Real bridge — needs input group membership
./test/test_bridge.sh direct
```

## Upstream

This is a fork of [Narsil/rdev](https://github.com/Narsil/rdev).
To pull upstream changes:

```bash
git fetch upstream
git cherry-pick <commit>
```

## License

MIT — see [LICENSE](LICENSE). Original copyright © Nicolas Patry.
