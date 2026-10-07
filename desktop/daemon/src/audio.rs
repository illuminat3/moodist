//! Audio engine: a small software mixer feeding the platform's sound server
//! (PulseAudio/PipeWire on Linux, WASAPI elsewhere; see the `output` backends).
//!
//! All gain changes (volume, play/pause, add/remove, swell) go through one per-track
//! ramp, so every transition is click-free without a separate fade timer. When nothing
//! is audible the output stream is closed and the thread parks, so an idle engine costs
//! no CPU and doesn't hold the audio device open.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

use rodio::source::UniformSourceIterator;
use rodio::Decoder;

#[cfg_attr(target_os = "linux", path = "output_pulse.rs")]
#[cfg_attr(not(target_os = "linux"), path = "output_cpal.rs")]
mod output;

pub use output::list_devices;
use output::{connect, Output};

const RATE: u32 = 48_000;
const CHANNELS: u16 = 2;
const BLOCK_FRAMES: usize = 1024;
/// Time for a gain change of 1.0 (silence to full) to complete.
const RAMP_SECS: f32 = 0.4;
/// Server-side buffer. Large enough that the thread wakes rarely, small enough
/// that controls still feel immediate.
const BUFFER_MS: u64 = 150;

const SWELL_PERIOD_SECS: f64 = 10.0;
const SWELL_MIN: f64 = 0.4;

type Stream = UniformSourceIterator<Decoder<BufReader<File>>>;

fn open(path: &Path) -> Option<Stream> {
    let file = File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let hint = path.extension().and_then(|e| e.to_str()).unwrap_or("mp3");
    let decoder = Decoder::builder()
        .with_data(BufReader::with_capacity(64 * 1024, file))
        .with_byte_len(len)
        .with_hint(hint)
        .with_gapless(true)
        .build()
        .ok()?;
    Some(UniformSourceIterator::new(decoder, CHANNELS, RATE))
}

#[derive(Clone, Copy, PartialEq)]
pub struct TrackParams {
    pub volume: f32,
    pub swell: bool,
}

struct Track {
    path: PathBuf,
    stream: Stream,
    looping: bool,
    gain: f32,
    params: TrackParams,
    swell_phase: f64,
    /// Removed by the controller; fade to zero then drop.
    removing: bool,
}

impl Track {
    fn target(&self, playing: bool, master: f32) -> f32 {
        if self.removing || (self.looping && !playing) {
            return 0.0;
        }
        let swell = if self.params.swell {
            (SWELL_MIN + (1.0 - SWELL_MIN) * (1.0 + self.swell_phase.cos()) / 2.0) as f32
        } else {
            1.0
        };
        self.params.volume * master * swell
    }

    fn next_sample(&mut self) -> Option<f32> {
        if let Some(s) = self.stream.next() {
            return Some(s);
        }
        if !self.looping {
            return None;
        }
        // Reopen instead of seeking: works for every format and keeps gapless trimming.
        self.stream = open(&self.path)?;
        self.stream.next()
    }
}

#[derive(Default)]
struct Control {
    version: u64,
    playing: bool,
    master: f32,
    duck: f32,
    device: Option<String>,
    /// Desired looping tracks. The audio thread diffs against what it has.
    tracks: HashMap<String, (PathBuf, TrackParams)>,
    oneshots: Vec<(PathBuf, f32)>,
    quit: bool,
}

struct Shared {
    ctl: Mutex<Control>,
    cv: Condvar,
}

#[derive(Clone)]
pub struct Engine(Arc<Shared>);

impl Engine {
    pub fn new(master: f32, device: Option<String>) -> Self {
        let shared = Arc::new(Shared {
            ctl: Mutex::new(Control { master, duck: 1.0, device, ..Default::default() }),
            cv: Condvar::new(),
        });
        let worker = shared.clone();
        thread::Builder::new()
            .name("moodist-audio".into())
            .spawn(move || run(worker))
            .expect("spawn audio thread");
        Engine(shared)
    }

    fn update(&self, f: impl FnOnce(&mut Control)) {
        let mut ctl = self.0.ctl.lock().unwrap();
        f(&mut ctl);
        ctl.version += 1;
        self.0.cv.notify_all();
    }

    pub fn set_playing(&self, playing: bool) {
        self.update(|c| c.playing = playing);
    }

    pub fn set_master(&self, master: f32) {
        self.update(|c| c.master = master);
    }

    /// Multiplier used by the sleep timer to fade everything out gradually.
    pub fn set_duck(&self, duck: f32) {
        self.update(|c| c.duck = duck);
    }

    pub fn set_device(&self, device: Option<String>) {
        self.update(|c| c.device = device);
    }

