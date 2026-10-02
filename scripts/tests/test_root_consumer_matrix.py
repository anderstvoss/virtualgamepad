import importlib.util
from pathlib import Path
import unittest

MODULE = Path(__file__).resolve().parents[1] / 'check-root-consumers.py'
SPEC = importlib.util.spec_from_file_location('root_consumers', MODULE)
consumers = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(consumers)


class RootConsumerMatrixTests(unittest.TestCase):
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
