//! Application state. Everything that must keep working with the window closed
//! (playback, playlist, timers) lives here rather than in the webview.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::audio::{Engine, TrackParams};

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

#[derive(Deserialize)]
struct CatalogEntry {
    id: String,
    label: String,
    path: String,
}

pub struct Catalog {
    sounds: HashMap<String, (String, PathBuf)>,
}

impl Catalog {
    pub fn load(dir: &Path) -> Self {
        let entries: Vec<CatalogEntry> =
            serde_json::from_str(include_str!("../catalog.json")).expect("valid catalog");
        let sounds = entries.into_iter().map(|e| (e.id, (e.label, dir.join(e.path)))).collect();
        Catalog { sounds }
    }

    pub fn contains(&self, id: &str) -> bool {
        self.sounds.contains_key(id)
    }

    fn label<'a>(&'a self, id: &'a str) -> &'a str {
        self.sounds.get(id).map_or(id, |(l, _)| l.as_str())
    }

    fn path(&self, id: &str) -> Option<&PathBuf> {
        self.sounds.get(id).map(|(_, p)| p)
    }

    fn ids(&self) -> Vec<&String> {
        let mut ids: Vec<_> = self.sounds.keys().collect();
        ids.sort();
        ids
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct SoundSetting {
    pub volume: f32,
    pub swell: bool,
}

pub type SoundSet = BTreeMap<String, SoundSetting>;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Mix {
    pub id: String,
    pub name: String,
    pub sounds: SoundSet,
    pub created_at: u64,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub master_volume: f32,
    pub alarm_volume: f32,
    pub device: Option<String>,
    pub theme: String,
    pub clock_style: String,
    pub clock_24h: bool,
    pub clock_seconds: bool,
    pub clock_date: bool,
    pub run_in_background: bool,
    pub sleep_fade_secs: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            master_volume: 1.0,
            alarm_volume: 1.0,
            device: None,
            theme: "system".into(),
            clock_style: "digital".into(),
            clock_24h: true,
            clock_seconds: false,
            clock_date: true,
            run_in_background: true,
            sleep_fade_secs: 30,
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct Persisted {
    settings: Settings,
    mixes: Vec<Mix>,
    favorites: Vec<String>,
    current: SoundSet,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Sound,
    Mix,
}

#[derive(Serialize, Clone)]
pub struct PlaylistItem {
    pub id: u64,
    pub kind: ItemKind,
    pub target: String,
    pub minutes: f64,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    items: Vec<PlaylistItem>,
    shuffle: bool,
    looping: bool,
    running: bool,
    current: Option<u64>,
    ends_at: Option<u64>,
    remaining_ms: Option<u64>,
    #[serde(skip)]
    order: Vec<u64>,
    #[serde(skip)]
    position: usize,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SleepTimer {
    ends_at: u64,
    duration_ms: u64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Timer {
    id: u64,
    label: String,
    duration_ms: u64,
    remaining_ms: u64,
    ends_at: Option<u64>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot<'a> {
    playing: bool,
    sounds: &'a SoundSet,
    favorites: &'a [String],
    mixes: &'a [Mix],
    playlist: &'a Playlist,
    sleep: &'a Option<SleepTimer>,
    timers: &'a [Timer],
    settings: &'a Settings,
}

pub struct Core {
    data: Persisted,
    pub playing: bool,
    playlist: Playlist,
    sleep: Option<SleepTimer>,
    timers: Vec<Timer>,
    next_id: u64,
    rng: u64,
    pub save_at: Option<u64>,
    ducking: bool,
    config: PathBuf,
    pub catalog: Catalog,
    alarm: PathBuf,
    pub engine: Engine,
}

const SAVE_DELAY_MS: u64 = 800;
const DEFAULT_VOLUME: f32 = 0.5;

impl Core {
    pub fn new(config: PathBuf, sounds_dir: &Path) -> Self {
        let mut data: Persisted = std::fs::read(&config)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        let catalog = Catalog::load(sounds_dir);
        data.current.retain(|id, _| catalog.contains(id));
        let engine = Engine::new(data.settings.master_volume, data.settings.device.clone());
        let core = Core {
            data,
            playing: false,
            playlist: Playlist::default(),
            sleep: None,
            timers: Vec::new(),
            next_id: 1,
            rng: now_ms() | 1,
            save_at: None,
            ducking: false,
            config,
            catalog,
            alarm: sounds_dir.join("alarm.mp3"),
            engine,
        };
        core.sync_audio();
        core
    }

    pub fn snapshot(&self) -> Snapshot<'_> {
        Snapshot {
            playing: self.playing,
            sounds: &self.data.current,
            favorites: &self.data.favorites,
            mixes: &self.data.mixes,
            playlist: &self.playlist,
            sleep: &self.sleep,
            timers: &self.timers,
            settings: &self.data.settings,
        }
    }

    fn id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    fn random(&mut self) -> u64 {
        // xorshift64: plenty for shuffling a queue.
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = (self.random() % (i as u64 + 1)) as usize;
            items.swap(i, j);
        }
    }

    fn dirty(&mut self) {
        self.save_at.get_or_insert(now_ms() + SAVE_DELAY_MS);
    }

    pub fn save(&mut self) {
        self.save_at = None;
        if let Some(dir) = self.config.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let Ok(json) = serde_json::to_vec_pretty(&self.data) else { return };
        // Write-then-rename so a crash mid-write never corrupts saved mixes.
        let tmp = self.config.with_extension("json.tmp");
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, &self.config);
        }
    }

    fn sync_audio(&self) {
        let tracks = self
            .data
            .current
            .iter()
            .filter_map(|(id, s)| {
                let path = self.catalog.path(id)?.clone();
                Some((id.clone(), (path, TrackParams { volume: s.volume, swell: s.swell })))
            })
            .collect();
        self.engine.set_tracks(tracks);
        self.engine.set_playing(self.playing);
    }

    // ---- Sounds ----

    pub fn set_playing(&mut self, playing: bool) {
        let playing = playing && !self.data.current.is_empty();
        if playing == self.playing {
            return;
        }
        self.playing = playing;
        let now = now_ms();
        let pl = &mut self.playlist;
        if pl.running {
            if playing {
                if let Some(rem) = pl.remaining_ms.take() {
                    pl.ends_at = Some(now + rem);
                }
            } else if let Some(end) = pl.ends_at.take() {
                pl.remaining_ms = Some(end.saturating_sub(now));
            }
        }
        self.engine.set_playing(playing);
    }

    pub fn toggle_sound(&mut self, id: &str) {
        if !self.catalog.contains(id) {
            return;
        }
        if self.data.current.remove(id).is_none() {
            self.data.current.insert(id.into(), SoundSetting { volume: DEFAULT_VOLUME, swell: false });
            self.playing = true;
        } else if self.data.current.is_empty() {
            self.playing = false;
        }
        self.dirty();
        self.sync_audio();
    }

    pub fn set_sound(&mut self, id: &str, setting: SoundSetting) {
        if let Some(s) = self.data.current.get_mut(id) {
            *s = SoundSetting { volume: setting.volume.clamp(0.0, 1.0), swell: setting.swell };
            self.dirty();
            self.sync_audio();
        }
    }

    pub fn replace_sounds(&mut self, sounds: SoundSet, play: bool) {
        self.data.current = sounds.into_iter().filter(|(id, _)| self.catalog.contains(id)).collect();
        self.playing = play && !self.data.current.is_empty();
        self.dirty();
        self.sync_audio();
    }

    pub fn shuffle_sounds(&mut self) {
        let mut ids: Vec<String> = self.catalog.ids().into_iter().cloned().collect();
        self.shuffle(&mut ids);
        let sounds = ids
            .into_iter()
            .take(4)
            .map(|id| {
                let volume = 0.2 + (self.random() % 800) as f32 / 1000.0;
                (id, SoundSetting { volume, swell: false })
            })
            .collect();
        self.replace_sounds(sounds, true);
    }

    pub fn set_favorite(&mut self, id: &str, favorite: bool) {
        self.data.favorites.retain(|f| f != id);
        if favorite && self.catalog.contains(id) {
            self.data.favorites.push(id.into());
        }
        self.dirty();
    }

    // ---- Mixes ----

    fn auto_name(&self, sounds: &SoundSet) -> String {
        let mut by_volume: Vec<_> = sounds.iter().collect();
        by_volume.sort_by(|a, b| b.1.volume.total_cmp(&a.1.volume).then(a.0.cmp(b.0)));
        let labels: Vec<&str> = by_volume.iter().map(|(id, _)| self.catalog.label(id)).collect();
        match labels.as_slice() {
            [] => "Silence".into(),
            [a] => (*a).into(),
            [a, b] => format!("{a} & {b}"),
            [a, b, c] => format!("{a}, {b} & {c}"),
            [a, b, rest @ ..] => format!("{a}, {b} & {} more", rest.len()),
        }
    }

    fn unique_name(&self, base: String, except: Option<&str>) -> String {
        let taken = |n: &str| self.data.mixes.iter().any(|m| m.name == n && Some(m.id.as_str()) != except);
        if !taken(&base) {
            return base;
        }
        (2..).map(|i| format!("{base} {i}")).find(|n| !taken(n)).unwrap()
    }

    /// Saves the current sounds as a mix. Updates `id` in place when given.
    pub fn save_mix(&mut self, name: Option<String>, id: Option<String>) -> Option<Mix> {
        if self.data.current.is_empty() {
            return None;
        }
        let sounds = self.data.current.clone();
        let name = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
        let name = name.unwrap_or_else(|| self.auto_name(&sounds));
        let existing = id.as_deref().and_then(|id| self.data.mixes.iter().position(|m| m.id == id));
        let name = self.unique_name(name, id.as_deref().filter(|_| existing.is_some()));
        let id = match existing {
            Some(i) => {
                let mix = &mut self.data.mixes[i];
                mix.name = name;
                mix.sounds = sounds;
                mix.id.clone()
            }
            None => {
                let id = format!("{:x}{:x}", now_ms(), self.random() & 0xffff);
                self.data.mixes.push(Mix { id: id.clone(), name, sounds, created_at: now_ms() });
                id
            }
        };
        self.dirty();
        self.data.mixes.iter().find(|m| m.id == id).cloned()
    }

    pub fn delete_mix(&mut self, id: &str) {
        self.data.mixes.retain(|m| m.id != id);
        let queued: Vec<u64> = self.playlist.items.iter().filter(|i| i.kind == ItemKind::Mix && i.target == id).map(|i| i.id).collect();
        for item in queued {
            self.playlist_remove(item);
        }
        self.dirty();
    }

    pub fn rename_mix(&mut self, id: &str, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let name = self.unique_name(name.into(), Some(id));
        if let Some(m) = self.data.mixes.iter_mut().find(|m| m.id == id) {
            m.name = name;
            self.dirty();
        }
    }

    pub fn play_mix(&mut self, id: &str) {
        if let Some(mix) = self.data.mixes.iter().find(|m| m.id == id) {
            let sounds = mix.sounds.clone();
            self.stop_playlist();
            self.replace_sounds(sounds, true);
        }
    }

    // ---- Playlist ----

    pub fn playlist_add(&mut self, kind: ItemKind, target: String, minutes: f64) {
        let valid = match kind {
            ItemKind::Sound => self.catalog.contains(&target),
            ItemKind::Mix => self.data.mixes.iter().any(|m| m.id == target),
        };
        if valid {
            let id = self.id();
            self.playlist.items.push(PlaylistItem { id, kind, target, minutes: minutes.max(0.1) });
        }
    }

    pub fn playlist_remove(&mut self, id: u64) {
        self.playlist.items.retain(|i| i.id != id);
        if self.playlist.current == Some(id) {
            self.playlist_skip(1);
        }
    }

    pub fn playlist_set_minutes(&mut self, id: u64, minutes: f64) {
        let minutes = minutes.max(0.1);
        let pl = &mut self.playlist;
        if let Some(item) = pl.items.iter_mut().find(|i| i.id == id) {
            let old = item.minutes;
            item.minutes = minutes;
            // Adjust the running countdown so the change takes effect immediately.
            if pl.current == Some(id) {
                let delta = ((minutes - old) * 60_000.0) as i64;
                let shift = |t: u64| (t as i64 + delta).max(now_ms() as i64) as u64;
                pl.ends_at = pl.ends_at.map(shift);
                pl.remaining_ms = pl.remaining_ms.map(|r| (r as i64 + delta).max(0) as u64);
            }
        }
    }

    pub fn playlist_move(&mut self, id: u64, to: usize) {
        let items = &mut self.playlist.items;
        if let Some(from) = items.iter().position(|i| i.id == id) {
            let item = items.remove(from);
            items.insert(to.min(items.len()), item);
        }
        if self.playlist.running && !self.playlist.shuffle {
            self.rebuild_order(false);
        }
    }

    pub fn playlist_clear(&mut self) {
        self.stop_playlist();
        self.playlist.items.clear();
    }

    pub fn playlist_options(&mut self, shuffle: bool, looping: bool) {
        let reshuffle = shuffle != self.playlist.shuffle;
        self.playlist.shuffle = shuffle;
        self.playlist.looping = looping;
        if reshuffle && self.playlist.running {
            self.rebuild_order(false);
        }
    }

    /// Recomputes the play order, keeping the current item at the current position.
    fn rebuild_order(&mut self, fresh: bool) {
        let mut ids: Vec<u64> = self.playlist.items.iter().map(|i| i.id).collect();
        if self.playlist.shuffle {
            self.shuffle(&mut ids);
        }
        let current = if fresh { None } else { self.playlist.current };
        if let Some(cur) = current {
            ids.retain(|&i| i != cur);
            ids.insert(0, cur);
        }
        self.playlist.order = ids;
        self.playlist.position = 0;
    }

    pub fn playlist_start(&mut self, from: Option<u64>) {
        if self.playlist.items.is_empty() {
            return;
        }
        self.playlist.running = true;
        self.playlist.current = from;
        self.rebuild_order(from.is_none());
        self.play_position();
    }

    fn play_position(&mut self) {
        let pl = &self.playlist;
        let Some(item) = pl.order.get(pl.position).and_then(|id| pl.items.iter().find(|i| i.id == *id)).cloned()
        else {
            return self.stop_playlist();
        };
        let sounds = match item.kind {
            ItemKind::Sound => {
                let volume = self.data.current.get(&item.target).map_or(DEFAULT_VOLUME, |s| s.volume);
                SoundSet::from([(item.target.clone(), SoundSetting { volume, swell: false })])
            }
            ItemKind::Mix => match self.data.mixes.iter().find(|m| m.id == item.target) {
                Some(m) => m.sounds.clone(),
                None => return self.stop_playlist(),
            },
        };
        let pl = &mut self.playlist;
        pl.current = Some(item.id);
        pl.ends_at = Some(now_ms() + (item.minutes * 60_000.0) as u64);
        pl.remaining_ms = None;
        self.replace_sounds(sounds, true);
    }

    pub fn playlist_skip(&mut self, delta: i64) {
        if !self.playlist.running {
            return;
        }
        // Drop deleted items and pick up any added while running.
        let pl = &mut self.playlist;
        pl.order.retain(|id| pl.items.iter().any(|i| i.id == *id));
        for item in &pl.items {
            if !pl.order.contains(&item.id) {
                pl.order.push(item.id);
            }
        }
        // If the current item was removed, the next one has slid into its position.
        let pos = match pl.current.and_then(|c| pl.order.iter().position(|&i| i == c)) {
            Some(p) => p as i64 + delta,
            None => pl.position as i64,
        };
        if pl.order.is_empty() {
            return self.stop_playlist();
        }
        if pos >= pl.order.len() as i64 {
            if !pl.looping {
                self.stop_playlist();
                return self.set_playing(false);
            }
            self.playlist.current = None;
            self.rebuild_order(true);
        } else {
            pl.position = pos.max(0) as usize;
        }
        self.play_position();
    }

    pub fn stop_playlist(&mut self) {
        let pl = &mut self.playlist;
        pl.running = false;
        pl.current = None;
        pl.ends_at = None;
        pl.remaining_ms = None;
    }

    // ---- Timers ----

    pub fn sleep_start(&mut self, minutes: f64) {
        let duration_ms = (minutes * 60_000.0) as u64;
        self.sleep = Some(SleepTimer { ends_at: now_ms() + duration_ms, duration_ms });
        self.unduck();
    }

    pub fn sleep_cancel(&mut self) {
        self.sleep = None;
        self.unduck();
    }

    fn unduck(&mut self) {
        if self.ducking {
            self.ducking = false;
            self.engine.set_duck(1.0);
        }
    }

    pub fn timer_add(&mut self, label: String, minutes: f64) {
        let id = self.id();
        let duration_ms = (minutes.max(1.0 / 60.0) * 60_000.0) as u64;
        let label = if label.trim().is_empty() { format_duration(duration_ms) } else { label.trim().into() };
        self.timers.push(Timer { id, label, duration_ms, remaining_ms: duration_ms, ends_at: Some(now_ms() + duration_ms) });
    }

    pub fn timer_toggle(&mut self, id: u64) {
        let now = now_ms();
        if let Some(t) = self.timers.iter_mut().find(|t| t.id == id) {
            match t.ends_at.take() {
                Some(end) => t.remaining_ms = end.saturating_sub(now),
                None => {
                    if t.remaining_ms == 0 {
                        t.remaining_ms = t.duration_ms;
                    }
                    t.ends_at = Some(now + t.remaining_ms);
                }
            }
        }
    }

    pub fn timer_reset(&mut self, id: u64) {
        if let Some(t) = self.timers.iter_mut().find(|t| t.id == id) {
            t.ends_at = None;
            t.remaining_ms = t.duration_ms;
        }
    }

    pub fn timer_remove(&mut self, id: u64) {
        self.timers.retain(|t| t.id != id);
    }

    pub fn set_settings(&mut self, settings: Settings) {
        let old = &self.data.settings;
        if old.master_volume != settings.master_volume {
            self.engine.set_master(settings.master_volume.clamp(0.0, 1.0));
        }
        if old.device != settings.device {
            self.engine.set_device(settings.device.clone());
        }
        self.data.settings = settings;
        self.dirty();
    }

    pub fn play_alarm(&self) {
        self.engine.play_once(self.alarm.clone(), self.data.settings.alarm_volume);
    }

    /// Advances time-based state. Returns (state changed, next wake-up time).
    pub fn tick(&mut self) -> (bool, Option<u64>) {
        let now = now_ms();
        let mut changed = false;

        if let Some(end) = self.playlist.ends_at {
            if now >= end {
                self.playlist_skip(1);
                changed = true;
            }
        }

        if let Some(sleep) = self.sleep.clone() {
            let fade = self.data.settings.sleep_fade_secs as u64 * 1000;
            if now >= sleep.ends_at {
                self.sleep = None;
                self.set_playing(false);
                self.unduck();
                changed = true;
            } else if fade > 0 && now + fade >= sleep.ends_at {
                self.ducking = true;
                self.engine.set_duck((sleep.ends_at - now) as f32 / fade as f32);
            }
        }

        let mut alarm = false;
        for t in &mut self.timers {
            if t.ends_at.is_some_and(|end| now >= end) {
                t.ends_at = None;
                t.remaining_ms = 0;
                alarm = true;
            }
        }
        if alarm {
            self.play_alarm();
            changed = true;
        }

        if self.save_at.is_some_and(|at| now >= at) {
            self.save();
        }

        let mut next = [self.playlist.ends_at, self.save_at]
            .into_iter()
            .chain(self.timers.iter().map(|t| t.ends_at))
            .flatten()
            .min();
        if let Some(sleep) = &self.sleep {
            let fade_start = sleep.ends_at.saturating_sub(self.data.settings.sleep_fade_secs as u64 * 1000);
            // Step the fade twice a second; the engine's ramp smooths between steps.
            let at = if now >= fade_start { (now + 500).min(sleep.ends_at) } else { fade_start };
            next = Some(next.map_or(at, |n| n.min(at)));
        }
        (changed, next)
    }
}

fn format_duration(ms: u64) -> String {
    let secs = ms / 1000;
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    match (h, m, s) {
        (0, m, 0) => format!("{m} min"),
        (0, m, s) => format!("{m}:{s:02}"),
        (h, 0, 0) => format!("{h} hr"),
        (h, m, _) => format!("{h} hr {m} min"),
    }
}
