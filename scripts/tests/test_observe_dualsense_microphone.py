"""Physical microphone gain checks must restore the original ALSA control."""

import importlib.util
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / 'observe-dualsense-microphone.py'
SPEC = importlib.util.spec_from_file_location('microphone_probe', SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class CaptureGainTest(unittest.TestCase):
    def test_parses_single_bounded_capture_gain(self):
        stdout = ('numid=6,iface=MIXER,name=\'Headset Capture Volume\'\n'
                  '  ; type=INTEGER,access=rw---R--,values=1,min=0,max=101,step=0\n'
                  '  : values=31\n')
        with patch.object(MODULE.subprocess, 'run',
                          return_value=SimpleNamespace(stdout=stdout)):
            self.assertEqual(MODULE.capture_gain(1), (31, 0, 101))

    def test_gain_is_restored_after_capture_failure(self):
        device = Path('/physical/controller')
        writes = []

        def set_gain(_, __, value):
            writes.append(value)

        with patch.object(MODULE, 'physical_card', return_value=(device, 1)), \
             patch.object(MODULE, 'capture_gain', return_value=(31, 0, 101)), \
             patch.object(MODULE, 'capture', side_effect=[{'frames': 144000},
                                                           RuntimeError('capture failed')]), \
             patch.object(MODULE, 'set_capture_gain', side_effect=set_gain):
            with self.assertRaisesRegex(RuntimeError, 'capture failed'):
                MODULE.compare_gain(3)
        self.assertEqual(writes, [101, 31])

    def test_cues_touch_only_selected_audio_channel(self):
        import struct
        device = Path('/physical/controller')
        calls = []

        def run(*args, **kwargs):
            calls.append((args, kwargs))

        with patch.object(MODULE, 'physical_card', return_value=(device, 1)), \
             patch.object(MODULE.subprocess, 'run', side_effect=run):
            MODULE.play_cue(device, 1)
            MODULE.play_cue(device, 1, grip=True)
        self.assertEqual(len(calls), 2)
        for call, active in zip(calls, [0, 2]):
            frames = list(struct.iter_unpack('<hhhh', call[1]['input']))
            self.assertEqual(len(frames), 24000)
            self.assertTrue(any(frame[active] for frame in frames))
            self.assertTrue(all(all(sample == 0 for index, sample in enumerate(frame)
                                if index != active) for frame in frames))

    def test_signal_comparison_restores_gain_after_capture_failure(self):
        device = Path('/physical/controller')
        writes = []

        def set_gain(_, __, value):
            writes.append(value)

        with patch.object(MODULE, 'physical_card', return_value=(device, 1)), \
             patch.object(MODULE, 'capture_gain', return_value=(31, 0, 101)), \
             patch.object(MODULE, 'capture', side_effect=[{'frames': 144000},
                                                           RuntimeError('capture failed')]), \
             patch.object(MODULE, 'play_cue'), \
             patch.object(MODULE, 'set_capture_gain', side_effect=set_gain):
            with self.assertRaisesRegex(RuntimeError, 'capture failed'):
                MODULE.compare_signal(3)
        self.assertEqual(writes, [101, 31])


if __name__ == '__main__':
    unittest.main()
