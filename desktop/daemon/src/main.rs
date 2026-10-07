//! moodistd: the always-on half of Moodist.
//!
//! Owns playback, mixes, playlist, timers and the tray icon, and serves the UI over a
//! Unix socket. It links no GUI toolkit, so running in the background costs only the
//! audio engine. The UI (`moodist`) is a separate process that exits when its window
//! closes.
//!
//! Protocol: newline-delimited JSON.
//!   UI → daemon:  {"id": 1, "method": "toggle_sound", "params": {"id": "rain"}}
//!   daemon → UI:  {"id": 1, "result": ...} | {"id": 1, "error": "..."}
//!                 {"event": "state", "data": {...}} | {"event": "quit"}
//!
//! Lifecycle: closing the window sends "detach" and the daemon keeps playing. If an
//! attached UI vanishes without detaching (force-killed), the daemon exits too.

mod audio;
mod core;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Condvar, Mutex, MutexGuard, OnceLock};
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::core::{now_ms, Core};

struct Daemon {
    core: Mutex<Core>,
    /// Wakes the scheduler when deadlines may have changed.
    wake: Condvar,
    clients: Mutex<Vec<UnixStream>>,
    tray: OnceLock<ksni::blocking::Handle<Tray>>,
    socket: PathBuf,
}

static DAEMON: OnceLock<Daemon> = OnceLock::new();

fn daemon() -> &'static Daemon {
    DAEMON.get().expect("daemon initialised")
}

impl Daemon {
    fn lock(&self) -> MutexGuard<'_, Core> {
        self.core.lock().unwrap()
    }

    /// Pushes state to every connected UI and the tray. Called with the core locked so
    /// clients never see updates out of order.
    fn broadcast(&self, core: &Core) {
        let playing = core.playing;
        if let Some(tray) = self.tray.get() {
            tray.update(|t| {
                if t.playing != playing {
                    t.playing = playing;
                }
            });
        }
        let mut clients = self.clients.lock().unwrap();
        if clients.is_empty() {
            return; // nobody watching; skip serialisation entirely
        }
        let Ok(mut line) = serde_json::to_vec(&json!({ "event": "state", "data": core.snapshot() })) else {
            return;
        };
        line.push(b'\n');
        clients.retain_mut(|c| c.write_all(&line).is_ok());
    }

    fn quit(&self) -> ! {
        let mut core = self.lock();
        core.save();
        core.engine.shutdown();
        for c in self.clients.lock().unwrap().iter_mut() {
            let _ = c.write_all(b"{\"event\":\"quit\"}\n");
        }
        let _ = std::fs::remove_file(&self.socket);
        std::process::exit(0);
    }
}

fn arg<T: DeserializeOwned>(params: &Value, key: &str) -> Result<T, String> {
    serde_json::from_value(params.get(key).cloned().unwrap_or(Value::Null))
        .map_err(|e| format!("bad `{key}`: {e}"))
}

/// Runs one request. Returns the result and whether state should be broadcast.
fn dispatch(core: &mut Core, method: &str, p: &Value) -> Result<(Value, bool), String> {
    let done = |changed| Ok((Value::Null, changed));
    match method {
        "get_state" => Ok((serde_json::to_value(core.snapshot()).unwrap_or_default(), false)),
        "set_playing" => {
            core.set_playing(arg(p, "playing")?);
            done(true)
        }
        "toggle_sound" => {
            core.toggle_sound(&arg::<String>(p, "id")?);
            done(true)
        }
        // Hot path while dragging a slider: the UI already shows the value.
        "set_sound" => {
            let setting = core::SoundSetting { volume: arg(p, "volume")?, swell: arg(p, "swell")? };
            core.set_sound(&arg::<String>(p, "id")?, setting);
            done(false)
        }
        "replace_sounds" => {
            core.replace_sounds(arg(p, "sounds")?, arg(p, "play")?);
            done(true)
        }
        "shuffle_sounds" => {
            core.shuffle_sounds();
            done(true)
        }
        "set_favorite" => {
            core.set_favorite(&arg::<String>(p, "id")?, arg(p, "favorite")?);
            done(true)
        }
        "save_mix" => {
            let mix = core.save_mix(arg(p, "name")?, arg(p, "id")?);
            Ok((serde_json::to_value(mix).unwrap_or_default(), true))
        }
        "delete_mix" => {
            core.delete_mix(&arg::<String>(p, "id")?);
            done(true)
        }
        "rename_mix" => {
            core.rename_mix(&arg::<String>(p, "id")?, &arg::<String>(p, "name")?);
            done(true)
        }
        "play_mix" => {
            core.play_mix(&arg::<String>(p, "id")?);
            done(true)
        }
        "playlist_add" => {
            core.playlist_add(arg(p, "kind")?, arg(p, "target")?, arg(p, "minutes")?);
            done(true)
        }
        "playlist_remove" => {
            core.playlist_remove(arg(p, "id")?);
            done(true)
        }
        "playlist_set_minutes" => {
            core.playlist_set_minutes(arg(p, "id")?, arg(p, "minutes")?);
            done(true)
        }
        "playlist_move" => {
            core.playlist_move(arg(p, "id")?, arg(p, "to")?);
            done(true)
        }
        "playlist_clear" => {
            core.playlist_clear();
            done(true)
        }
        "playlist_options" => {
            core.playlist_options(arg(p, "shuffle")?, arg(p, "looping")?);
            done(true)
        }
        "playlist_start" => {
            core.playlist_start(arg(p, "from")?);
            done(true)
        }
        "playlist_stop" => {
            core.stop_playlist();
            done(true)
        }
        "playlist_skip" => {
            core.playlist_skip(arg(p, "delta")?);
            done(true)
        }
        "sleep_start" => {
            core.sleep_start(arg(p, "minutes")?);
            done(true)
        }
        "sleep_cancel" => {
            core.sleep_cancel();
            done(true)
        }
        "timer_add" => {
            core.timer_add(arg(p, "label")?, arg(p, "minutes")?);
            done(true)
        }
        "timer_toggle" => {
            core.timer_toggle(arg(p, "id")?);
            done(true)
        }
        "timer_reset" => {
            core.timer_reset(arg(p, "id")?);
            done(true)
        }
        "timer_remove" => {
            core.timer_remove(arg(p, "id")?);
            done(true)
        }
        "set_settings" => {
            core.set_settings(arg(p, "settings")?);
            done(true)
        }
        "test_alarm" => {
            core.play_alarm();
            done(false)
        }
        _ => Err(format!("unknown method `{method}`")),
    }
}

