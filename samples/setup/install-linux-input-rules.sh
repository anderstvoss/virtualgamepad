#!/usr/bin/env bash

set -euo pipefail

if [[ "${EUID}" -ne 0 ]]; then
  echo "error: run this script with sudo or as root" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RULE_DIR="/etc/udev/rules.d"
TARGET_GROUP="${TARGET_GROUP:-input}"

install_rule() {
  local source_name="$1"
  local source_path="${SCRIPT_DIR}/${source_name}"
  local target_path="${RULE_DIR}/${source_name}"

  if [[ ! -f "${source_path}" ]]; then
    echo "error: missing rule file ${source_path}" >&2
    exit 1
  fi

  install -m 0644 "${source_path}" "${target_path}"
  echo "installed ${target_path}"
}

trigger_node() {
  local device_node="$1"
  local fallback_sysfs_path="$2"
  local sysfs_path

  if ! sysfs_path="$(udevadm info -q path -n "${device_node}" 2>/dev/null)"; then
    if [[ -e "${fallback_sysfs_path}" ]]; then
      sysfs_path="${fallback_sysfs_path#/sys}"
    else
      echo "warning: ${device_node} is not available yet; skipping trigger" >&2
      return 0
    fi
  fi

  echo "triggering ${device_node} via ${sysfs_path}"
  udevadm trigger --verbose --action=add "/sys${sysfs_path}"
}

ensure_group_note() {
  local user_name="${SUDO_USER:-${USER:-}}"

  if [[ -z "${user_name}" ]]; then
    return 0
  fi

  if id -nG "${user_name}" | tr ' ' '\n' | grep -qx "${TARGET_GROUP}"; then
    echo "group check: user ${user_name} is already in ${TARGET_GROUP}"
  else
    echo "group check: add ${user_name} to ${TARGET_GROUP} with:"
    echo "  sudo usermod -aG ${TARGET_GROUP} ${user_name}"
    echo "  then log out and back in before running smoke tests"
  fi
}

install_rule "99-virtualgamepad-uinput.rules"
install_rule "99-virtualgamepad-uhid.rules"

udevadm control --reload-rules
trigger_node "/dev/uinput" "/sys/devices/virtual/misc/uinput"
trigger_node "/dev/uhid" "/sys/devices/virtual/misc/uhid"

echo
echo "current permissions:"
ls -l /dev/uinput /dev/uhid 2>/dev/null || true
echo
ensure_group_note
echo
echo "verification commands:"
echo "  cargo run -p gr-cli -- run-uinput-smoke generic-gamepad"
echo "  cargo run -p gr-cli -- run-uhid-smoke dualsense --bus usb"
echo "  cargo run -p gr-cli -- run-uhid-smoke dualsense --bus bluetooth"
