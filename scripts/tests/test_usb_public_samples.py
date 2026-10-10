import tempfile
import unittest
import os
from unittest.mock import Mock, patch
from pathlib import Path

ROOT = Path(__file__).parents[1]
namespace = {'__name__': 'captured_fixture'}
exec(compile((ROOT/'validate-usb-audio-live.py').read_text(), 'captured-validator', 'exec'), namespace)
exec(compile((ROOT/'validate-usb-public-samples.py').read_text(), 'captured-public-helper', 'exec'), namespace)


class PublicSampleIdentity(unittest.TestCase):
    def fixture(self, root):
        sound = root/'sound';sound.mkdir()
        hub = root/'vhci_hcd.0'/'usb4';hub.mkdir(parents=True)
        (hub/'serial').write_text('vhci_hcd.0')
        device = hub/'4-1';device.mkdir()
        (device/'serial').write_text('vg-test-lab-0000000000000001')
        (device/'manufacturer').write_text('Virtualgamepad')
        (device/'product').write_text(namespace['PROFILES']['dualsense'][0])
        interface = device/'4-1:1.0';interface.mkdir()
        card = sound/'card1';card.mkdir();(card/'device').symlink_to(interface)
        (card/'id').write_text('sanitized_card')
        return sound, device, card

    def test_compiled_entrypoint_checks_isolation_before_and_after_exchange(self):
        with tempfile.TemporaryDirectory(prefix='virtualgamepad-root-graph-') as directory:
            root = Path(directory)
            sound, device, _ = self.fixture(root)
            isolation = Mock(return_value={'serial': 'vg-test-lab-0000000000000001', 'shared_defaults': []})
            trial = Mock(return_value={'passed': True})
            original_lookup = namespace['public_card']
            lookup = lambda *args: original_lookup(*args, sound=sound)
            with patch.dict(os.environ, PIPEWIRE_RUNTIME_DIR=directory, XDG_RUNTIME_DIR=directory), \
                 patch.object(os, 'geteuid', return_value=1000), \
                 patch('sys.argv', ['fixture', '--family', 'dualsense', '--card-id', 'sanitized_card', '--instance', 'test-lab']), \
                 patch.dict(namespace, public_card=lookup, reserve_direct_alsa=isolation, run_trial=trial), \
                 patch('builtins.print') as output:
                self.assertEqual(namespace['public_samples_main'](), 0)
                self.assertEqual(isolation.call_count, 2)
                isolation.assert_called_with(1, device.name, 'test-lab', 1)
                trial.assert_called_once_with(1, 'dualsense', 3)
                self.assertEqual(output.call_args_list[0].args, ('R',))

    def test_exact_card_and_session_identity_excludes_root_hub_serial(self):
        with tempfile.TemporaryDirectory() as directory:
            sound, device, _ = self.fixture(Path(directory))
            self.assertEqual(namespace['public_card']('sanitized_card','dualsense','test-lab',sound),
                             (1,device,'vg-test-lab-0000000000000001'))
            for invalid in ('default', 'hw:0', '../card1', 'card;command', ''):
                with self.assertRaises(ValueError):namespace['public_card'](invalid,'dualsense','test-lab',sound)

    def test_foreign_serial_family_and_duplicate_card_reject_before_streaming(self):
        with tempfile.TemporaryDirectory() as directory:
            sound, device, card = self.fixture(Path(directory))
            lookup = lambda: namespace['public_card']('sanitized_card','dualsense','test-lab',sound)
            for serial in ['vg-foreign-0000000000000001','vg-test-lab-0000000000000000']:
                (device/'serial').write_text(serial)
                with self.assertRaises(ValueError):lookup()
            (device/'serial').write_text('vg-test-lab-0000000000000001')
            with self.assertRaises(ValueError):namespace['public_card']('sanitized_card','dualshock4','test-lab',sound)
            duplicate = sound/'card2';duplicate.mkdir();(duplicate/'id').write_text('sanitized_card')
            with self.assertRaises(ValueError):lookup()
            self.assertTrue(card.exists())

    def test_missing_vhci_ancestry_rejects_even_matching_manufacturer_and_serial(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);sound,device,_=self.fixture(root)
            (root/'vhci_hcd.0').rename(root/'foreign-controller')
            card=sound/'card1';(card/'device').unlink();(card/'device').symlink_to(root/'foreign-controller/usb4/4-1/4-1:1.0')
            with self.assertRaises(ValueError):namespace['public_card']('sanitized_card','dualsense','test-lab',sound)


if __name__ == '__main__':unittest.main()
