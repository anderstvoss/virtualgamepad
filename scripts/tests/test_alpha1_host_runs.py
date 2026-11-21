"""Synthetic-only regression tests; no device or privileged operation is performed."""
import importlib.util
import json
import tempfile
from pathlib import Path
import unittest
from unittest.mock import patch, Mock

SCRIPTS = Path(__file__).resolve().parents[1]


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


matrix = load("alpha1_matrix", "alpha1-host-matrix.py")
converter = load("alpha1_converter", "alpha1-host-to-research.py")
SHA_VG = "a" * 40
SHA_RESEARCH = "b" * 40


def condition():
    return {
        "schema_version": 1, "kind": "alpha1_host_matrix",
        "evidence": "virtual_host_run", "status": "completed",
        "family": "switch-pro", "controller": "virtualgamepad.switch-pro",
        "realization": "linux.uhid.usb", "condition": "bound",
        "environment": {
            "virtualgamepad_sha": SHA_VG, "research_pin_sha": SHA_RESEARCH,
            "kernel": "test-kernel", "sdl_pkg_config_version": "test-sdl",
        },
        "nodes": [{"kind": "hidraw", "node": "hidraw7", "driver": "hid-nintendo"}],
        "sdl_probes": [
            {"requested_backend": "hidapi", "passed": True, "backend": "SDL_HIDAPI",
             "guid": "test-guid"},
            {"requested_backend": "evdev", "passed": False, "backend": None,
             "guid": None},
        ],
        "switch_pro": {"request_order": [
            {"report": "0x80", "command": "0x01", "spi_read": False},
            {"report": "0x01", "command": "0x10", "spi_read": True},
            {"report": "0x01", "command": "0x40", "spi_read": False},
        ], "reply_0x21": "not observed by output callback"},
    }


class MatrixParsing(unittest.TestCase):
    def test_sdl_summary_is_allowlisted_and_reports_actual_backend(self):
        raw = {
            "consumer": "SDL", "passed": True, "selected_count": 1,
            "serial": "must-never-persist", "physical_path": "must-never-persist",
            "observations": {
                "identity": {"value": {"guid": "guid-synth"}},
                "backend": {"value": "SDL_HIDAPI"},
                "build": {"value": {"revision": "rev-synth", "version": 30200}},
            },
        }
        summary = matrix.summarize_probe(json.dumps(raw), "hidapi", 0)
        self.assertEqual(summary["backend"], "SDL_HIDAPI")
        self.assertEqual(summary["guid"], "guid-synth")
        self.assertTrue(summary["passed"])
        self.assertNotIn("must-never-persist", json.dumps(summary))
        self.assertFalse(matrix.summarize_probe(json.dumps(raw), "hidapi", 1)["passed"])

    def test_invalid_sdl_contract_cannot_pass(self):
        self.assertFalse(matrix.summarize_probe("invalid", "evdev", 0)["passed"])
        self.assertFalse(matrix.summarize_probe("", "hidapi", 0)["passed"])

    def test_switch_command_order_and_spi_flag_from_output_only(self):
        lines = ["READY\tinternal", "SW\t80\t01", "SW\t01\t03",
                 "SW\t01\t10", "SW\t01\t40", "SW\t01\t48",
                 "SW\t01\tff", "SW\t21\t04", "CLOSED"]
        requests = matrix.switch_requests(lines)
        self.assertEqual([x["command"] for x in requests],
                         ["0x01", "0x03", "0x10", "0x40", "0x48", "0xff"])
        self.assertEqual([x["spi_read"] for x in requests],
                         [False, False, True, False, False, False])
        self.assertNotIn("0x21", [x["report"] for x in requests])

    def test_node_physical_label_matches_only_explicit_association(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "uevent").write_text("HID_ID=0003:0000057E:00002009\n"
                                         "HID_PHYS=virtualgamepad/synthetic\n")
            self.assertEqual(matrix.node_phys(root), "virtualgamepad/synthetic")
            self.assertIsNone(matrix.node_phys(root.parent / "unrelated-nonexistent"))

    @patch.object(matrix, "invoke", return_value="")
    @patch.object(matrix.os, "geteuid", return_value=1000)
    @patch.object(matrix.subprocess, "run")
    def test_preflight_rejects_missing_device_before_creation(self, run, _uid, _invoke):
        run.return_value = Mock(stdout=json.dumps([{"checks": [
            {"check": "creation-device", "status": "missing"},
            {"check": "consumer-access", "status": "unvalidated"},
        ]}]))
        with self.assertRaises(matrix.HostRunError):
            matrix.preflight(["linux.uhid.usb"])

    @patch.object(matrix, "invoke", return_value="")
    @patch.object(matrix.os, "geteuid", return_value=1000)
    @patch.object(matrix.subprocess, "run")
    def test_preflight_allows_unvalidated_consumer_access(self, run, _uid, _invoke):
        run.return_value = Mock(stdout=json.dumps([{"checks": [
            {"check": "creation-device", "status": "ready"},
            {"check": "consumer-access", "status": "unvalidated"},
        ]}]))
        matrix.preflight(["linux.uinput"])


