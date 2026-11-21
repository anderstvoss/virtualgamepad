#!/usr/bin/env python3
"""Opt-in Alpha 1 virtual host consumer matrix. No host policy installation.

Only explicitly allowlisted, sanitized observations are written. The raw SDL
probe stream and ephemeral association labels remain process-local.
"""
import argparse
import hashlib
import json
import os
import platform
from pathlib import Path
import queue
import re
import subprocess
import sys
import threading
import time
import tomllib

ROOT = Path(__file__).resolve().parents[1]
FAMILIES = ("dualsense", "dualshock4", "switch-pro", "xbox360")
REALIZATIONS = ("linux.uinput", "linux.uhid.usb", "linux.uhid.bluetooth")
SHA = re.compile(r"^[a-f0-9]{40}$")


class HostRunError(Exception):
    """Sanitized error with no host-specific identifiers."""


def invoke(*args, timeout=30):
    try:
        result = subprocess.run(args, cwd=ROOT, capture_output=True, text=True,
                                check=False, timeout=timeout)
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise HostRunError("required command unavailable or timed out") from exc
    if result.returncode:
        raise HostRunError("required command failed; check prerequisites locally")
    return result.stdout.strip()


def git(*args):
    return invoke("git", *args)


def declared(family, realization):
    manifest = ROOT / "acceptance" / ("virtualgamepad." + family + ".toml")
    with manifest.open("rb") as stream:
        return realization in tomllib.load(stream)["realizations"]


def preflight(realizations, rebind=False):
    if not sys.platform.startswith("linux"):
        raise HostRunError("Linux host is required")
    if os.geteuid() == 0:
        raise HostRunError("run as an ordinary user; SDL probes must not run as root")
    for tool in ("git", "cargo", "rustc", "cc", "pkg-config"):
        invoke("sh", "-c", 'command -v "$1" >/dev/null', "sh", tool)
    invoke("pkg-config", "--modversion", "sdl3")
    # The existing script exits nonzero for its intentionally unvalidated
    # consumer-access cell. Only creation-device readiness is a preflight gate.
    for realization in sorted(set(realizations)):
        target = "linux.uhid.usb" if realization == "linux.uhid.bluetooth" else realization
        run = subprocess.run([sys.executable, str(ROOT / "scripts/host-preflight.py"), target],
                             capture_output=True, text=True, check=False, timeout=10)
        try:
            checks = json.loads(run.stdout)[0]["checks"]
        except (ValueError, IndexError, KeyError) as exc:
            raise HostRunError("host preflight output is invalid") from exc
        creation = [item for item in checks if item["check"] == "creation-device"]
        if len(creation) != 1 or creation[0]["status"] != "ready":
            raise HostRunError("creation device is not ready for " + realization +
                               "; follow docs/ALPHA_2_HOST_AUDIT.md")
    if rebind:
        # Root is used for per-device sysfs writes only, never for SDL or Cargo.
        invoke("sudo", "-n", "true")


def exact_environment():
    vg_sha = git("rev-parse", "--verify", "HEAD")
    research_sha = git("rev-parse", "--verify", "HEAD:protocol-research")
    if not SHA.fullmatch(vg_sha) or not SHA.fullmatch(research_sha):
        raise HostRunError("exact repository commit IDs unavailable")
    return {
        "virtualgamepad_sha": vg_sha,
        "research_pin_sha": research_sha,
        "working_tree_dirty": bool(git("status", "--porcelain", "--untracked-files=no")),
        "kernel": platform.release(),
        "architecture": platform.machine(),
        "rustc": invoke("rustc", "--version"),
        "sdl_pkg_config_version": invoke("pkg-config", "--modversion", "sdl3"),
        "probe_source_sha256": hashlib.sha256(
            (ROOT / "scripts/sdl3-gamepad-probe.c").read_bytes()).hexdigest(),
    }


def read(path):
    try:
        return path.read_text().strip()
    except (OSError, UnicodeError):
        return None


def device_ancestor(path):
    """Find the owned HID parent, if any, without exposing its sysfs address."""
    current = path.resolve()
    for entry in (current, *current.parents):
        if (entry / "uevent").is_file() and "HID_ID=" in (read(entry / "uevent") or ""):
            return entry
        if entry == Path("/sys"):
            break
    return None


def node_phys(path):
    current = path.resolve()
    for entry in (current, *current.parents):
        value = read(entry / "phys")
        if value:
            return value
        uevent = read(entry / "uevent") or ""
        for line in uevent.splitlines():
            if line.startswith("HID_PHYS="):
                return line[len("HID_PHYS="):]
        if entry == Path("/sys"):
            break
    return None


