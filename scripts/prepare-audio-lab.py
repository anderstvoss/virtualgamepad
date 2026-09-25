#!/usr/bin/env python3
"""Prepare local USB lab modules; never grant device access or start a broker.

Default is a read-only plan. --apply requires an administrator shell. Existing
UDCs/gadgets are never unbound, reset or removed; occupied or insufficient dummy
controllers require the administrator to resolve the conflict independently.
"""
import argparse
import os
from pathlib import Path
import subprocess


def commands(configfs_mounted, dummy_loaded, transport="dummy-hcd"):
    if transport == "usbip":
        return [["/sbin/modprobe", "usbip_core"], ["/sbin/modprobe", "vhci_hcd"]]
    if transport != "dummy-hcd":
        raise ValueError("unsupported lab transport")
    result = [["/sbin/modprobe", "libcomposite"],
              ["/sbin/modprobe", "usb_f_fs"],
              ["/sbin/modprobe", "usb_f_uac2"],
              ["/sbin/modprobe", "usb_f_uac1"]]
    if not dummy_loaded:
        result.append(["/sbin/modprobe", "dummy_hcd", "num=2"])
    if not configfs_mounted:
        result.append(["mount", "-t", "configfs", "configfs", "/sys/kernel/config"])
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--apply", action="store_true")
    parser.add_argument("--transport", choices=["dummy-hcd", "usbip"], default="dummy-hcd")
    args = parser.parse_args()
    mounts = Path("/proc/mounts").read_text().splitlines()
    mounted = any(line.split()[1:3] == ["/sys/kernel/config", "configfs"] for line in mounts)
    loaded = Path("/sys/module/dummy_hcd").exists()
    todo = commands(mounted, loaded, args.transport)
    for command in todo:
        print(" ".join(command), flush=True)
    if not args.apply:
        print("Read-only plan. No changes made.")
        return 0
    if os.geteuid() != 0:
        parser.error("--apply requires an administrator; no automatic elevation")
    for command in todo:
        subprocess.run(command, check=True, timeout=30)
    if args.transport == "dummy-hcd":
        udcs = list(Path("/sys/class/udc").glob("dummy_udc.*"))
        if len(udcs) < 2:
            raise SystemExit("Two dummy UDCs are required; existing controllers were not reset.")
        print("Stock dummy_hcd does not support isochronous USB audio; this setup does not enable PCM streaming.")
    elif not list(Path("/sys/devices/platform").glob("vhci_hcd.*/status")):
        raise SystemExit("VHCI status is unavailable; no ports were attached or detached.")
    print("Lab modules ready. Broker installation and authorization remain separate.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
