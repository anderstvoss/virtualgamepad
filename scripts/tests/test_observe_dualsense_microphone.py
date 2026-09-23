import importlib.util
from pathlib import Path
import struct
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('physical_mic',Path(__file__).parents[1]/'observe-dualsense-microphone.py')
lab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lab)


class PhysicalMicrophoneTests(unittest.TestCase):
    def test_levels_are_channel_separate_and_require_complete_frames(self):
        values = struct.pack('<hhhhhhhh',100,0,-100,0,100,200,-100,-200)
        self.assertEqual(lab.levels(values),dict(frames=4,rms=[100.0,141.42],peak=[100,200]))
        with self.assertRaises(ValueError): lab.levels(values[:-1])

    def test_resolution_rejects_ambiguous_and_noncontroller_cards(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            usb = root/'usb'; sound = root/'sound'
            usb.mkdir(); sound.mkdir()
            reference = usb/'1-2'; reference.mkdir()
            (reference/'idVendor').write_text('054c\n')
            (reference/'idProduct').write_text('0ce6\n')
            other = usb/'1-3'; other.mkdir()
            (other/'idVendor').write_text('9999\n')
            (other/'idProduct').write_text('0001\n')
            card = sound/'card7'; card.mkdir()
            (card/'device').symlink_to(other,target_is_directory=True)
            with self.assertRaisesRegex(ValueError,'ALSA card'):
                lab.physical_card(usb,sound)
            (card/'device').unlink()
            (card/'device').symlink_to(reference,target_is_directory=True)
            self.assertEqual(lab.physical_card(usb,sound),(reference.resolve(),7))
            vhci = root/'vhci_hcd.0'/'usb4'/'4-1'; vhci.mkdir(parents=True)
            (vhci/'idVendor').write_text('054c\n')
            (vhci/'idProduct').write_text('0ce6\n')
            (usb/'4-1').symlink_to(vhci,target_is_directory=True)
            self.assertEqual(lab.physical_card(usb,sound),(reference.resolve(),7))
            second = usb/'1-4'; second.mkdir()
            (second/'idVendor').write_text('054c\n')
            (second/'idProduct').write_text('0ce6\n')
            with self.assertRaisesRegex(ValueError,'exactly one attached'):
                lab.physical_card(usb,sound)
