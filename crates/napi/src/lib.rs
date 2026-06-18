#![deny(clippy::all)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use napi::bindgen_prelude::*;
use napi::threadsafe_function::{
    ErrorStrategy, ThreadSafeCallContext, ThreadsafeFunction, ThreadsafeFunctionCallMode,
};
use napi::{JsFunction, Result};
use napi_derive::napi;

// ── internal event type (not exposed to JS directly) ─────────────────────────

struct InputEvent {
    event_type: String,
    key: Option<String>,
    button: Option<String>,
    x: Option<f64>,
    y: Option<f64>,
    delta_x: Option<f64>,
    delta_y: Option<f64>,
    timestamp: f64,
}

// ── global listener state ─────────────────────────────────────────────────────

struct State {
    stop: Arc<AtomicBool>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

// ── helpers ───────────────────────────────────────────────────────────────────

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

fn build_js_event(
    ctx: ThreadSafeCallContext<InputEvent>,
) -> Result<Vec<napi::JsUnknown>> {
    let env = ctx.env;
    let ev = ctx.value;
    let mut obj = env.create_object()?;
    obj.set_named_property("type", env.create_string(&ev.event_type)?)?;
    if let Some(k) = ev.key {
        obj.set_named_property("key", env.create_string(&k)?)?;
    }
    if let Some(b) = ev.button {
        obj.set_named_property("button", env.create_string(&b)?)?;
    }
    if let Some(x) = ev.x {
        obj.set_named_property("x", env.create_double(x)?)?;
    }
    if let Some(y) = ev.y {
        obj.set_named_property("y", env.create_double(y)?)?;
    }
    if let Some(dx) = ev.delta_x {
        obj.set_named_property("deltaX", env.create_double(dx)?)?;
    }
    if let Some(dy) = ev.delta_y {
        obj.set_named_property("deltaY", env.create_double(dy)?)?;
    }
    obj.set_named_property("timestamp", env.create_double(ev.timestamp)?)?;
    Ok(vec![obj.into_unknown()])
}

// ── rdev → InputEvent conversion ─────────────────────────────────────────────

fn convert_rdev(event: rdev::Event) -> Option<InputEvent> {
    let ts = now_ms();
    match event.event_type {
        rdev::EventType::KeyPress(key) => Some(InputEvent {
            event_type: "KeyDown".into(),
            key: Some(format!("{key:?}")),
            button: None,
            x: None,
            y: None,
            delta_x: None,
            delta_y: None,
            timestamp: ts,
        }),
        rdev::EventType::KeyRelease(key) => Some(InputEvent {
            event_type: "KeyUp".into(),
            key: Some(format!("{key:?}")),
            button: None,
            x: None,
            y: None,
            delta_x: None,
            delta_y: None,
            timestamp: ts,
        }),
        rdev::EventType::ButtonPress(btn) => {
            let button = btn_name(btn)?;
            Some(InputEvent {
                event_type: "MouseDown".into(),
                key: None,
                button: Some(button.into()),
                x: None,
                y: None,
                delta_x: None,
                delta_y: None,
                timestamp: ts,
            })
        }
        rdev::EventType::ButtonRelease(btn) => {
            let button = btn_name(btn)?;
            Some(InputEvent {
                event_type: "MouseUp".into(),
                key: None,
                button: Some(button.into()),
                x: None,
                y: None,
                delta_x: None,
                delta_y: None,
                timestamp: ts,
            })
        }
        rdev::EventType::MouseMove { x, y } => Some(InputEvent {
            event_type: "MouseMove".into(),
            key: None,
            button: None,
            x: Some(x),
            y: Some(y),
            delta_x: None,
            delta_y: None,
            timestamp: ts,
        }),
        rdev::EventType::Wheel { delta_x, delta_y } => Some(InputEvent {
            event_type: "Wheel".into(),
            key: None,
            button: None,
            x: None,
            y: None,
            delta_x: Some(delta_x as f64),
            delta_y: Some(delta_y as f64),
            timestamp: ts,
        }),
    }
}

fn btn_name(btn: rdev::Button) -> Option<&'static str> {
    match btn {
        rdev::Button::Left => Some("Left"),
        rdev::Button::Right => Some("Right"),
        rdev::Button::Middle => Some("Middle"),
        rdev::Button::Unknown(_) => None,
    }
}

