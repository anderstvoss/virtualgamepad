"""The physical probe must never write trigger, motor or LED fields."""

import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / 'probe-dualsense-speaker.py'
SPEC = importlib.util.spec_from_file_location('speaker_probe', SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class SpeakerReportTest(unittest.TestCase):
    def test_speaker_route_only_sets_audio_fields(self):
        packet = MODULE.report(True)
        self.assertEqual(len(packet), 63)
        self.assertEqual({i: byte for i, byte in enumerate(packet) if byte},
                         {0: 2, 1: 0xa0, 2: 0x80, 6: 0x64, 8: 0x30, 38: 2})

    def test_restore_only_sets_audio_route(self):
        packet = MODULE.report(False)
        self.assertEqual({i: byte for i, byte in enumerate(packet) if byte},
                         {0: 2, 1: 0x80})

    def test_tone_uses_only_right_audible_channel(self):
        import struct
        frames = list(struct.iter_unpack('<hhhh', MODULE.tone()))
        self.assertEqual(len(frames), 48000)
        self.assertTrue(any(right for _, right, _, _ in frames))
        self.assertTrue(all(left == grip_l == grip_r == 0
                            for left, _, grip_l, grip_r in frames))

    def test_playback_failure_still_restores_headphone_route(self):
        target = (Path('/physical/controller'), 1, Path('/dev/fakehidraw'))
        writes = []

        def write(_, packet):
            writes.append(packet)
            return len(packet)

        with patch.object(MODULE, 'physical_devices', return_value=target), \
             patch.object(MODULE.os, 'open', return_value=3), \
             patch.object(MODULE.os, 'write', side_effect=write), \
             patch.object(MODULE.os, 'close'), \
             patch.object(MODULE.subprocess, 'run', side_effect=RuntimeError('ALSA')):
            with self.assertRaisesRegex(RuntimeError, 'ALSA'):
                MODULE.probe()
        self.assertEqual(writes, [MODULE.report(True), MODULE.report(False)])


if __name__ == '__main__':
    unittest.main()