class ResearchConversion(unittest.TestCase):
    def test_draft_matches_current_research_experiment_contract(self):
        record = converter.research_draft(condition())
        self.assertEqual(set(record), converter.SCHEMA_FIELDS)
        self.assertEqual(record["schema_version"], 1)
        self.assertEqual(record["kind"], "virtual_host_run")
        self.assertEqual(record["status"], "blocked")
        self.assertEqual(record["sources"], [])
        self.assertIn(SHA_VG, record["procedure"])
        self.assertIn(SHA_RESEARCH, record["procedure"])
        self.assertIn("0x80:0x01", record["semantic_outcome"])
        self.assertIn("0x01:0x10", record["semantic_outcome"])
        self.assertIn("0x21 ACK not measured", record["semantic_outcome"])

    def test_missing_sha_or_backend_attempt_rejected(self):
        candidate = condition()
        candidate["environment"]["research_pin_sha"] = "short"
        with self.assertRaises(converter.InvalidRecord):
            converter.research_draft(candidate)
        candidate = condition()
        candidate["sdl_probes"] = [candidate["sdl_probes"][0]]
        with self.assertRaises(converter.InvalidRecord):
            converter.research_draft(candidate)

    def test_unapproved_fields_are_never_copied_to_research(self):
        candidate = condition()
        candidate["serial"] = "secret-test-value"
        candidate["nodes"][0]["location"] = "sensitive-test-value"
        draft = converter.research_draft(candidate)
        self.assertNotIn("secret-test-value", json.dumps(draft))
        self.assertNotIn("sensitive-test-value", json.dumps(draft))

    def test_directory_converter_writes_only_matrix_records(self):
        with tempfile.TemporaryDirectory() as tmp:
            source, dest = Path(tmp) / "source", Path(tmp) / "out"
            source.mkdir()
            (source / "a.json").write_text(json.dumps(condition()))
            (source / "b.json").write_text(json.dumps({"kind": "alpha1_acceptance_sanitized"}))
            files = converter.convert_directory(source, dest)
            self.assertEqual(len(files), 1)
            self.assertEqual(json.loads((dest / files[0]).read_text())["status"], "blocked")
            with self.assertRaises(FileExistsError):
                converter.convert_directory(source, dest)

    def test_acceptance_sanitizer_preserves_timings_not_node_paths(self):
        sample = {
            "schema_version": 1, "kind": "virtual_host_run",
            "controller": "virtualgamepad.dualsense", "realization": "linux.uhid.usb",
            "environment": {
                "git_sha": SHA_VG, "research_revision": SHA_RESEARCH,
                "host_label": "sensitive-label", "user": "sensitive-user",
            },
            "config": {"cycles": 50, "concurrency": 4},
            "evidence_status": "virtual-live",
            "summaries": {"cold/sdl_ready": {"n": 1, "p50_ns": 8000000}},
            "samples": [{
                "cycle": 0, "index": 0, "temperature": "cold",
                "stages": {"api_call": {"value_ns": 1000, "reason": None},
                           "first_neutral": {"value_ns": None, "reason": "sensitive-label"}},
                "observations": {
                    "node_present": {"devices": [{"path": "sensitive-node",
                                                  "bound_driver": "hid-playstation"}]},
                    "sdl_ready": {"observation": {
                        "passed": True, "observations": {
                            "identity": {"value": {"guid": "synthetic-guid"}},
                            "backend": {"value": "SDL_HIDAPI"},
                        },
                    }},
                },
                "errors": ["sensitive-error"],
            }],
        }
        clean = converter.sanitize_acceptance(sample)
        output = json.dumps(clean)
        self.assertIn("8000000", output)
        self.assertIn("hid-playstation", output)
        self.assertIn("synthetic-guid", output)
        self.assertNotIn("sensitive-", output)
        self.assertTrue(clean["samples"][0]["had_errors"])
        self.assertEqual(clean["samples"][0]["stages"]["first_neutral"]["reason"], "not measured")


if __name__ == "__main__":
    unittest.main()
