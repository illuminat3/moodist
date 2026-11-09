#!/usr/bin/env bash
# Build Moodist from this checkout and install it as a native Arch/CachyOS package.
# Re-run after pulling or editing to update. If Moodist is running it is quit
# before installing and relaunched afterwards, so the new build is picked up.
#
#   ./desktop/install.sh
set -euo pipefail

cd "$(dirname "$(readlink -f "$0")")/packaging/arch"

if ! command -v makepkg >/dev/null || ! command -v pacman >/dev/null; then
  echo "install.sh needs makepkg and pacman (Arch-based distros)." >&2
  echo "On other systems, see desktop/README.md for building an AppImage." >&2
  exit 1
fi

echo "==> Building package"
rm -f ./*.pkg.tar.zst
makepkg --syncdeps --force --noconfirm

pkg=$(ls ./*.pkg.tar.zst)

# Ask the daemon to quit over its socket (saves state), falling back to a signal.
was_running=false
if pgrep -x moodistd >/dev/null; then
  was_running=true
  echo "==> Quitting running Moodist"
  sock="${XDG_RUNTIME_DIR:+$XDG_RUNTIME_DIR/moodist.sock}"
  sock="${sock:-${TMPDIR:-/tmp}/moodist-$USER.sock}"
  if [[ -S $sock ]] && command -v python3 >/dev/null; then
    python3 - "$sock" <<'EOF' || true
import socket, sys
s = socket.socket(socket.AF_UNIX)
s.settimeout(2)
s.connect(sys.argv[1])
s.sendall(b'{"id":0,"method":"quit"}\n')
EOF
  fi
  for _ in {1..20}; do pgrep -x moodistd >/dev/null || break; sleep 0.1; done
  pkill -x moodistd 2>/dev/null || true
  pkill -x moodist 2>/dev/null || true
fi

echo "==> Installing $pkg"
sudo pacman -U --noconfirm "$pkg"

# Refresh the launcher's icon cache and desktop entry index, where available.
command -v gtk-update-icon-cache >/dev/null && sudo gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true
command -v update-desktop-database >/dev/null && sudo update-desktop-database -q /usr/share/applications || true

if $was_running; then
  echo "==> Relaunching Moodist"
  setsid -f moodistd --open >/dev/null 2>&1
fi

echo "==> Done. Moodist is in your app launcher, or run: moodist"
