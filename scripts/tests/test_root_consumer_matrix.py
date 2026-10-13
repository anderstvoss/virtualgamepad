import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch
import contextlib
import io

MODULE = Path(__file__).resolve().parents[1] / 'check-root-consumers.py'
SPEC = importlib.util.spec_from_file_location('root_consumers', MODULE)
consumers = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(consumers)


class RootConsumerMatrixTests(unittest.TestCase):
    def test_help_never_runs_builds(self):
        with patch.object(consumers, 'main') as build, contextlib.redirect_stdout(io.StringIO()) as output:
            with self.assertRaises(SystemExit) as exit_status:
                consumers.cli(['--help'])
            self.assertEqual(exit_status.exception.code, 0)
            self.assertIn('usage:', output.getvalue())
            build.assert_not_called()

    def test_unknown_arguments_never_run_builds(self):
        with patch.object(consumers, 'main') as build, contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit) as exit_status:
                consumers.cli(['--unrecognized'])
            self.assertEqual(exit_status.exception.code, 2)
            build.assert_not_called()

    def test_default_invocation_runs_matrix(self):
        with patch.object(consumers, 'main') as build:
            consumers.cli([])
            build.assert_called_once_with()

    def test_experimental_is_exercised_with_each_audio_combination(self):
        self.assertEqual(consumers.feature_combinations(['experimental', 'audio-usbip', 'audio-pipewire']), [
            '', 'audio-pipewire', 'audio-usbip', 'experimental',
            'audio-pipewire,audio-usbip', 'audio-pipewire,experimental',
            'audio-usbip,experimental', 'audio-pipewire,audio-usbip,experimental',
        ])

    def test_features_are_read_from_manifest_and_alsa_routing_stays_in_demo(self):
        import tomllib
        root = MODULE.parents[1]
        library = tomllib.loads((root/'Cargo.toml').read_text())['features']
        demo = tomllib.loads((root/'demo/Cargo.toml').read_text())['features']
        self.assertEqual(library['default'], [])
        self.assertNotIn('audio-alsa', library)
        self.assertEqual(demo['audio-alsa'], [])
