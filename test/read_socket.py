#!/usr/bin/env python3
"""
Connect to the rinhook socket and print events.
Works with both the real bridge (/run/rinhook.sock)
and the mock bridge (/tmp/rinhook.sock).

Usage:
    python3 test/read_socket.py [/run/rinhook.sock | /tmp/rinhook.sock]
"""
import json
import socket
import sys
import time

SOCKET_PATH = sys.argv[1] if len(sys.argv) > 1 else "/run/rinhook.sock"

COLORS = {
    "KeyDown":   "\033[32m",   # green
    "KeyUp":     "\033[36m",   # cyan
    "MouseDown": "\033[33m",   # yellow
    "MouseUp":   "\033[33m",
    "MouseMove": "\033[90m",   # dark grey (noisy, dim it)
    "Wheel":     "\033[35m",   # magenta
}
RESET = "\033[0m"


def format_event(ev: dict) -> str:
    t = ev.get("type", "?")
    color = COLORS.get(t, "")
    ts = ev.get("timestamp", 0)
    ts_s = time.strftime("%H:%M:%S", time.localtime(ts / 1000)) + f".{ts % 1000:03d}"

    parts = [f"{color}{t:<11}{RESET}", ts_s]

    if "key" in ev:
        parts.append(f"key={ev['key']}")
    if "button" in ev:
        parts.append(f"button={ev['button']}")
    if "x" in ev:
        parts.append(f"x={ev['x']:.1f} y={ev['y']:.1f}")
    if "deltaX" in ev:
        parts.append(f"Δx={ev['deltaX']} Δy={ev['deltaY']}")

    return "  ".join(parts)


def main():
    print(f"Connecting to {SOCKET_PATH} …")
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    try:
        sock.connect(SOCKET_PATH)
    except FileNotFoundError:
        print(f"ERROR: socket not found at {SOCKET_PATH}")
        print("  → start the mock:  python3 test/mock_bridge.py /tmp/rinhook.sock")
        print("  → or the real bridge:  sudo ./target/release/rinhook-bridge")
        sys.exit(1)
    except PermissionError:
        print(f"ERROR: permission denied on {SOCKET_PATH}")
        sys.exit(1)

    print(f"Connected. Listening for events (Ctrl-C to stop)…\n")

    buf = b""
    try:
        while True:
            chunk = sock.recv(4096)
            if not chunk:
                print("Connection closed by server.")
                break
            buf += chunk
            while b"\n" in buf:
                line, buf = buf.split(b"\n", 1)
                try:
                    ev = json.loads(line.decode())
                    print(format_event(ev))
                except json.JSONDecodeError:
                    print(f"  [bad JSON] {line!r}")
    except KeyboardInterrupt:
        print("\nStopped.")
    finally:
        sock.close()


if __name__ == "__main__":
    main()