fn handle_client(stream: UnixStream) {
    let d = daemon();
    // A UI that stops reading must not be able to stall playback.
    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
    let Ok(mut writer) = stream.try_clone() else { return };
    if let Ok(w) = stream.try_clone() {
        d.clients.lock().unwrap().push(w);
    }

    // A UI "attaches"; if it later disconnects without "detach" it was killed or
    // crashed, which we treat as a force quit of the whole app.
    let (mut attached, mut detached) = (false, false);

    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else { break };
        let Ok(req) = serde_json::from_str::<Value>(&line) else { continue };
        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let method = req.get("method").and_then(Value::as_str).unwrap_or_default();
        let params = req.get("params").cloned().unwrap_or(Value::Null);

        let response = match method {
            "quit" => d.quit(),
            "attach" | "detach" => {
                attached |= method == "attach";
                detached |= method == "detach";
                json!({ "id": id, "result": null })
            }
            // Talks to the sound server; don't hold the core lock while it does.
            "list_devices" => json!({ "id": id, "result": audio::list_devices() }),
            "open" => {
                open_ui();
                json!({ "id": id, "result": null })
            }
            _ => {
                let mut core = d.lock();
                let res = dispatch(&mut core, method, &params);
                if let Ok((_, true)) = res {
                    d.broadcast(&core);
                }
                drop(core);
                d.wake.notify_all();
                match res {
                    Ok((v, _)) => json!({ "id": id, "result": v }),
                    Err(e) => json!({ "id": id, "error": e }),
                }
            }
        };
        let mut out = serde_json::to_vec(&response).unwrap_or_default();
        out.push(b'\n');
        // Same lock as broadcasts, so a response never interleaves with a state event.
        let sent = {
            let _guard = d.clients.lock().unwrap();
            writer.write_all(&out)
        };
        if sent.is_err() {
            break;
        }
    }

    if attached && !detached {
        d.quit();
    }
}

/// Single thread that sleeps until the next deadline (playlist step, timer, sleep fade, save).
fn run_scheduler() {
    let d = daemon();
    let mut core = d.lock();
    loop {
        let (changed, next) = core.tick();
        if changed {
            d.broadcast(&core);
        }
        core = match next {
            Some(at) => {
                let wait = Duration::from_millis(at.saturating_sub(now_ms()).max(1));
                d.wake.wait_timeout(core, wait).unwrap().0
            }
            None => d.wake.wait(core).unwrap(),
        };
    }
}

/// Launches the UI. If it's already open, its single-instance guard focuses it instead.
fn open_ui() {
    let sibling = std::env::current_exe().ok().and_then(|p| Some(p.parent()?.join("moodist")));
    let program = sibling.filter(|p| p.is_file()).unwrap_or_else(|| PathBuf::from("moodist"));
    if let Ok(mut child) = Command::new(program)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
    {
        // Reap it when it exits so it doesn't linger as a zombie.
        std::thread::spawn(move || child.wait());
    }
}

struct Tray {
    playing: bool,
    icons: Vec<ksni::Icon>,
}

impl ksni::Tray for Tray {
    fn id(&self) -> String {
        "moodist".into()
    }

