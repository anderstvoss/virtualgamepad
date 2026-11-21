#!/usr/bin/env python3
"""Produce review-only research virtual_host_run drafts; sanitize xtask timing JSON.

Never copy raw captures, host paths, physical labels or private controller data.
"""
import argparse
import json
from pathlib import Path
import re
import sys

SHA = re.compile(r"^[0-9a-f]{40}$")
FAMILIES = ("dualsense", "dualshock4", "switch-pro", "xbox360")
REALIZATIONS = ("linux.uinput", "linux.uhid.usb", "linux.uhid.bluetooth")
TIMING_REALIZATIONS = (*REALIZATIONS, "linux.usbip.usb-audio")
CONDITIONS = ("bound", "hid-generic")
SCHEMA_FIELDS = {"schema_version", "id", "kind", "status", "sources",
                 "fixtures", "procedure", "semantic_outcome"}


class InvalidRecord(ValueError):
    pass


def validate_source(record):
    if not isinstance(record, dict) or record.get("schema_version") != 1:
        raise InvalidRecord("unsupported matrix version")
    if record.get("kind") != "alpha1_host_matrix":
        raise InvalidRecord("not a matrix record")
    if (record.get("family") not in FAMILIES or
            record.get("realization") not in REALIZATIONS or
            record.get("condition") not in CONDITIONS):
        raise InvalidRecord("unknown condition")
    if record.get("status") != "completed":
        raise InvalidRecord("incomplete source observation")
    environment = record.get("environment")
    if not isinstance(environment, dict) or not all(
        isinstance(environment.get(k), str) and SHA.fullmatch(environment[k])
        for k in ("virtualgamepad_sha", "research_pin_sha")
    ):
        raise InvalidRecord("missing exact source pins")
    probes = record.get("sdl_probes")
    if not isinstance(probes, list) or len(probes) != 2:
        raise InvalidRecord("two explicit SDL backend attempts required")
    if {p.get("requested_backend") for p in probes if isinstance(p, dict)} != {"hidapi", "evdev"}:
        raise InvalidRecord("missing SDL backend attempt")
    for p in probes:
        if type(p.get("passed")) is not bool:
            raise InvalidRecord("SDL result must be explicit")
    return environment, probes


def research_draft(record):
    environment, probes = validate_source(record)
    family = record["family"]
    realization = record["realization"]
    condition = record["condition"]
    short_id = (f"vg-alpha1-p4-{family}-{realization.replace('.', '-')}-"
                f"{condition}-{environment['virtualgamepad_sha'][:12]}")
    probe_result = ", ".join(
        f"{p['requested_backend']}: {'probe passed' if p['passed'] else 'probe failed'}"
        f", SDL backend {p.get('backend') if isinstance(p.get('backend'), str) else 'not measured'}"
        for p in probes
    )
    drivers = sorted({node.get("driver") for node in record.get("nodes", [])
                      if isinstance(node, dict) and isinstance(node.get("driver"), str)})
    procedure = (
        "DRAFT, human review required. Virtual-only host probe (not physical evidence). "
        f"Controller virtualgamepad.{family}; realization {realization}; condition {condition}. "
        "Public Alpha 1 holder and existing SDL3 exact-node probe with HIDAPI enabled/disabled. "
        f"vg SHA {environment['virtualgamepad_sha']}; "
        f"research submodule SHA {environment['research_pin_sha']}; "
        f"kernel {environment.get('kernel', 'not recorded')}; "
        f"SDL pkg-config {environment.get('sdl_pkg_config_version', 'not recorded')}. "
        "Inputs are sanitized matrix JSON; raw captures and host identifiers are not imported."
    )
    outcome = (
        "DRAFT awaiting operator source-output and privacy review; not acceptance or "
        "physical fidelity. " + probe_result +
        "; bound driver set: " + (", ".join(drivers) if drivers else "not observed") + ". "
    )
    if family == "switch-pro":
        request = record.get("switch_pro") or {}
        order = request.get("request_order", [])
        # Only allowlisted command categories, never raw output payloads.
        allowed = {"0x80", "0x01", "0x03", "0x40", "0x48", "0x10"}
        safe = []
        for item in order:
            if (isinstance(item, dict) and item.get("report") in allowed and
                    item.get("command") in allowed):
                safe.append(item["report"] + ":" + item["command"])
        outcome += "Outgoing Switch request order: " + (
            ", ".join(safe) if safe else "not observed") + (
                "; 0x21 ACK not measured from the output callback."
            )
    result = {
        "schema_version": 1,
        "id": short_id,
        "kind": "virtual_host_run",
        "status": "blocked",  # No automatic promotion, even if SDL exits zero.
        "sources": [],
        "fixtures": [],
        "procedure": procedure,
        "semantic_outcome": outcome,
    }
    if set(result) != SCHEMA_FIELDS:
        raise AssertionError("research experiment schema drift")
    return result


def safe_stage(stages):
    output = {}
    for name, measurement in stages.items():
        if not isinstance(name, str) or not isinstance(measurement, dict):
            continue
        ns = measurement.get("value_ns")
        output[name] = {"value_ns": ns if type(ns) is int and ns >= 0 else None,
                        "reason": None if type(ns) is int and ns >= 0 else "not measured"}
    return output


