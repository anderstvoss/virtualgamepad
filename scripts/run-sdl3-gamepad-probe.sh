#!/usr/bin/env bash
set -euo pipefail

if ! command -v pkg-config >/dev/null || ! pkg-config --exists sdl3; then
  echo "SDL3 development files are required (pkg-config package: sdl3)." >&2
  exit 2
fi

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
binary="${TMPDIR:-/tmp}/virtualgamepad-sdl3-gamepad-probe"
if [ ! -x "$binary" ] || [ "$script_dir/sdl3-gamepad-probe.c" -nt "$binary" ]; then
  cc -std=c17 -Wall -Wextra -Werror "$script_dir/sdl3-gamepad-probe.c" \
    $(pkg-config --cflags --libs sdl3) -o "$binary"
fi
exec "$binary" "$@"
