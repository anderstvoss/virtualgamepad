"""Exercise the actual C probe with deterministic fake SDL, without devices."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('probe_comparison', ROOT / 'compare-sdl3-observations.py')
COMPARE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(COMPARE)


class ProbeIntegration(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory()
        cls.binary = Path(cls.directory.name) / 'probe'
        subprocess.run(['cc', '-std=c17', '-Wall', '-Wextra', '-Werror', '-I', str(ROOT / 'tests/fixtures'), str(ROOT / 'sdl3-gamepad-probe.c'), '-o', str(cls.binary)], check=True)

    @classmethod
    def tearDownClass(cls):
        cls.directory.cleanup()

    def run_probe(self, scenario, *options, evidence=None):
        env = {key: value for key, value in os.environ.items() if not key.startswith(('PROBE_', 'SDL_'))}
        env.update(PROBE_SCENARIO=scenario)
        env.update(evidence or {})
        result = subprocess.run([str(self.binary), env.get('PROBE_PATH', 'synthetic-device'), '300', *options], env=env, capture_output=True, text=True)
        records = [json.loads(line) for line in result.stdout.splitlines()]
        record = records[-1]
        COMPARE.normalize(record)
        return result, record

    def test_reviewed_guid_and_path_identify_backend_independently_of_hint(self):
        # Synthetic GUIDs use fake VID/PID 1:2, never captures from user devices.
        for guid, path, backend, conflicting_hint in [
            ('03000000010000000200000000006800', '/dev/hidraw7', 'hidapi', '0'),
            ('03000000010000000200000000000000', '/dev/input/event12', 'linux-evdev', '1'),
        ]:
            result, record = self.run_probe('normal', evidence={
                'PROBE_REVISION': 'SDL3-3.2.0-release-3.2.0', 'PROBE_GUID': guid,
                'PROBE_PATH': path, 'SDL_JOYSTICK_HIDAPI': conflicting_hint,
            })
            self.assertEqual(result.returncode, 0)
            self.assertEqual(record['observations']['backend']['value'], backend)
            self.assertEqual(record['observations']['backend_request']['value'], conflicting_hint)
            self.assertIn('source-derived', record['backend_evidence']['method'])
            self.assertIsNone(record['observations']['mapping_source']['value'])

    def test_ambiguous_backend_and_failed_open_remain_unmeasured(self):
        baseline = {'PROBE_REVISION': 'SDL3-3.2.0-release-3.2.0',
                    'PROBE_GUID': '03000000010000000200000000006800', 'PROBE_PATH': '/dev/hidraw7'}
        for override in [
            {'PROBE_REVISION': 'synthetic-build'}, {'PROBE_VERSION': '3004000'},
            {'PROBE_GUID': 'synthetic-guid'}, {'PROBE_GUID': '03000000000000000200000000006800'},
            {'PROBE_PATH': '/dev/hidraw7-extra'}, {'PROBE_PATH': '/dev/input/event7'},
            {'PROBE_SCENARIO': 'open-fail'},
        ]:
            _, record = self.run_probe('normal', evidence=baseline | override)
            self.assertIsNone(record['observations']['backend']['value'])
            self.assertTrue(record['observations']['backend']['reason'])
            self.assertIsNone(record['backend_evidence'])

    def test_selection_and_open_failures_never_claim_close(self):
        for scenario in ['absent', 'duplicate', 'init-fail', 'open-fail']:
            result, record = self.run_probe(scenario)
            self.assertEqual(result.returncode, 1)
            self.assertFalse(record['consumer_closed'])
            self.assertFalse(record['observations']['opened']['value'])
            self.assertIsNone(record['observations']['capabilities']['value'])

    def test_successful_reopen_and_failed_reopen_record_actual_calls(self):
        result, record = self.run_probe('normal', '--reopen')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('fake_closes=2', result.stderr)
        for field in ['opened', 'closed', 'reopened', 'reclosed']:
            self.assertTrue(record['observations'][field]['value'])
        result, record = self.run_probe('reopen-fail', '--reopen')
        self.assertEqual(result.returncode, 1)
        self.assertIn('fake_closes=1', result.stderr)
        self.assertFalse(record['observations']['reopened']['value'])
        self.assertFalse(record['observations']['reclosed']['value'])
        self.assertTrue(record['consumer_closed'])

    def test_extra_button_touch_and_output_call_are_separate_observations(self):
        result, record = self.run_probe('events')
        self.assertEqual(result.returncode, 0)
        obs = record['observations']
        self.assertIn('left_paddle1', obs['capabilities']['value']['buttons'])
        self.assertEqual(obs['controls']['value']['down_mask'], 1 << 20)
        self.assertEqual(obs['touch']['value']['down_mask'], 2)
        self.assertTrue(obs['output_calls']['value']['rumble'])
        self.assertFalse(obs['output_calls']['value']['led'])
        self.assertIsNone(obs['controller_reverse']['value'])
        self.assertIsNone(obs['device_removed']['value'])

    def test_existing_control_mode_and_new_reopen_flag_compose(self):
        result, record = self.run_probe('normal', '--control-0', '--reopen')
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertIn('"record_type":"mapping_ready"', result.stdout)
        self.assertEqual(record['profile'], '--control-0')
        self.assertIsNone(record['observations']['output_calls']['value'])
        self.assertTrue(record['observations']['reclosed']['value'])
        for options in [('--reopen', '--reopen'), ('--bad-option',), ('--control-0', '--control-1')]:
            result, record = self.run_probe('normal', *options)
            self.assertEqual(result.returncode, 1)
            self.assertFalse(record['consumer_closed'])
