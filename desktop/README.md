# Moodist Desktop

A native desktop build of Moodist for Linux and Windows: Tauri (the system webview,
no Electron) for the window, and a small Rust daemon for everything that has to keep
running.

```
moodist   (UI, Tauri + Svelte)  ──local socket──▶  moodistd (audio, state, tray)
  exits when the window closes                      ~3 MB private memory, no GUI toolkit
```

- **moodistd** decodes and mixes the sounds itself and writes them to the system's
  sound server: PipeWire/PulseAudio on Linux, WASAPI on Windows. You can pick any
  output device. When nothing is audible it closes the stream and sleeps, using no
  CPU. It also owns mixes, the playlist, timers and the tray icon (a
  StatusNotifierItem on Linux, e.g. in Waybar's tray, and the notification area on
  Windows).
- **moodist** is only a view. Closing the window ends the process, which frees the
  whole webview. Launching it again (or clicking the tray icon) reconnects.

Closing the window keeps sounds playing (you can turn this off in Settings). Quit
with <kbd>Ctrl Q</kbd> or from the tray menu. If the UI is force-killed, the daemon
exits too.

| | Linux | Windows |
| --- | --- | --- |
| State (mixes, settings, last selection) | `~/.config/moodist/state.json` | `%APPDATA%\Moodist\state.json` |
| UI ↔ daemon | Unix socket in `$XDG_RUNTIME_DIR` | loopback TCP; port in `%LOCALAPPDATA%\Moodist\daemon.port` |

## Develop

You need Rust (`rustup`) and Node 20+. On Linux, also install the Tauri system
packages (WebKitGTK 4.1, GTK 3, librsvg) and the libpulse headers. On Windows,
install the MSVC build tools; WebView2 already ships with Windows 10 and 11.

From `desktop/`:

```sh
npm install
npm run tauri dev        # builds moodistd, then runs the UI with hot reload
npm run catalog          # regenerate sound list + icons from ../src/data/sounds
```

In development, sounds are read from `../public/sounds`. You can point to another
folder with `$MOODIST_SOUNDS`.

## Build for production

`npm run bundle` builds the frontend and the release `moodistd`, then packages both
together with the sounds from `../public/sounds`. Run it on the OS you are building
for, because Tauri doesn't cross-compile installers.

### Windows (.exe installer)

Prerequisites:
- [Rust](https://rustup.rs) with the MSVC toolchain (the default on Windows)
- Visual Studio Build Tools with the "Desktop development with C++" workload
- Node.js 20+

In PowerShell, from `desktop\`:

```powershell
npm ci
npm run bundle -- --bundles nsis
```

Output: `target\release\bundle\nsis\Moodist_<version>_x64-setup.exe`

The installer is per-user, so it doesn't need admin rights. It installs
`moodist.exe`, `moodistd.exe` and `sounds\` into `%LOCALAPPDATA%\Moodist`, adds a
Start menu entry, and installs WebView2 if it's missing.

### Linux (AppImage)

Prerequisites (Debian/Ubuntu package names):

```sh
sudo apt install build-essential curl file patchelf libfuse2 \
  libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libpulse-dev
```

Install Rust from [rustup.rs](https://rustup.rs) and Node.js 20+. Then, from
`desktop/`:

```sh
npm ci
npm run bundle -- --bundles appimage
```

Output: `target/release/bundle/appimage/Moodist_<version>_amd64.AppImage`

Make it executable (`chmod +x`) and run it. The AppImage starts its daemon as a
second instance of itself, so sounds keep playing after the window (and its mount)
closes. Build on the oldest distro you want to support, because the AppImage needs
at least the glibc it was built against.

### Arch / CachyOS (native package)

```sh
cd desktop/packaging/arch
makepkg -si
```

### CI

The **Build desktop app** workflow (`.github/workflows/build_desktop.yml`) builds
both the Windows installer and the AppImage from the same commit. It only runs when
started manually: go to **Actions → Build desktop app → Run workflow**, or run
`gh workflow run build_desktop.yml`. The results are attached to the run as the
`moodist-windows` and `moodist-linux` artifacts.
