use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use rdev::{Button, Event, EventType, Key, listen};
use serde::Serialize;

fn socket_path() -> String {
    std::env::var("RINHOOK_SOCKET").unwrap_or_else(|_| {
        let runtime_dir =
            std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".to_string());
        format!("{runtime_dir}/rinhook.sock")
    })
}

#[derive(Serialize)]
struct BridgeEvent {
    #[serde(rename = "type")]
    event_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    button: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    x: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    y: Option<f64>,
    #[serde(rename = "deltaX", skip_serializing_if = "Option::is_none")]
    delta_x: Option<f64>,
    #[serde(rename = "deltaY", skip_serializing_if = "Option::is_none")]
    delta_y: Option<f64>,
    timestamp: u64,
    #[serde(rename = "virtual")]
    is_virtual: bool,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn key_name(key: Key) -> String {
    format!("{key:?}")
}

fn button_str(button: Button) -> Option<&'static str> {
    match button {
        Button::Left => Some("Left"),
        Button::Right => Some("Right"),
        Button::Middle => Some("Middle"),
        Button::Unknown(_) => None,
    }
}

fn to_bridge_event(event: Event) -> Option<BridgeEvent> {
    let ts = now_ms();
    let is_virtual = event.is_virtual;
    match event.event_type {
        EventType::KeyPress(key) => Some(BridgeEvent {
            event_type: "KeyDown",
            key: Some(key_name(key)),
            button: None,
            x: None,
            y: None,
            delta_x: None,
            delta_y: None,
            timestamp: ts,
            is_virtual,
        }),
        EventType::KeyRelease(key) => Some(BridgeEvent {
            event_type: "KeyUp",
            key: Some(key_name(key)),
            button: None,
            x: None,
            y: None,
            delta_x: None,
            delta_y: None,
            timestamp: ts,
            is_virtual,
        }),
        EventType::ButtonPress(btn) => button_str(btn).map(|b| BridgeEvent {
            event_type: "MouseDown",
            key: None,
            button: Some(b),
            x: None,
            y: None,
            delta_x: None,
            delta_y: None,
            timestamp: ts,
            is_virtual,
        }),
        EventType::ButtonRelease(btn) => button_str(btn).map(|b| BridgeEvent {
            event_type: "MouseUp",
            key: None,
            button: Some(b),
            x: None,
            y: None,
            delta_x: None,
            delta_y: None,
            timestamp: ts,
            is_virtual,
        }),
        EventType::MouseMove { x, y } => Some(BridgeEvent {
            event_type: "MouseMove",
            key: None,
            button: None,
            x: Some(x),
            y: Some(y),
            delta_x: None,
            delta_y: None,
            timestamp: ts,
            is_virtual,
        }),
        EventType::Wheel { delta_x, delta_y } => Some(BridgeEvent {
            event_type: "Wheel",
            key: None,
            button: None,
            x: None,
            y: None,
            delta_x: Some(delta_x as f64),
            delta_y: Some(delta_y as f64),
            timestamp: ts,
            is_virtual,
        }),
    }
}

fn main() {
    let sock = socket_path();

    let _ = std::fs::remove_file(&sock);

    let listener =
        UnixListener::bind(&sock).unwrap_or_else(|e| panic!("Failed to bind {sock}: {e}"));

    // World-readable so any user in the session can connect.
    std::fs::set_permissions(&sock, std::fs::Permissions::from_mode(0o666))
        .expect("Failed to set socket permissions");

    let clients: Arc<Mutex<Vec<std::os::unix::net::UnixStream>>> = Arc::new(Mutex::new(Vec::new()));

    let accept_clients = clients.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(s) => accept_clients.lock().unwrap().push(s),
                Err(e) => {
                    eprintln!("rinhook-bridge: accept error: {e}");
                    break;
                }
            }
        }
    });

    eprintln!("rinhook-bridge: listening on {sock}");

    listen(move |event| {
        let Some(bridge_event) = to_bridge_event(event) else {
            return;
        };
        let Ok(mut line) = serde_json::to_string(&bridge_event) else {
            return;
        };
        line.push('\n');

        let mut locked = clients.lock().unwrap();
        locked.retain_mut(|client| client.write_all(line.as_bytes()).is_ok());
    })
    .expect("rinhook-bridge: listen failed");
}
