//! The Moodist window. A thin client for `moodistd`, which owns all state and audio;
//! this process exists only while the window is open.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};

type Reply = Result<Value, String>;

struct Client {
    writer: Mutex<UnixStream>,
    pending: Mutex<HashMap<u64, mpsc::Sender<Reply>>>,
    next_id: AtomicU64,
    run_in_background: AtomicBool,
}

impl Client {
    fn send(&self, method: &str, params: Value) -> mpsc::Receiver<Reply> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::channel();
        self.pending.lock().unwrap().insert(id, tx);
        let mut line = serde_json::to_vec(&json!({ "id": id, "method": method, "params": params })).unwrap_or_default();
        line.push(b'\n');
        if self.writer.lock().unwrap().write_all(&line).is_err() {
            if let Some(tx) = self.pending.lock().unwrap().remove(&id) {
                let _ = tx.send(Err("Moodist daemon is not running".into()));
            }
        }
        rx
    }
}

fn socket_path() -> PathBuf {
    match std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).filter(|p| p.is_absolute()) {
        Some(dir) => dir.join("moodist.sock"),
        None => std::env::temp_dir().join(format!("moodist-{}.sock", std::env::var("USER").unwrap_or_default())),
    }
}

/// Connects to the daemon, starting it first if needed.
fn connect() -> std::io::Result<UnixStream> {
    let socket = socket_path();
    if let Ok(s) = UnixStream::connect(&socket) {
        return Ok(s);
    }
    let sibling = std::env::current_exe().ok().and_then(|p| Some(p.parent()?.join("moodistd")));
    let program = sibling.filter(|p| p.is_file()).unwrap_or_else(|| PathBuf::from("moodistd"));
    Command::new(program)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        // Own process group: survives this window and the terminal it was started from.
        .process_group(0)
        .spawn()?;
    let mut last = None;
    for _ in 0..100 {
        match UnixStream::connect(&socket) {
            Ok(s) => return Ok(s),
            Err(e) => last = Some(e),
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    Err(last.unwrap())
}

fn spawn_reader(app: AppHandle, stream: UnixStream) {
    std::thread::spawn(move || {
        let client = app.state::<Client>();
        for line in BufReader::new(stream).lines() {
            let Ok(line) = line else { break };
            let Ok(mut msg) = serde_json::from_str::<Value>(&line) else { continue };
            match msg.get("event").and_then(Value::as_str) {
                Some("state") => {
                    let data = msg["data"].take();
                    let bg = data["settings"]["runInBackground"].as_bool().unwrap_or(true);
                    client.run_in_background.store(bg, Ordering::Relaxed);
                    let _ = app.emit("state", data);
                }
                Some("quit") => break,
                _ => {
                    let Some(id) = msg.get("id").and_then(Value::as_u64) else { continue };
                    let reply = match msg.get("error").and_then(Value::as_str) {
                        Some(e) => Err(e.to_string()),
                        None => Ok(msg["result"].take()),
                    };
                    if let Some(tx) = client.pending.lock().unwrap().remove(&id) {
                        let _ = tx.send(reply);
                    }
                }
            }
        }
        // Daemon quit (or crashed): nothing to control, so close too.
        app.exit(0);
    });
}

#[tauri::command]
async fn rpc(client: State<'_, Client>, method: String, params: Option<Value>) -> Reply {
    let rx = client.send(&method, params.unwrap_or(Value::Null));
    rx.recv_timeout(Duration::from_secs(5)).map_err(|_| "Moodist daemon did not respond".to_string())?
}

pub fn run() {
    let stream = match connect() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("moodist: could not reach moodistd: {e}");
            std::process::exit(1);
        }
    };
    let reader = stream.try_clone().expect("clone socket");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.unminimize();
                let _ = win.show();
                let _ = win.set_focus();
            }
        }))
        .manage(Client {
            writer: Mutex::new(stream),
            pending: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            run_in_background: AtomicBool::new(true),
        })
        .invoke_handler(tauri::generate_handler![rpc])
        .setup(move |app| {
            spawn_reader(app.handle().clone(), reader);
            let _ = app.state::<Client>().send("attach", Value::Null);
            WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("Moodist")
                .inner_size(1040.0, 760.0)
                .min_inner_size(420.0, 520.0)
                .background_color(tauri::window::Color(9, 9, 11, 255))
                .build()?;
            Ok(())
        })
        .on_window_event(|win, event| {
            if let WindowEvent::Destroyed = event {
                // A clean close detaches (daemon keeps playing) or quits, per the setting.
                // Wait for the reply so the message lands before this process exits.
                let client = win.state::<Client>();
                let method = if client.run_in_background.load(Ordering::Relaxed) { "detach" } else { "quit" };
                let _ = client.send(method, Value::Null).recv_timeout(Duration::from_millis(500));
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Moodist");
}