def safe_summary(summary):
    if not isinstance(summary, dict):
        return {}
    return {name: {key: value for key, value in entries.items()
                   if key in ("n", "missing", "min_ns", "p50_ns", "p90_ns", "p95_ns",
                              "p99_ns", "max_ns", "mean_ns", "population_stddev_ns",
                              "mad_ns") and type(value) in (int, float)}
            for name, entries in summary.items()
            if isinstance(name, str) and isinstance(entries, dict)}


def sanitize_acceptance(record):
    if (not isinstance(record, dict) or record.get("schema_version") != 1 or
            record.get("kind") != "virtual_host_run" or
            record.get("controller") not in ["virtualgamepad." + f for f in FAMILIES] or
            record.get("realization") not in TIMING_REALIZATIONS):
        raise InvalidRecord("not a supported Alpha 1 xtask timing run")
    env = record.get("environment", {})
    if not isinstance(env, dict) or not isinstance(env.get("git_sha"), str) or not SHA.fullmatch(env["git_sha"]):
        raise InvalidRecord("timing run missing exact SHA")
    samples = record.get("samples", [])
    if not isinstance(samples, list) or not samples:
        raise InvalidRecord("timing run has no samples")
    clean = []
    for sample in samples:
        if not isinstance(sample, dict):
            raise InvalidRecord("invalid timing sample")
        observations = sample.get("observations") or {}
        nodes = observations.get("node_present", {}).get("devices", []) if isinstance(observations, dict) else []
        drivers = [n.get("bound_driver") for n in nodes if isinstance(n, dict) and
                   isinstance(n.get("bound_driver"), str)]
        sdl = observations.get("sdl_ready", {}) if isinstance(observations, dict) else {}
        capture = sdl.get("observation", {}) if isinstance(sdl, dict) else {}
        clean.append({
            "cycle": sample.get("cycle") if type(sample.get("cycle")) is int else None,
            "index": sample.get("index") if type(sample.get("index")) is int else None,
            "temperature": sample.get("temperature") if sample.get("temperature") in ("cold", "warm") else None,
            "stages": safe_stage(sample.get("stages", {})),
            "driver_names": sorted(set(drivers)),
            "owned_node_count": len(nodes),
            "sdl": summarize_acceptance_sdl(capture),
            "had_errors": bool(sample.get("errors")),
        })
    config = record.get("config", {})
    return {
        "schema_version": 1, "kind": "alpha1_acceptance_sanitized",
        "controller": record["controller"], "realization": record["realization"],
        "cycles": config.get("cycles"), "concurrency": config.get("concurrency"),
        "environment": {
            "virtualgamepad_sha": env["git_sha"],
            "research_pin_sha": env.get("research_revision") if isinstance(env.get("research_revision"), str) and SHA.fullmatch(env["research_revision"]) else None,
            "kernel": env.get("kernel"), "architecture": env.get("cpu_arch"),
            "rustc": env.get("rustc"), "sdl_version": env.get("sdl_version"),
            "probe_source_sha256": env.get("probe_source_sha256"),
        },
        "evidence_status": record.get("evidence_status"),
        "summaries": safe_summary(record.get("summaries")),
        "samples": clean,
        "note": "Sanitized timing observations; original xtask output was ephemeral. No physical evidence.",
    }


def summarize_acceptance_sdl(capture):
    if not isinstance(capture, dict):
        return {"passed": False, "backend": None, "guid": None}
    observations = capture.get("observations", {})
    def measure(field):
        item = observations.get(field, {}) if isinstance(observations, dict) else {}
        return item.get("value") if isinstance(item, dict) else None
    identity = measure("identity")
    return {"passed": capture.get("passed") is True,
            "backend": measure("backend") if isinstance(measure("backend"), str) else None,
            "guid": identity.get("guid") if isinstance(identity, dict) else None}


def convert_directory(source, destination):
    destination.mkdir(parents=True, exist_ok=True)
    written = []
    for path in sorted(source.glob("*.json")):
        record = json.loads(path.read_text())
        if record.get("kind") != "alpha1_host_matrix":
            continue
        draft = research_draft(record)
        output = destination / (draft["id"] + ".json")
        with output.open("x") as stream:
            json.dump(draft, stream, indent=2)
            stream.write("\n")
        written.append(output.name)
    if not written:
        raise InvalidRecord("no matrix condition records found")
    return written


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, help="directory with sanitized matrix JSON")
    parser.add_argument("--out", type=Path, help="directory for draft research experiment JSON")
    parser.add_argument("--sanitize-acceptance", nargs=2, metavar=("RAW", "SANITIZED"),
                        type=Path, help="sanitize one temporary xtask timing result")
    args = parser.parse_args()
    try:
        if args.sanitize_acceptance:
            if args.input or args.out:
                parser.error("sanitization and conversion are separate commands")
            source, dest = args.sanitize_acceptance
            clean = sanitize_acceptance(json.loads(source.read_text()))
            with dest.open("x") as stream:
                json.dump(clean, stream, indent=2)
                stream.write("\n")
        elif args.input and args.out:
            for filename in convert_directory(args.input, args.out):
                print(filename)
        else:
            parser.error("specify --input DIR --out DIR or --sanitize-acceptance RAW DEST")
    except (OSError, ValueError, KeyError, TypeError) as exc:
        print("Host-run record conversion failed: " +
              ("invalid or missing supported record" if not isinstance(exc, InvalidRecord) else str(exc)),
              file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