    /// Replace the whole set of looping tracks.
    pub fn set_tracks(&self, tracks: HashMap<String, (PathBuf, TrackParams)>) {
        self.update(|c| c.tracks = tracks);
    }

    pub fn play_once(&self, path: PathBuf, volume: f32) {
        self.update(|c| c.oneshots.push((path, volume)));
    }

    pub fn shutdown(&self) {
        self.update(|c| c.quit = true);
    }
}

fn run(shared: Arc<Shared>) {
    let mut tracks: HashMap<String, Track> = HashMap::new();
    let mut oneshots: Vec<Track> = Vec::new();
    let mut seen = u64::MAX;
    let (mut playing, mut master) = (false, 1.0f32);
    let mut device: Option<String> = None;
    let mut out: Option<Output> = None;
    let mut buf = vec![0f32; BLOCK_FRAMES * CHANNELS as usize];
    let block_secs = BLOCK_FRAMES as f32 / RATE as f32;
    let max_step = block_secs / RAMP_SECS;
    let swell_step = block_secs as f64 / SWELL_PERIOD_SECS * std::f64::consts::TAU;

    loop {
        let mut ctl = shared.ctl.lock().unwrap();
        if ctl.quit {
            return;
        }
        if ctl.version != seen {
            seen = ctl.version;
            playing = ctl.playing;
            master = ctl.master * ctl.duck;
            if ctl.device != device {
                device = ctl.device.clone();
                out = None;
            }
            for (id, t) in tracks.iter_mut() {
                match ctl.tracks.get(id) {
                    Some((_, params)) => {
                        t.params = *params;
                        t.removing = false;
                    }
                    None => t.removing = true,
                }
            }
            for (id, (path, params)) in &ctl.tracks {
                if tracks.contains_key(id) {
                    continue;
                }
                if let Some(stream) = open(path) {
                    tracks.insert(id.clone(), Track {
                        path: path.clone(),
                        stream,
                        looping: true,
                        gain: 0.0,
                        params: *params,
                        swell_phase: 0.0,
                        removing: false,
                    });
                }
            }
            for (path, volume) in ctl.oneshots.drain(..) {
                if let Some(stream) = open(&path) {
                    oneshots.push(Track {
                        path,
                        stream,
                        looping: false,
                        gain: volume,
                        params: TrackParams { volume, swell: false },
                        swell_phase: 0.0,
                        removing: false,
                    });
                }
            }
        }

        let silent = oneshots.is_empty()
            && tracks.values().all(|t| t.gain == 0.0 && t.target(playing, master) == 0.0);
        if silent {
            // Nothing audible: release the device and sleep until something changes.
            if let Some(s) = out.take() {
                s.drain();
            }
            tracks.retain(|_, t| !t.removing);
            drop(shared.cv.wait_while(ctl, |c| c.version == seen && !c.quit).unwrap());
            continue;
        }
        drop(ctl);

        if out.is_none() {
            out = connect(device.as_deref());
            if out.is_none() {
                // No sound server yet (e.g. right after login); retry shortly.
                thread::sleep(Duration::from_secs(1));
                continue;
            }
        }

        buf.fill(0.0);
        let frames = BLOCK_FRAMES as f32;
        for t in tracks.values_mut() {
            let target = t.target(playing, master);
            let start = t.gain;
            let end = start + (target - start).clamp(-max_step, max_step);
            t.gain = end;
            if start == 0.0 && end == 0.0 {
                continue; // inaudible: don't decode, so the track keeps its place
            }
            if t.params.swell {
                t.swell_phase = (t.swell_phase + swell_step) % std::f64::consts::TAU;
            }
            let delta = (end - start) / frames;
            for (i, frame) in buf.chunks_exact_mut(CHANNELS as usize).enumerate() {
                let g = start + delta * i as f32;
                for s in frame {
                    *s += t.next_sample().unwrap_or(0.0) * g;
                }
            }
        }
        tracks.retain(|_, t| !(t.removing && t.gain == 0.0));

        oneshots.retain_mut(|t| {
            let g = t.gain;
            for s in buf.iter_mut() {
                match t.next_sample() {
                    Some(v) => *s += v * g,
                    None => return false,
                }
            }
            true
        });

        for s in buf.iter_mut() {
            *s = s.clamp(-1.0, 1.0);
        }

        if out.as_ref().is_some_and(|s| !s.write(&buf)) {
            out = None; // server went away; reconnect next block
        }
    }
}

#[derive(Clone, serde::Serialize)]
pub struct OutputDevice {
    pub name: String,
    pub description: String,
}