def owned_nodes(labels):
    """Require exact ephemeral public-association ownership before any use."""
    result = []
    for kind, directory, prefix in (
        ("hidraw", Path("/sys/class/hidraw"), "hidraw"),
        ("evdev", Path("/sys/class/input"), "event"),
    ):
        if not directory.is_dir():
            continue
        for entry in directory.iterdir():
            if not re.fullmatch(prefix + r"[0-9]+", entry.name):
                continue
            if node_phys(entry / "device") not in labels:
                continue
            hid = device_ancestor(entry / "device")
            driver_link = hid / "driver" if hid else None
            driver = driver_link.resolve().name if driver_link and driver_link.is_symlink() else None
            devnode = Path("/dev") / entry.name if kind == "hidraw" else Path("/dev/input") / entry.name
            result.append({"kind": kind, "name": entry.name, "driver": driver,
                           "_node": devnode, "_hid": hid})
    return sorted(result, key=lambda item: (item["kind"], item["name"]))


def sanitized_nodes(nodes):
    # Only ephemeral node basenames, never full paths, HID_PHYS labels or sysfs IDs.
    return [{"kind": node["kind"], "node": node["name"], "driver": node["driver"]}
            for node in nodes]


def summarize_probe(output, requested_backend, returncode):
    try:
        records = [json.loads(line) for line in output.splitlines() if line.strip()]
        probe = next(record for record in records if record.get("consumer") == "SDL")
        observations = probe["observations"]

        def value(field):
            measured = observations[field]
            return measured.get("value") if isinstance(measured, dict) else None

        identity = value("identity")
        build = value("build")
        result = {
            "requested_backend": requested_backend,
            "passed": returncode == 0 and probe.get("passed") is True,
            "selected_count": probe.get("selected_count") if type(probe.get("selected_count")) is int else None,
            "backend": value("backend") if isinstance(value("backend"), str) else None,
            "guid": identity.get("guid") if isinstance(identity, dict) else None,
            "sdl_revision": build.get("revision") if isinstance(build, dict) else None,
            "sdl_version": build.get("version") if isinstance(build, dict) else None,
        }
        # A raw consumer error or mapping may contain host data: do not copy it.
        return result
    except (ValueError, KeyError, TypeError, StopIteration, AttributeError):
        return {"requested_backend": requested_backend, "passed": False,
                "selected_count": None, "backend": None, "guid": None,
                "sdl_revision": None, "sdl_version": None}


def probe(node, requested_backend):
    env = os.environ.copy()
    env["SDL_JOYSTICK_HIDAPI"] = "1" if requested_backend == "hidapi" else "0"
    try:
        child = subprocess.run(
            ["bash", str(ROOT / "scripts/run-sdl3-gamepad-probe.sh"),
             str(node["_node"]), "1000"],
            cwd=ROOT, env=env, capture_output=True, text=True, check=False, timeout=30)
        return summarize_probe(child.stdout, requested_backend, child.returncode)
    except (OSError, subprocess.TimeoutExpired):
        return summarize_probe("", requested_backend, 1)


def observation(labels):
    for _ in range(50):
        nodes = owned_nodes(labels)
        if nodes:
            return nodes
        time.sleep(0.1)
    raise HostRunError("no association-owned consumer node appeared")


def condition(family, realization, name, labels, environment):
    nodes = observation(labels)
    hidraw = next((n for n in nodes if n["kind"] == "hidraw"), None)
    event = next((n for n in nodes if n["kind"] == "evdev"), None)
    captures = []
    for backend, node in (("hidapi", hidraw or event), ("evdev", event)):
        if node is None:
            captures.append({"requested_backend": backend, "passed": False,
                             "skipped": "associated node unavailable"})
        else:
            captures.append(probe(node, backend))
    return {
        "schema_version": 1, "kind": "alpha1_host_matrix",
        "evidence": "virtual_host_run", "status": "completed",
        "family": family, "controller": "virtualgamepad." + family,
        "realization": realization, "condition": name,
        "environment": environment, "nodes": sanitized_nodes(nodes),
        "sdl_probes": captures,
        "switch_pro": {"request_order": [], "reply_0x21": "not observed by output callback"}
                      if family == "switch-pro" else None,
        "note": "Virtual device only; consumer selection is not physical fidelity.",
    }


def write_sysfs(path, value):
    try:
        result = subprocess.run(["sudo", "-n", "tee", str(path)], input=value,
                                text=True, capture_output=True, check=False, timeout=10)
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise HostRunError("per-device privileged binding write unavailable") from exc
    if result.returncode:
        raise HostRunError("per-device privileged binding write failed")


def current_driver(hid):
    link = hid / "driver"
    return link.resolve().name if link.is_symlink() else None


def rebind_generic(hid):
    """Only the verified virtual HID parent; restore the exact previous driver."""
    if hid is None or not hid.exists():
        raise HostRunError("owned HID device not identified for rebind")
    original = current_driver(hid)
    if original is None:
        raise HostRunError("owned HID device has no bound driver")
    if original == "hid-generic":
        return original, False
    path = Path("/sys/bus/hid/drivers")
    write_sysfs(path / original / "unbind", hid.name)
    try:
        write_sysfs(path / "hid-generic" / "bind", hid.name)
        if current_driver(hid) != "hid-generic":
            raise HostRunError("generic binding did not take effect")
    except BaseException:
        try:
            # The driver core may have bound generic even when the manual
            # bind command failed. Attempt rollback for either state.
            restore(hid, original, current_driver(hid) != original)
        except BaseException as rollback:
            raise HostRunError("generic bind failed and original driver restoration failed") from rollback
        raise
    return original, True


