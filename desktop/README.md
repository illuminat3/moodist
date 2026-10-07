# Moodist Desktop

A native Linux build of Moodist: Tauri (system WebKitGTK, no Electron) for the
window, and a small Rust daemon for everything that has to keep running.

```
moodist   (UI, Tauri + Svelte)  ──unix socket──▶  moodistd (audio, state, tray)
  exits when the window closes                     ~3 MB private memory, no GTK
```

- **moodistd** decodes and mixes the sounds itself and writes to PipeWire through
  its PulseAudio interface, so it can target any output device. When nothing is
  audible it closes the stream and sleeps (zero CPU). It also owns mixes,
  playlist, timers and the tray icon (StatusNotifierItem, e.g. Waybar's tray).
- **moodist** is only a view. Closing the window ends the process, freeing all
  of WebKit. Launching it again (or clicking the tray icon) reconnects.

Closing the window keeps sounds playing (toggle in Settings). Quit with
<kbd>Ctrl Q</kbd> or the tray menu. If the UI is force-killed, the daemon exits too.

State (mixes, settings, last selection) lives in `~/.config/moodist/state.json`.

## Install (Arch / CachyOS)

```sh
cd desktop/packaging/arch
makepkg -si
```

## Develop

Needs Rust (`rustup`) and Node. From `desktop/`:

```sh
npm install
npm run tauri dev        # builds moodistd, then runs the UI with hot reload
npm run catalog          # regenerate sound list + icons from ../src/data/sounds
```

Sounds are read from `../public/sounds` in development, `/usr/share/moodist/sounds`
when installed, or `$MOODIST_SOUNDS` if set.
