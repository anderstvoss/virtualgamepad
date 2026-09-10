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

    def run_probe(self, scenario, *options):
        result = subprocess.run([str(self.binary), 'synthetic-device', '300', *options], env=dict(os.environ, PROBE_SCENARIO=scenario), capture_output=True, text=True)
        records = [json.loads(line) for line in result.stdout.splitlines()]
        record = records[-1]
        COMPARE.normalize(record)
        return result, record

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
