import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "alpha2_polish_report", Path(__file__).resolve().parents[1] / "alpha2-polish-report.py"
)
report = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(report)
SHA = "a" * 40


class Alpha2PolishReportTests(unittest.TestCase):
    def setUp(self):
        attempt = tempfile.TemporaryDirectory()
        self.addCleanup(attempt.cleanup)
        self.attempt = Path(attempt.name)
        self.matrix = report.load_matrix()
        self.first, self.second = list(self.matrix)[:2]

    def create_receipt(self, **changes):
        receipt = {
            "id": self.first,
            "sha": SHA,
            "status": "verified",
            "cleanup": True,
            "counts": {"sessions": 3},
            "observations": "private observation",
        }
        receipt.update(changes)
        (self.attempt / f"{self.first}.json").write_text(json.dumps(receipt))

    def test_reports_status_and_counts_only(self):
        self.create_receipt()
        rows = report.review_receipts(self.attempt, SHA, self.matrix)
        text = report.markdown_table(self.matrix, rows)
        self.assertIn("verified (sessions 3)", text)
        self.assertNotIn("private observation", text)

    def test_cleanup_prefix_lists_distinct_rows_and_excludes_baseline(self):
        result = subprocess.run(
            [sys.executable, report.__file__, self.attempt, SHA, "--row-prefix", "cleanup2-"],
            check=True, capture_output=True, text=True,
        )
        self.assertIn("`cleanup2-map-dualsense-hidapi`", result.stdout)
        self.assertIn("`cleanup2-map-dualsense-evdev`", result.stdout)
        self.assertIn("`cleanup2-timing-dualsense-n1`", result.stdout)
        self.assertIn("`cleanup2-timing-dualsense-n3`", result.stdout)
        self.assertNotIn("`timing-dualsense-n1`", result.stdout)

    def test_resume_preserves_manifest_order_and_rejects_leftovers(self):
        self.create_receipt(status="expected refusal")
        rows = report.review_receipts(self.attempt, SHA, self.matrix)
        self.assertEqual(report.remaining_rows(self.matrix, rows, lambda: False)[0], self.second)
        with self.assertRaisesRegex(ValueError, "still owns devices"):
            report.remaining_rows(self.matrix, rows, lambda: True)

    def test_resume_rejects_wrong_revision_and_unverified_cleanup(self):
        self.create_receipt(sha="b" * 40)
        with self.assertRaisesRegex(ValueError, "wrong row or revision"):
            report.review_receipts(self.attempt, SHA, self.matrix)
        self.create_receipt(cleanup=False)
        with self.assertRaisesRegex(ValueError, "unverified cleanup"):
            report.review_receipts(self.attempt, SHA, self.matrix)
