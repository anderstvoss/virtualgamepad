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
    def test_indexed_markers_detect_period_loss_offset_by_replay(self):
        for family in ('dualsense', 'dualshock4', 'xbox360'):
            channels = lab.PROFILES[family][2]
            pcm = lab.marker_pcm(192000, channels)
            width = 2*channels
            # Balanced corruption inside the measured window. The old 97-frame
            # periodic samples were byte-identical under this exact sequence.
            bad = (pcm[:97000*width] + pcm[97097*width:98000*width]
                   + pcm[98000*width:98097*width] + pcm[98000*width:])
            self.assertEqual(len(bad), len(pcm))
            self.assertNotEqual(bad, pcm)
            def run(command, **kwargs):
                return subprocess.CompletedProcess(command, 0,
                    bad if command[0] == 'arecord' else b'', b'')
            with self.subTest(family=family), patch.object(lab.subprocess, 'run', side_effect=run):
                result = lab.run_trial(1, family, 1)
            self.assertEqual(result['captured_total_frames'], 192000)
            self.assertEqual(result['silence'], 0)
            self.assertFalse(result['passed'])
            self.assertTrue(result['pattern_gaps'] or result['exact_pattern'] < 48000)

    def test_mono_pairs_cross_measurement_edges_and_require_complete_drain(self):
        complete = lab.decode_capture(lab.marker_pcm(4, 1), 1)
        result = lab.capture_summary(complete, 1, 3)
        self.assertEqual((result['exact_pattern'], result['first_source_frame'], result['last_source_frame']),
                         (2, 1, 2))
        incomplete = lab.decode_capture(lab.marker_pcm(3, 1), 1)
        self.assertEqual(lab.capture_summary(incomplete, 1, 3)['exact_pattern'], 1)
        skipped = lab.marker_pcm(2, 1) + lab.marker_pcm(2, 1, 4)
        decoded = lab.decode_capture(skipped, 1)
        self.assertEqual(lab.capture_summary(decoded, 3, 4)['pattern_gaps'], 1)

    def test_indexed_marker_wrap_phase_channels_and_bounded_framing(self):
        for channels, start in [(1, 65532), (2, 32766), (4, 32766)]:
            with self.subTest(channels=channels):
                result = lab.inspect_capture(lab.marker_pcm(4, channels, start), channels)
                self.assertEqual((result['exact_pattern'], result['pattern_gaps']), (4, 0))
                self.assertEqual((result['first_source_frame'], result['last_source_frame']), (start, start+3))
        bad = struct.pack('<4h', 1, 1, -1, 0)
        self.assertEqual(lab.inspect_capture(bad, 4)['exact_pattern'], 0)
        for channels in (0, 3, True):
            with self.assertRaises(ValueError): lab.inspect_capture(bytes(4), channels)
        for position in (-1, lab.MARKER_BASE**2):
            with self.assertRaises(ValueError): lab.marker_frame(position, 2)
        with self.assertRaises(ValueError): lab.marker_pcm(lab.MAX_CAPTURE_FRAMES+1, 2)
        with self.assertRaises(ValueError): lab.marker_pcm(0, 3)
        with self.assertRaises(ValueError): lab.marker_pcm(0, 2, -1)

    def test_partial_mono_pairs_replay_and_reorder_never_prove_continuity(self):
        cases = [lab.marker_pcm(2, 1)*2,
                 lab.marker_pcm(2, 1, 2)+lab.marker_pcm(2, 1),
                 struct.pack('<3h', 1, 1, -1)]
        for data in cases:
            result = lab.inspect_capture(data, 1)
            self.assertTrue(result['pattern_gaps'] or result['exact_pattern'] != result['frames'])

    def test_warmup_is_reported_separately_and_steady_state_loss_still_fails(self):
        for bad_frame, passed in [(0, True), (96000, False)]:
            frames = [lab.marker_frame(i,1)[0] for i in range(192000)]
            frames[bad_frame] = 0
            pcm = struct.pack('<'+'h'*len(frames),*frames)
            def run(command, **kwargs):
                if command[0] == 'arecord':
                    self.assertEqual(command[-1], '4')
                    return subprocess.CompletedProcess(command,0,pcm,b'')
                self.assertEqual(len(kwargs['input']),192000*4)
                return subprocess.CompletedProcess(command,0,b'',b'')
            with self.subTest(bad_frame=bad_frame), patch.object(lab.subprocess,'run',side_effect=run):
                result = lab.run_trial(1,'xbox360',1)
                self.assertEqual(result['passed'],passed)
                self.assertEqual(result['frames'],48000)
                self.assertEqual(result['captured_total_frames'],192000)
                self.assertEqual(result['warmup']['silence'],int(passed))
                self.assertEqual(result['edge_capture_seconds'],1)
                self.assertEqual(result['edge_capture']['frames'],48000)
                self.assertEqual(result['edge_capture']['exact_pattern'],48000)

    def test_only_active_direct_virtual_port_is_accepted(self):
        self.assertEqual(lab.active_device(HEADER+'hs 0000 006 003 00010001 000004 4-1\n',0),(0x10001,'4-1'))
        for row in ['hs 0000 004 000 00000000 000000 0-0',
                    'hs 0000 006 003 00010001 000004 ../../other',
                    'ss 0000 006 003 00010001 000004 4-1',
                    'hs 0001 006 003 00010001 000004 4-1']:
            with self.subTest(row=row), self.assertRaises(ValueError):
                lab.active_device(HEADER+row,0)

    def test_capture_checks_ramp_wrap_channel_isolation_and_frame_loss(self):
        frames=[lab.marker_frame(32700+i,2) for i in range(150)]
        encode=lambda values: b''.join(struct.pack('<2h',*frame) for frame in values)
        result=lab.inspect_capture(encode(frames),2)
        self.assertEqual(result,dict(frames=150,exact_pattern=150,silence=0,pattern_gaps=0,
                                    first_unexpected_frame=None,last_unexpected_frame=None,
                                    first_pattern_gap=None,last_pattern_gap=None,
                                    first_source_frame=32700,last_source_frame=32849))
        skipped=lab.inspect_capture(encode(frames[:50]+frames[51:]),2)
        self.assertEqual((skipped['pattern_gaps'],skipped['first_pattern_gap'],skipped['last_pattern_gap']), (1,50,50))
        self.assertEqual(lab.inspect_capture(encode([(100,0)]),2)['exact_pattern'],0)
        self.assertEqual(lab.inspect_capture(bytes(8),2)['silence'],2)
        self.assertEqual(lab.inspect_capture(bytes(8),2)['first_unexpected_frame'],0)
        self.assertEqual(lab.inspect_capture(bytes(8),2)['last_unexpected_frame'],1)
        with self.assertRaises(ValueError):
            lab.inspect_capture(bytes(3),2)

    def test_pre_attachment_isolation_is_required_for_integer_and_string_cards(self):
        properties = dict(ACP_IGNORE='1', VG_ALPHA_AUDIO_INSTANCE='lab')
        lab.isolated_card(properties, [], 7, 'lab')
        for bad in [{}, dict(ACP_IGNORE='0', VG_ALPHA_AUDIO_INSTANCE='lab'),
                    dict(ACP_IGNORE='1', VG_ALPHA_AUDIO_INSTANCE='foreign')]:
            with self.assertRaises(ValueError): lab.isolated_card(bad, [], 7, 'lab')
        for card in [7, '7']:
            for kind in ['PipeWire:Interface:Device', 'PipeWire:Interface:Node']:
                imported = dict(type=kind, info=dict(props={'api.alsa.card': card}))
                with self.assertRaisesRegex(ValueError, 'imported'):
                    lab.isolated_card(properties, [imported], 7, 'lab')

    def test_session_serial_rejects_malformed_or_zero_generation(self):
        self.assertEqual(lab.compiled_serial('lab', 7), 'vg-lab-0000000000000007')
        for instance, generation in [('', 1), ('../escape', 1), ('lab', 0),
                                     ('lab', True), ('lab', 2**64)]:
            with self.assertRaises(ValueError): lab.compiled_serial(instance, generation)

    def test_shared_defaults_are_collected_without_profile_or_route_mutations(self):
        values = [dict(subject=0, key='default.audio.sink', value='synthetic-a'),
                  dict(subject=0, key='unrelated', value='ignored')]
        metadata = dict(type='PipeWire:Interface:Metadata', props={'metadata.name':'default'}, metadata=values)
        self.assertEqual(lab.shared_defaults([metadata]), values[:1])

    def test_progress_is_visible_before_blocking_trial(self):
        output = io.StringIO()
        def trial(*_):
            self.assertEqual(json.loads(output.getvalue().splitlines()[0])['status'],'running')
            return {'passed':True}
        with patch('sys.argv',
                          ['validator','--profile','xbox360','--port','0','--instance','lab','--generation','7','--trials','1']), \
             patch('sys.stdout',output), \
             patch.object(lab,'resolve_card',return_value=((1,'4-1'),1)), \
             patch.object(lab,'reserve_direct_alsa',return_value={'shared_defaults': []}), \
             patch.object(lab,'run_trial',side_effect=trial):
            self.assertEqual(lab.main(),0)

    def test_prepare_only_reserves_exact_owned_card_without_streaming(self):
        output = io.StringIO()
        with patch('sys.argv',['validator','--profile','dualsense','--port','1','--instance','lab','--generation','7',
                               '--reserve-owned-card','--prepare-only']), \
             patch('sys.stdout',output), \
             patch.object(lab,'resolve_card',return_value=((17,'4-2'),9)), \
             patch.object(lab,'reserve_direct_alsa') as reserve, \
             patch.object(lab,'run_trial') as trial:
            self.assertEqual(lab.main(),0)
        reserve.assert_called_once_with(9,'4-2','lab',7)
        trial.assert_not_called()
        self.assertEqual(json.loads(output.getvalue())['card'],9)

    def test_disappearance_preserves_pcm_result_and_stops_trials(self):
        output = io.StringIO()
        with patch('sys.argv',['validator','--profile','xbox360','--port','0','--instance','lab','--generation','7']), \
             patch('sys.stdout',output), \
             patch.object(lab,'resolve_card',side_effect=[((1,'4-1'),1),
                          ValueError('selected port has no active high-speed lab session')]), \
             patch.object(lab,'reserve_direct_alsa',return_value={'shared_defaults': []}), \
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