    fn title(&self) -> String {
        "Moodist".into()
    }

    fn icon_name(&self) -> String {
        "moodist".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        self.icons.clone()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        open_ui();
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;
        vec![
            StandardItem { label: "Open Moodist".into(), activate: Box::new(|_| open_ui()), ..Default::default() }.into(),
            StandardItem {
                label: if self.playing { "Pause" } else { "Play" }.into(),
                activate: Box::new(|_| {
                    let d = daemon();
                    let mut core = d.lock();
                    let playing = !core.playing;
                    core.set_playing(playing);
                    d.broadcast(&core);
                    drop(core);
                    d.wake.notify_all();
                }),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem { label: "Quit".into(), activate: Box::new(|_| daemon().quit()), ..Default::default() }.into(),
        ]
    }
}

fn tray_icons() -> Vec<ksni::Icon> {
    let pngs: [&[u8]; 2] = [include_bytes!("../../src-tauri/icons/32x32.png"), include_bytes!("../../src-tauri/icons/64x64.png")];
    pngs.iter()
        .filter_map(|bytes| {
            let mut decoder = png::Decoder::new(*bytes);
            decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
            let mut reader = decoder.read_info().ok()?;
            let mut buf = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut buf).ok()?;
            if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
                return None;
            }
            // SNI wants ARGB32 in network byte order.
            let data = buf[..info.buffer_size()].chunks_exact(4).flat_map(|p| [p[3], p[0], p[1], p[2]]).collect();
            Some(ksni::Icon { width: info.width as i32, height: info.height as i32, data })
        })
        .collect()
}

fn config_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("moodist").join("state.json")
}

pub fn socket_path() -> PathBuf {
    match std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).filter(|p| p.is_absolute()) {
        Some(dir) => dir.join("moodist.sock"),
        None => std::env::temp_dir().join(format!("moodist-{}.sock", std::env::var("USER").unwrap_or_default())),
    }
}

fn sounds_dir() -> PathBuf {
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from));
    let candidates = [
        std::env::var_os("MOODIST_SOUNDS").map(PathBuf::from),
        exe_dir.as_ref().map(|d| d.join("../share/moodist/sounds")),
        Some(PathBuf::from("/usr/share/moodist/sounds")),
        Some(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../public/sounds"))),
    ];
    candidates
        .into_iter()
        .flatten()
        .find(|p| p.join("alarm.mp3").is_file())
        .unwrap_or_else(|| PathBuf::from("/usr/share/moodist/sounds"))
}

/// Blocks termination signals in every thread and handles them on one, so logout or
/// `kill` saves state and closes the UI instead of dying mid-write.
fn handle_signals() {
    // SAFETY: plain libc signal-mask calls on a zeroed sigset; run before other threads exist.
    unsafe {
        let mut set: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut set);
        for sig in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] {
            libc::sigaddset(&mut set, sig);
        }
        libc::pthread_sigmask(libc::SIG_BLOCK, &set, std::ptr::null_mut());
        std::thread::spawn(move || {
            let mut sig = 0;
            libc::sigwait(&set, &mut sig);
            match DAEMON.get() {
                Some(d) => d.quit(),
                None => std::process::exit(0),
            }
        });
    }
}

fn main() {
    let socket = socket_path();
    if UnixStream::connect(&socket).is_ok() {
        // Already running. `moodistd --open` doubles as "show the window".
        if std::env::args().any(|a| a == "--open") {
            if let Ok(mut s) = UnixStream::connect(&socket) {
                let _ = s.write_all(b"{\"id\":0,\"method\":\"open\"}\n");
            }
        }
        return;
    }
    let _ = std::fs::remove_file(&socket); // stale socket from a crash
    let listener = match UnixListener::bind(&socket) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("moodistd: cannot bind {}: {e}", socket.display());
            std::process::exit(1);
        }
    };

    handle_signals();
    let core = Core::new(config_path(), &sounds_dir());
    let _ = DAEMON.set(Daemon {
        core: Mutex::new(core),
        wake: Condvar::new(),
        clients: Mutex::new(Vec::new()),
        tray: OnceLock::new(),
        socket,
    });

    std::thread::Builder::new().name("scheduler".into()).spawn(run_scheduler).expect("spawn scheduler");

    // The tray is optional: without a StatusNotifier host the daemon still works.
    use ksni::blocking::TrayMethods;
    match (Tray { playing: false, icons: tray_icons() }).spawn() {
        Ok(handle) => {
            let _ = daemon().tray.set(handle);
        }
        Err(e) => eprintln!("moodistd: tray unavailable: {e}"),
    }

    if std::env::args().any(|a| a == "--open") {
        open_ui();
    }

    for stream in listener.incoming().flatten() {
        let _ = std::thread::Builder::new().name("client".into()).spawn(move || handle_client(stream));
    }
}
