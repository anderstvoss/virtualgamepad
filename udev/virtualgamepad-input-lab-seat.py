#!/usr/bin/python3 -I
"""Validate an opt-in controller UHID/uinput label and emit only its inactive seat name.

The producer uses lowercase hexadecimal numeric IDs. No device-supplied value
is used as a command, path, or network address. Both transports use the 63-byte
UHID lab-label budget; accepted input is ASCII, so characters equal bytes.
"""

from __future__ import annotations

import re
import sys

_MAX_PHYS_LENGTH = 63
_MAX_PID = (1 << 31) - 1  # Linux pid_t is a signed 32-bit integer.
_PHYS = re.compile(
    r"virtualgamepad/input-lab/(?:(?:uhid|uinput)/(?:dualsense|dualshock4|steam-controller-2026)|uhid/xbox-series)/"
    r"p([1-9a-f][0-9a-f]{0,7})-i(0|[1-9a-f][0-9a-f]{0,15})",
    re.ASCII,
)


def seat_for_phys(phys: str) -> str | None:
    """Return a seat only for a bounded, canonical, positive numeric PID."""
    if not 1 <= len(phys) <= _MAX_PHYS_LENGTH:
        return None
    match = _PHYS.fullmatch(phys)
    if match is None:
        return None
    process = int(match.group(1), 16)
    if process > _MAX_PID:
        return None
    return f"seat-vg-alpha-p{process:x}"


def main() -> int:
    if len(sys.argv) != 2:
        return 1
    seat = seat_for_phys(sys.argv[1])
    if seat is None:
        return 1
    print(seat)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
