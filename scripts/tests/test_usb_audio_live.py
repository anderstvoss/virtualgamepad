import importlib.util
from pathlib import Path
import struct
import unittest
import io
import json
import subprocess
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('usb_audio_live',Path(__file__).parents[1]/'validate-usb-audio-live.py')
lab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lab)
HEADER = 'hub port sta spd dev sockfd local_busid\n'


class LiveHarness(unittest.TestCase):
    def test_warmup_is_reported_separately_and_steady_state_loss_still_fails(self):
        for bad_frame, passed in [(0, True), (96000, False)]:
            frames = [100 + i % 97 for i in range(144000)]
            frames[bad_frame] = 0
            pcm = struct.pack('<'+'h'*len(frames),*frames)
            def run(command, **kwargs):
                if command[0] == 'arecord':
                    self.assertEqual(command[-1], '3')
                    return subprocess.CompletedProcess(command,0,pcm,b'')
                self.assertEqual(len(kwargs['input']),144000*4)
                return subprocess.CompletedProcess(command,0,b'',b'')
            with self.subTest(bad_frame=bad_frame), patch.object(lab.subprocess,'run',side_effect=run):
                result = lab.run_trial(1,'xbox360',1)
                self.assertEqual(result['passed'],passed)
                self.assertEqual(result['frames'],48000)
                self.assertEqual(result['captured_total_frames'],144000)
                self.assertEqual(result['warmup']['silence'],int(passed))

    def test_only_active_direct_virtual_port_is_accepted(self):
        self.assertEqual(lab.active_device(HEADER+'hs 0000 006 003 00010001 000004 4-1\n',0),(0x10001,'4-1'))
        for row in ['hs 0000 004 000 00000000 000000 0-0',
                    'hs 0000 006 003 00010001 000004 ../../other',
                    'ss 0000 006 003 00010001 000004 4-1',
                    'hs 0001 006 003 00010001 000004 4-1']:
            with self.subTest(row=row), self.assertRaises(ValueError):
                lab.active_device(HEADER+row,0)

    def test_capture_checks_ramp_wrap_channel_isolation_and_frame_loss(self):
        frames=[(100+i%97,200+i%97) for i in range(150)]
        encode=lambda values: b''.join(struct.pack('<2h',*frame) for frame in values)
        result=lab.inspect_capture(encode(frames),2)
        self.assertEqual(result,dict(frames=150,exact_pattern=150,silence=0,pattern_gaps=0,
                                    first_unexpected_frame=None,last_unexpected_frame=None))
        self.assertEqual(lab.inspect_capture(encode(frames[:50]+frames[51:]),2)['pattern_gaps'],1)
        self.assertEqual(lab.inspect_capture(encode([(100,100)]),2)['exact_pattern'],0)
        self.assertEqual(lab.inspect_capture(bytes(8),2)['silence'],2)
        self.assertEqual(lab.inspect_capture(bytes(8),2)['first_unexpected_frame'],0)
        self.assertEqual(lab.inspect_capture(bytes(8),2)['last_unexpected_frame'],1)
        with self.assertRaises(ValueError):
            lab.inspect_capture(bytes(3),2)

    def test_progress_is_visible_before_blocking_trial(self):
        output = io.StringIO()
        def trial(*_):
            self.assertEqual(json.loads(output.getvalue().splitlines()[0])['status'],'running')
            return {'passed':True}
        with patch('sys.argv',
                          ['validator','--profile','xbox360','--port','0','--trials','1']), \
             patch('sys.stdout',output), \
             patch.object(lab,'resolve_card',return_value=((1,'4-1'),1)), \
             patch.object(lab,'run_trial',side_effect=trial):
            self.assertEqual(lab.main(),0)

    def test_prepare_only_reserves_exact_owned_card_without_streaming(self):
        output = io.StringIO()
        with patch('sys.argv',['validator','--profile','dualsense','--port','1',
                               '--reserve-owned-card','--prepare-only']), \
             patch('sys.stdout',output), \
             patch.object(lab,'resolve_card',return_value=((17,'4-2'),9)), \
             patch.object(lab,'reserve_direct_alsa') as reserve, \
             patch.object(lab,'run_trial') as trial:
            self.assertEqual(lab.main(),0)
        reserve.assert_called_once_with(9,'4-2')
        trial.assert_not_called()
        self.assertEqual(json.loads(output.getvalue())['card'],9)

    def test_disappearance_preserves_pcm_result_and_stops_trials(self):
        output = io.StringIO()
        with patch('sys.argv',['validator','--profile','xbox360','--port','0']), \
             patch('sys.stdout',output), \
             patch.object(lab,'resolve_card',side_effect=[((1,'4-1'),1),
                          ValueError('selected port has no active high-speed lab session')]), \
             patch.object(lab,'run_trial',return_value={'passed':False,
                          'capture_status':1,'frames':1200}) as trial:
            self.assertEqual(lab.main(),1)
        trial.assert_called_once()
        result = json.loads(output.getvalue().splitlines()[-1])
        self.assertFalse(result['session_present_after_trial'])
        self.assertFalse(result['passed'])
        self.assertEqual(result['frames'],1200)
        self.assertEqual(result['capture_status'],1)
        self.assertIn('no active',result['session_error'])


if __name__ == '__main__':
    unittest.main()