def restore(hid, previous, changed):
    if not changed:
        return
    current = current_driver(hid)
    if current == "hid-generic":
        write_sysfs(Path("/sys/bus/hid/drivers/hid-generic/unbind"), hid.name)
    if current_driver(hid) is not None:
        raise HostRunError("cannot safely restore: unexpected binding")
    write_sysfs(Path("/sys/bus/hid/drivers") / previous / "bind", hid.name)
    if current_driver(hid) != previous:
        raise HostRunError("original driver restoration failed")


def switch_requests(messages):
    order = []
    for line in messages:
        parts = line.split("\t")
        if len(parts) != 3 or parts[0] != "SW":
            continue
        if parts[1] not in ("80", "01") or not re.fullmatch("[0-9a-f]{2}", parts[2]):
            continue
        sub = parts[2]
        order.append({
            "report": "0x" + parts[1],
            "command": "0x" + sub,
            "spi_read": parts[1] == "01" and sub == "10",
        })
    return order


def run_family(family, realization, environment, out, rebind=False):
    process = subprocess.Popen(
        ["cargo", "run", "--quiet", "--locked", "--example", "alpha1_host_hold", "--",
         family, realization],
        cwd=ROOT, text=True, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL, bufsize=1)
    lines = queue.Queue()
    received = []
    def collect():
        for line in process.stdout:
            lines.put(line.rstrip("\n"))
    thread = threading.Thread(target=collect, daemon=True)
    thread.start()
    try:
        try:
            hello = lines.get(timeout=180)
        except queue.Empty as exc:
            raise HostRunError("public example failed to create a controller") from exc
        if not hello.startswith("READY\t"):
            raise HostRunError("public example did not report readiness")
        labels = set(hello.removeprefix("READY\t").split("|"))
        if not labels or any(not label.startswith("virtualgamepad") for label in labels):
            raise HostRunError("unexpected controller association")
        first = condition(family, realization, "bound", labels, environment)
        results = [first]
        if rebind and realization.startswith("linux.uhid"):
            nodes = observation(labels)
            parents = {n["_hid"] for n in nodes if n["_hid"] is not None}
            if len(parents) != 1:
                raise HostRunError("ambiguous HID parent: refusing rebind")
            hid = parents.pop()
            original = None
            changed = False
            try:
                original, changed = rebind_generic(hid)
                if changed:
                    results.append(condition(family, realization, "hid-generic", labels, environment))
            finally:
                if original is not None:
                    restore(hid, original, changed)
            first["driver_restored"] = current_driver(hid) == original
        # Close while still servicing required protocol traffic, not by killing
        # the holder. The pipe contains only ephemeral association labels.
        process.stdin.write("stop\n")
        process.stdin.flush()
        try:
            process.wait(timeout=8)
        except subprocess.TimeoutExpired as exc:
            raise HostRunError("public example did not close cleanly") from exc
        if process.returncode != 0:
            raise HostRunError("public example exited with a service error")
        thread.join(timeout=2)
        while not lines.empty():
            received.append(lines.get_nowait())
        requests = switch_requests(received)
        for record in results:
            if record["switch_pro"] is not None:
                record["switch_pro"]["request_order"] = requests
            output = out / (family + "--" + realization + "--" + record["condition"] + ".json")
            with output.open("x") as stream:
                json.dump(record, stream, indent=2, sort_keys=True)
                stream.write("\n")
        return results
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--family", choices=(*FAMILIES, "all"), default="all")
    parser.add_argument("--realization", choices=(*REALIZATIONS, "all"), default="all")
    parser.add_argument("--out", type=Path)
    parser.add_argument("--rebind", action="store_true", help="dev-only sudo per-device HID rebind")
    parser.add_argument("--preflight-only", action="store_true")
    args = parser.parse_args()
    families = FAMILIES if args.family == "all" else (args.family,)
    requested = REALIZATIONS if args.realization == "all" else (args.realization,)
    selected = [(f, r) for f in families for r in requested if declared(f, r)]
    for family in families:
        for realization in requested:
            if (family, realization) not in selected:
                print("SKIP undeclared realization: " + family + " " + realization)
    if not selected:
        return 0
    if not args.preflight_only and args.out is None:
        parser.error("--out is required for host runs")
    try:
        preflight((r for _, r in selected), args.rebind)
        if args.preflight_only:
            print("Preflight ready for declared realizations; no host runs performed.")
            return 0
        args.out.mkdir(parents=True, exist_ok=True)
        environment = exact_environment()
        failures = 0
        for family, realization in selected:
            records = run_family(family, realization, environment, args.out, args.rebind)
            failures += sum(not all(c.get("passed") for c in record["sdl_probes"])
                            for record in records)
        return int(failures != 0)
    except (OSError, ValueError, HostRunError) as exc:
        # Never interpolate raw process output or host-specific paths.
        print("Alpha 1 host-run failed: " +
              (str(exc) if isinstance(exc, HostRunError) else "invalid input or filesystem state"),
              file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
