#!/usr/bin/env bash
set -uo pipefail

# ─── Args ─────────────────────────────────────────────────────────────────────
DEVICE="${1:-}"
GLIDE="${2:-0.05}"

if [[ -z "$DEVICE" ]]; then
    echo "✗ Usage: sudo ./bridge.sh /dev/input/eventX [glide]"
    exit 1
fi

if [[ ! -e "$DEVICE" ]]; then
    echo "✗ Device $DEVICE not found!"
    exit 1
fi

# ─── Locate the C++ binary (same dir as this script) ──────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY="$SCRIPT_DIR/stable-motion"

if [[ ! -x "$BINARY" ]]; then
    echo "✗ Binary not found at $BINARY — did you compile it?"
    exit 1
fi

# ─── Recover the real user's Wayland session ──────────────────────────────────
REAL_USER="${SUDO_USER:-$USER}"
REAL_UID="$(id -u "$REAL_USER")"

export XDG_RUNTIME_DIR="/run/user/$REAL_UID"

if [[ -z "${WAYLAND_DISPLAY:-}" ]]; then
    WAYLAND_SOCK="$(ls "$XDG_RUNTIME_DIR"/wayland-* 2>/dev/null | head -1)"
    if [[ -n "$WAYLAND_SOCK" ]]; then
        export WAYLAND_DISPLAY="$(basename "$WAYLAND_SOCK")"
    else
        export WAYLAND_DISPLAY="wayland-0"
    fi
fi

export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"

# ─── Cursor theme ─────────────────────────────────────────────────────────────
ORIGINAL_THEME="$(sudo -u "$REAL_USER" \
    DBUS_SESSION_BUS_ADDRESS="$DBUS_SESSION_BUS_ADDRESS" \
    gsettings get org.gnome.desktop.interface cursor-theme 2>/dev/null || echo "'breeze_cursors'")"
ORIGINAL_THEME="${ORIGINAL_THEME//\'/}"  # strip quotes gsettings adds

set_cursor_theme() {
    sudo -u "$REAL_USER" \
        DBUS_SESSION_BUS_ADDRESS="$DBUS_SESSION_BUS_ADDRESS" \
        XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" \
        gsettings set org.gnome.desktop.interface cursor-theme "$1" || true
    sudo -u "$REAL_USER" \
        DBUS_SESSION_BUS_ADDRESS="$DBUS_SESSION_BUS_ADDRESS" \
        XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" \
        kwriteconfig6 --file kcminputrc --group Mouse --key cursorTheme "$1" || true
    sudo -u "$REAL_USER" \
        DBUS_SESSION_BUS_ADDRESS="$DBUS_SESSION_BUS_ADDRESS" \
        XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" \
        qdbus6 org.kde.KWin /KWin reconfigure || true
}

# ─── Log header ───────────────────────────────────────────────────────────────
echo "▶ Bridge starting up"
echo "  User       : $REAL_USER (uid $REAL_UID)"
echo "  Device     : $DEVICE"
echo "  Glide      : $GLIDE"
echo "  Wayland    : $WAYLAND_DISPLAY"
echo "  Runtime dir: $XDG_RUNTIME_DIR"
echo "  Binary     : $BINARY"
echo "─────────────────────────────────────"

# ─── Cleanup on exit ──────────────────────────────────────────────────────────
CPP_PID=""

cleanup() {
    if [[ -n "$CPP_PID" ]] && kill -0 "$CPP_PID" 2>/dev/null; then
        kill -TERM "$CPP_PID" 2>/dev/null
        wait "$CPP_PID" 2>/dev/null
    fi
    set_cursor_theme "$ORIGINAL_THEME"
    kill -TERM "-$$" 2>/dev/null
}

trap cleanup EXIT INT TERM

# ─── Swap cursor theme & launch ───────────────────────────────────────────────
set_cursor_theme "Sweet-cursors"
echo "● Launching stable_motion..."
"$BINARY" "$DEVICE" "$GLIDE" &
CPP_PID=$!

echo "● stable_motion live! (pid $CPP_PID)"

# ─── Wait & monitor ───────────────────────────────────────────────────────────
wait "$CPP_PID"
EXIT_CODE=$?

if [[ $EXIT_CODE -ne 0 ]]; then
    echo "✗ stable_motion exited with code $EXIT_CODE"
else
    echo "■ stable_motion exited cleanly."
fi

exit $EXIT_CODE
