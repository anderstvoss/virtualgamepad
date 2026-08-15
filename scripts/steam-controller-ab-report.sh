#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 3 ]; then
  echo "usage: $0 STEAM_CONSOLE_LOG PHYSICAL_HIDRAW VIRTUAL_SESSION_SERIAL" >&2
  exit 2
fi

log_path=$1
physical_path=$2
virtual_session_serial=$3

if [ ! -r "$log_path" ]; then
  echo "cannot read Steam console log: $log_path" >&2
  exit 2
fi

report_path() {
  local label=$1
  local hidraw_path=$2
  local line
  line=$(awk -v path="$hidraw_path" \
    'index($0, "Added HIDAPI device") && index($0, "path = " path ",") { line = NR } END { if (line) print line }' \
    "$log_path")
  if [ -z "$line" ]; then
    echo "$label: no Steam HIDAPI discovery record for $hidraw_path"
    return
  fi
  echo "$label: latest Steam record for $hidraw_path"
  sed -n "${line},$((line + 20))p" "$log_path"
}

report_path physical "$physical_path"
report_virtual() {
  local serial=$1
  local line
  line=$(awk -v serial="$serial" \
    'index($0, "Added HIDAPI device") && index($0, "serial " serial ",") { line = NR } END { if (line) print line }' \
    "$log_path")
  if [ -z "$line" ]; then
    echo "virtual: no Steam HIDAPI discovery record for session serial $serial"
    return
  fi
  echo "virtual: latest Steam record for session serial $serial"
  sed -n "${line},$((line + 20))p" "$log_path"
}

report_virtual "$virtual_session_serial"
