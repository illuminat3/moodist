import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import catalogJson from './catalog.json';

export interface CatalogSound {
  icon: string;
  id: string;
  label: string;
  path: string;
}

export interface CatalogCategory {
  icon: string;
  id: string;
  sounds: CatalogSound[];
  title: string;
}

export const catalog = catalogJson as {
  categories: CatalogCategory[];
  icons: Record<string, string>;
  ui: Record<string, string>;
};

export const soundsById = new Map(
  catalog.categories.flatMap(c => c.sounds.map(s => [s.id, s] as const)),
);

export interface SoundSetting {
  swell: boolean;
  volume: number;
}

export type SoundSet = Record<string, SoundSetting>;

export interface Mix {
  createdAt: number;
  id: string;
  name: string;
  sounds: SoundSet;
}

export interface PlaylistItem {
  id: number;
  kind: 'sound' | 'mix';
  minutes: number;
  target: string;
}

export interface Settings {
  alarmVolume: number;
  clock24h: boolean;
  clockDate: boolean;
  clockSeconds: boolean;
  clockStyle: string;
  device: string | null;
  masterVolume: number;
  runInBackground: boolean;
  sleepFadeSecs: number;
  theme: 'system' | 'dark' | 'light';
}

export interface AppState {
  favorites: string[];
  mixes: Mix[];
  playing: boolean;
  playlist: {
    current: number | null;
    endsAt: number | null;
    items: PlaylistItem[];
    looping: boolean;
    remainingMs: number | null;
    running: boolean;
    shuffle: boolean;
  };
  settings: Settings;
  sleep: { durationMs: number; endsAt: number } | null;
  sounds: SoundSet;
  timers: Array<{
    durationMs: number;
    endsAt: number | null;
    id: number;
    label: string;
    remainingMs: number;
  }>;
}

export type Page = 'mixes' | 'sounds' | 'playlist' | 'timers' | 'clock' | 'settings';

function stored<T extends string>(key: string, fallback: T): T {
  try {
    return (localStorage.getItem(key) as T) || fallback;
  } catch {
    return fallback;
  }
}

export const ui = $state({
  /** Mix being edited on the sounds page; `new` for an unsaved mix started from +. */
  editing: null as null | { id: string | null; name: string },
  menuOpen: false,
  page: stored<Page>('moodist-page', 'sounds'),
  saveOpen: false,
  toast: '',
});

export const app = $state({ s: null as AppState | null });

/** Calls a method on the moodistd daemon. */
export const call = <T = void>(method: string, params?: Record<string, unknown>) =>
  invoke<T>('rpc', { method, params: params ?? null });

export async function init() {
  await listen<AppState>('state', e => {
    app.s = e.payload;
  });
  app.s = await call<AppState>('get_state');
}

export function go(page: Page) {
  ui.page = page;
  ui.menuOpen = false;
  try {
    localStorage.setItem('moodist-page', page);
  } catch {}
}

let toastTimer: ReturnType<typeof setTimeout> | undefined;
export function toast(message: string) {
  ui.toast = message;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (ui.toast = ''), 2400);
}

/**
 * Coalesces rapid calls (slider drags) into at most one per ~16ms. Uses a timer rather
 * than requestAnimationFrame, which WebKit pauses when the window isn't being painted.
 */
function throttle(flush: () => void) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  return () => {
    timer ??= setTimeout(() => {
      timer = undefined;
      flush();
    }, 16);
  };
}

const pending = new Map<string, SoundSetting>();
const flushSounds = throttle(() => {
  for (const [id, s] of pending) call('set_sound', { id, ...s });
  pending.clear();
});

export function setSound(id: string, setting: SoundSetting) {
  if (app.s?.sounds[id]) app.s.sounds[id] = setting;
  pending.set(id, setting);
  flushSounds();
}

const flushSettings = throttle(() => {
  if (app.s) call('set_settings', { settings: $state.snapshot(app.s.settings) });
});

export function setSettings(patch: Partial<Settings>) {
  if (!app.s) return;
  Object.assign(app.s.settings, patch);
  if (patch.theme) applyTheme(patch.theme);
  flushSettings();
}

export function applyTheme(pref: Settings['theme']) {
  try {
    localStorage.setItem('moodist-theme', pref);
  } catch {}
  const dark =
    pref === 'system'
      ? matchMedia('(prefers-color-scheme: dark)').matches
      : pref === 'dark';
  document.documentElement.dataset.theme = dark ? 'dark' : 'light';
}

/** Mirrors the Rust auto-naming so the save dialog can preview it. */
export function autoName(sounds: SoundSet) {
  const labels = Object.entries(sounds)
    .sort((a, b) => b[1].volume - a[1].volume || a[0].localeCompare(b[0]))
    .map(([id]) => soundsById.get(id)?.label ?? id);
  if (labels.length === 0) return 'Silence';
  if (labels.length === 1) return labels[0];
  if (labels.length === 2) return `${labels[0]} & ${labels[1]}`;
  if (labels.length === 3) return `${labels[0]}, ${labels[1]} & ${labels[2]}`;
  return `${labels[0]}, ${labels[1]} & ${labels.length - 2} more`;
}

export function sameSounds(a: SoundSet, b: SoundSet) {
  const ka = Object.keys(a);
  if (ka.length !== Object.keys(b).length) return false;
  return ka.every(
    k => b[k] && Math.abs(b[k].volume - a[k].volume) < 0.005 && b[k].swell === a[k].swell,
  );
}

export function formatMs(ms: number) {
  const total = Math.max(0, Math.ceil(ms / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const pad = (n: number) => String(n).padStart(2, '0');
  return h ? `${h}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`;
}

/** A ticking clock shared by every countdown on screen. Only runs while observed. */
export const clock = $state({ now: Date.now() });
let clockUsers = 0;
let clockTimer: ReturnType<typeof setTimeout> | undefined;
function tick() {
  clock.now = Date.now();
  clockTimer = setTimeout(tick, 1000 - (clock.now % 1000) + 5);
}
export function useClock(active: () => unknown = () => true) {
  $effect(() => {
    if (!active()) return;
    if (clockUsers++ === 0) tick();
    return () => {
      if (--clockUsers === 0) clearTimeout(clockTimer);
    };
  });
}
