import importlib.util
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'compare-sdl3-observations.py'
SPEC = importlib.util.spec_from_file_location('sdl_compare', SCRIPT)
COMPARE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(COMPARE)


def capture():
    record = dict(schema_version=2, consumer='SDL', selected_count=1, passed=True,
                  profile='--control-1', control_case=1, observations={})
    for field in COMPARE.FIELDS:
        record['observations'][field] = dict(value=None, reason='not measured in synthetic fixture')
    for field, value in dict(identity=dict(vendor=1, product=2, guid='synthetic'),
                             build=dict(version=3002000, revision='synthetic-build'),
                             capabilities=dict(buttons=['south', 'left_paddle1'], axes=['leftx'], touchpads=1, rumble=True, rgb_led=False),
                             controls=dict(down_mask=1, up_mask=1, axis_mask=0, final_buttons=0, final_axes=[0]*6),
                             output_calls=dict(rumble=True, led=False), opened=True, closed=True).items():
        record['observations'][field] = dict(value=value, reason=None)
    return record


class ComparisonTests(unittest.TestCase):
    def classifications(self, result):
        return {item['field']: item['classification'] for item in result['differences']}

    def test_equal_partial_capture_never_becomes_complete_pass(self):
        result = COMPARE.compare(capture(), capture())
        self.assertEqual(result['counts']['unexpected difference'], 0)
        self.assertGreater(result['counts']['not measured'], 0)
        self.assertEqual(self.classifications(result)['controller_reverse'], 'not measured')
        self.assertEqual(self.classifications(result)['device_removed'], 'not measured')

    def test_backend_inference_provenance_survives_comparison(self):
        left, right = capture(), capture()
        left['observations']['backend'] = dict(value='hidapi', reason=None)
        right['observations']['backend'] = dict(value='hidapi', reason=None)
        left['backend_evidence'] = dict(method='source-derived synthetic test', source_revision='synthetic')
        result = COMPARE.compare(left, right)
        self.assertEqual(self.classifications(result)['backend'], 'match')
        self.assertEqual(result['backend_evidence']['reference'], left['backend_evidence'])
        self.assertIsNone(result['backend_evidence']['virtual'])
        self.assertEqual(self.classifications(result)['mapping_source'], 'not measured')

    def test_extra_buttons_and_exact_evidence_linked_limitations(self):
        left, right = capture(), capture()
        right['observations']['capabilities']['value']['buttons'] = ['south']
        path = 'capabilities.buttons'
        self.assertEqual(self.classifications(COMPARE.compare(left, right))[path], 'unexpected difference')
        rule = dict(field=path, reference=['south', 'left_paddle1'], virtual=['south'],
                    reason='Synthetic target cannot expose paddle', evidence='synthetic-experiment')
        self.assertEqual(self.classifications(COMPARE.compare(left, right, [rule]))[path], 'expected realization limitation')
        right['observations']['capabilities']['value']['buttons'] = []
        self.assertEqual(self.classifications(COMPARE.compare(left, right, [rule]))[path], 'unexpected difference')
        del rule['evidence']
        with self.assertRaises(ValueError):
            COMPARE.compare(left, right, [rule])

    def test_missing_evidence_and_bad_types_cannot_match(self):
        left, right = capture(), capture()
        right['observations']['closed'] = dict(value=None, reason='not observed')
        self.assertEqual(self.classifications(COMPARE.compare(left, right))['closed'], 'not measured')
        rule = dict(field='closed', reference=True, virtual=None, reason='missing', evidence='fixture')
        with self.assertRaises(ValueError):
            COMPARE.compare(left, right, [rule])
        for measurement in [dict(value=None, reason=None), dict(value='true', reason=None), dict(value=True, reason='unavailable')]:
            right['observations']['closed'] = measurement
            with self.assertRaises(ValueError):
                COMPARE.compare(left, right)

    def test_failed_open_and_two_failures_are_not_compatibility(self):
        failed = capture()
        failed.update(selected_count=0, passed=False)
        failed['observations']['opened']['value'] = False
        failed['observations']['closed']['value'] = False
        result = COMPARE.compare(failed, failed)
        self.assertEqual(result['counts']['unexpected difference'], 2)

    def test_different_scenarios_and_builds_are_visible(self):
        left, right = capture(), capture()
        right['profile'] = '--control-2'
        right['control_case'] = 2
        result = self.classifications(COMPARE.compare(left, right))
        self.assertEqual(result['comparison_context'], 'unexpected difference')
        self.assertEqual(result['controls'], 'not measured')
        right = capture()
        right['observations']['build']['value']['revision'] = 'different-build'
        self.assertEqual(self.classifications(COMPARE.compare(left, right))['build.revision'], 'unexpected difference')

    def test_version_one_close_is_untrusted_and_unknown_schema_rejected(self):
        old = dict(schema_version=1, consumer='SDL', selected_count=1, passed=True,
                   profile='diagnostic', consumer_closed=True, consumer_version=3002000,
                   consumer_revision='synthetic', guid='synthetic', vendor=1, product=2)
        self.assertIsNone(COMPARE.normalize(old)['closed']['value'])
        old['schema_version'] = 99
        with self.assertRaises(ValueError):
            COMPARE.normalize(old)

    def test_reopen_failure_differs_from_not_attempted(self):
        left, right = capture(), capture()
        left['observations']['reopened'] = dict(value=True, reason=None)
        self.assertEqual(self.classifications(COMPARE.compare(left, right))['reopened'], 'not measured')
        right['observations']['reopened'] = dict(value=False, reason=None)
        self.assertEqual(self.classifications(COMPARE.compare(left, right))['reopened'], 'unexpected difference')

    def test_external_reverse_cleanup_evidence_is_bound_to_exact_capture(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            capture_path, evidence_path = root / 'capture.json', root / 'evidence.json'
            record = capture()
            capture_path.write_text(json.dumps(record, indent=2))
            evidence = dict(schema_version=1, capture_sha256=hashlib.sha256(capture_path.read_bytes()).hexdigest(),
                            evidence='synthetic-owned-session-test', observations=dict(controller_reverse=dict(rumble_seen=True), device_removed=True, backend='synthetic'))
            evidence_path.write_text(json.dumps(evidence))
            locator = COMPARE.attach_evidence(record, capture_path, evidence_path)
            self.assertEqual(locator, evidence['evidence'])
            self.assertTrue(record['observations']['device_removed']['value'])
            self.assertEqual(COMPARE.load(capture_path)['schema_version'], 2)
            with self.assertRaises(ValueError):
                COMPARE.attach_evidence(record, capture_path, evidence_path)
            capture_path.write_text(json.dumps(capture()) + '\n')
            with self.assertRaises(ValueError):
                COMPARE.attach_evidence(capture(), capture_path, evidence_path)

    def test_cli_jsonl_readiness_and_exit_codes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            paths = [root / 'reference.jsonl', root / 'virtual.jsonl']
            for path in paths:
                path.write_text(json.dumps(dict(schema_version=2, record_type='mapping_ready')) + '\n' + json.dumps(capture()) + '\n')
            result = subprocess.run(['python3', str(SCRIPT), *map(str, paths)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertGreater(json.loads(result.stdout)['counts']['not measured'], 0)
            paths[1].write_text(json.dumps(capture()) + '\n' + json.dumps(capture()))
            result = subprocess.run(['python3', str(SCRIPT), *map(str, paths)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 2)
