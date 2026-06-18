#!/usr/bin/env python3
"""
Mock bridge — sends fake input events over a Unix socket.
Simulates what rinhook-bridge does for the real bridge,
so you can test the napi socket reader without root/libinput.

Usage:
    python3 test/mock_bridge.py [/tmp/rinhook.sock]
"""
import json
import os
import socket
import sys
import threading
import time

SOCKET_PATH = sys.argv[1] if len(sys.argv) > 1 else "/tmp/rinhook.sock"

FAKE_EVENTS = [
    {"type": "KeyDown", "key": "KeyA", "timestamp": 0},
    {"type": "KeyUp",   "key": "KeyA", "timestamp": 0},
    {"type": "MouseMove", "x": 100.0, "y": 200.0, "timestamp": 0},
    {"type": "MouseMove", "x": 150.0, "y": 220.0, "timestamp": 0},
    {"type": "MouseDown", "button": "Left", "timestamp": 0},
    {"type": "MouseUp",   "button": "Left", "timestamp": 0},
    {"type": "KeyDown", "key": "Space", "timestamp": 0},
    {"type": "KeyUp",   "key": "Space", "timestamp": 0},
    {"type": "Wheel", "deltaX": 0.0, "deltaY": -1.0, "timestamp": 0},
    {"type": "KeyDown", "key": "ShiftLeft", "timestamp": 0},
    {"type": "KeyDown", "key": "KeyS",      "timestamp": 0},
    {"type": "KeyUp",   "key": "KeyS",      "timestamp": 0},
    {"type": "KeyUp",   "key": "ShiftLeft", "timestamp": 0},
]

clients = []
clients_lock = threading.Lock()


def client_handler(conn, addr):
    print(f"  [mock] client connected")
    with clients_lock:
        clients.append(conn)
    # Keep socket open — broadcaster handles writes
    try:
        conn.recv(1)  # block until client disconnects
    except Exception:
        pass
    with clients_lock:
        if conn in clients:
            clients.remove(conn)
    print(f"  [mock] client disconnected")


def broadcaster():
    idx = 0
    while True:
        time.sleep(0.3)
        event = dict(FAKE_EVENTS[idx % len(FAKE_EVENTS)])
        event["timestamp"] = int(time.time() * 1000)
        line = json.dumps(event) + "\n"
        idx += 1

        with clients_lock:
            dead = []
            for c in clients:
                try:
                    c.sendall(line.encode())
                except Exception:
                    dead.append(c)
            for c in dead:
                clients.remove(c)


def main():
    try:
        os.unlink(SOCKET_PATH)
    except FileNotFoundError:
        pass

    srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    srv.bind(SOCKET_PATH)
    os.chmod(SOCKET_PATH, 0o666)
    srv.listen(8)

    print(f"mock_bridge listening on {SOCKET_PATH}")
    print("Events are sent every 300ms. Ctrl-C to stop.\n")

    t = threading.Thread(target=broadcaster, daemon=True)
    t.start()

    try:
        while True:
            conn, addr = srv.accept()
            threading.Thread(target=client_handler, args=(conn, addr), daemon=True).start()
    except KeyboardInterrupt:
        print("\nStopped.")
    finally:
        srv.close()
        try:
            os.unlink(SOCKET_PATH)
        except FileNotFoundError:
            pass


if __name__ == "__main__":
    main()
