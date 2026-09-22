import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('audio_install', Path(__file__).parents[1] / 'install-audio-broker.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class AudioInstallerTests(unittest.TestCase):
    def test_policy_is_closed_and_separates_worker_from_client(self):
        self.assertIn(b'allow_vhci_port=2\n', module.configuration(1000, 900, 900, [0, 2]))
        for args in [(0, 1, 1, [0]), (1, 1, 1, [0]), (1, 2, 0, [0]), (1, 2, 2, []), (1, 2, 2, [0, 0]), (1, 2, 2, [-1]), (1, 2, 2, [65536])]:
            with self.assertRaises(ValueError):
                module.configuration(*args)

    def test_service_has_only_fixed_paths_and_required_credentials(self):
        unit = module.service()
        self.assertIn(b'CAP_SETUID CAP_SETGID', unit)
        self.assertIn(b'NoNewPrivileges=true', unit)
        self.assertIn(b'/sys/devices/platform/vhci_hcd.0/attach', unit)
        self.assertNotIn(b'/dev/snd', unit)
        self.assertNotIn(b'ExecStartPre', unit)
        self.assertIn(b'Restart=no', unit)

    def test_socket_group_cannot_inject_unit_settings(self):
        self.assertIn(b'SocketGroup=test-group', module.socket_unit('test-group'))
        for group in ['', 'test\nSocketMode=0666', '../root', 'a b']:
            with self.assertRaises(ValueError):
                module.socket_unit(group)