// ── platform detection ────────────────────────────────────────────────────────

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
fn is_wayland() -> bool {
    std::env::var("XDG_SESSION_TYPE")
        .map(|s| s.eq_ignore_ascii_case("wayland"))
        .unwrap_or(false)
}

// ── listener threads ──────────────────────────────────────────────────────────

fn rdev_listener(
    tsfn: ThreadsafeFunction<InputEvent, ErrorStrategy::CalleeHandled>,
    stop: Arc<AtomicBool>,
) {
    rdev::listen(move |event| {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        if let Some(ev) = convert_rdev(event) {
            tsfn.call(Ok(ev), ThreadsafeFunctionCallMode::NonBlocking);
        }
    })
    .ok();
}

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
fn socket_listener(
    tsfn: ThreadsafeFunction<InputEvent, ErrorStrategy::CalleeHandled>,
    stop: Arc<AtomicBool>,
) {
    use std::io::Read;
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    let socket_path = std::env::var("RINHOOK_SOCKET")
        .unwrap_or_else(|_| "/run/rinhook.sock".to_string());

    // Retry connecting until the bridge is available or we're asked to stop.
    let mut stream = loop {
        match UnixStream::connect(&socket_path) {
            Ok(s) => break s,
            Err(_) => {
                if stop.load(Ordering::Relaxed) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    };
    stream
        .set_read_timeout(Some(Duration::from_millis(100)))
        .ok();

    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    let mut tmp = [0u8; 4096];

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                // Process all complete newline-delimited JSON lines.
                while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                    let line_bytes: Vec<u8> = buf.drain(..pos).collect();
                    buf.drain(..1); // consume the '\n'
                    if let Ok(line) = std::str::from_utf8(&line_bytes) {
                        if let Some(ev) = parse_socket_event(line) {
                            tsfn.call(Ok(ev), ThreadsafeFunctionCallMode::NonBlocking);
                        }
                    }
                }
            }
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                continue;
            }
            Err(_) => break,
        }
    }
}

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
fn parse_socket_event(line: &str) -> Option<InputEvent> {
    #[derive(serde::Deserialize)]
    struct SocketEvent {
        #[serde(rename = "type")]
        event_type: String,
        key: Option<String>,
        button: Option<String>,
        x: Option<f64>,
        y: Option<f64>,
        #[serde(rename = "deltaX")]
        delta_x: Option<f64>,
        #[serde(rename = "deltaY")]
        delta_y: Option<f64>,
        timestamp: f64,
    }
    let se: SocketEvent = serde_json::from_str(line).ok()?;
    Some(InputEvent {
        event_type: se.event_type,
        key: se.key,
        button: se.button,
        x: se.x,
        y: se.y,
        delta_x: se.delta_x,
        delta_y: se.delta_y,
        timestamp: se.timestamp,
    })
}

// ── public API ────────────────────────────────────────────────────────────────

/// Start listening for global input events.
/// The callback receives one `InputEvent` object per event.
/// Throws if already listening.
#[napi]
pub fn start_listening(callback: JsFunction) -> Result<()> {
    let mut state = STATE
        .lock()
        .map_err(|_| napi::Error::from_reason("STATE lock poisoned"))?;

    if state.is_some() {
        return Err(napi::Error::from_reason("Already listening"));
    }

    let stop = Arc::new(AtomicBool::new(false));
    *state = Some(State { stop: stop.clone() });
    drop(state);

    let tsfn: ThreadsafeFunction<InputEvent, ErrorStrategy::CalleeHandled> =
        callback.create_threadsafe_function(0, build_js_event)?;

    #[cfg(all(target_family = "unix", not(target_os = "macos")))]
    if is_wayland() {
        let tsfn_clone = tsfn.clone();
        std::thread::spawn(move || socket_listener(tsfn_clone, stop));
        return Ok(());
    }

    let tsfn_clone = tsfn.clone();
    std::thread::spawn(move || rdev_listener(tsfn_clone, stop));

    Ok(())
}

/// Stop listening. Safe to call when not listening.
#[napi]
pub fn stop_listening() {
    if let Ok(mut state) = STATE.lock() {
        if let Some(s) = state.take() {
            s.stop.store(true, Ordering::Relaxed);
        }
    }
}
