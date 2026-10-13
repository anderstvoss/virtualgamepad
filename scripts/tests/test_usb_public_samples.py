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

    def test_native_microphone_requires_exact_capture_and_successful_source(self):
        for scenario in ('complete', 'balanced-loss-replay', 'incomplete', 'source-failed'):
            with self.subTest(scenario=scenario):
                captured = bytearray(namespace['marker_pcm'](288000, 1))
                if scenario == 'balanced-loss-replay':
                    # Replace a complete, individually valid mono pair with its predecessor.
                    captured[240000:240004] = captured[239996:240000]
                if scenario == 'incomplete':
                    captured = captured[:-2]
                def run(command, **options):
                    if command[0] == 'arecord':
                        self.assertEqual(command[-2:], ['-d', '6'])
                        return Mock(returncode=0, stdout=bytes(captured), stderr=b'')
                    channels = 1 if command[0] == 'pw-cat' else 2
                    self.assertEqual(len(options['input']), 288000 * channels * 2)
                    self.assertEqual(options['timeout'], 14)
                    status = 1 if scenario == 'source-failed' and command[0] == 'pw-cat' else 0
                    return Mock(returncode=status, stdout=b'', stderr=b'')
                identity = Mock(return_value=(1, 11))
                with patch.dict(namespace, native_microphone_identity=identity), \
                     patch.object(namespace['subprocess'], 'run', side_effect=run):
                    result = namespace['run_native_microphone_trial'](1, 'dualshock4', 'virtualgamepad.12.1.microphone')
                self.assertEqual(result['passed'], scenario == 'complete')
                self.assertIsNone(result['native_source_graph_submissions'])
                self.assertEqual(result['native_source_stdin_frames'], 288000)
                if scenario == 'balanced-loss-replay':
                    self.assertEqual(result['exact_pattern'], 144000)
                    self.assertGreater(result['pattern_gaps'], 0)


    def test_native_playback_checks_exact_markers_and_all_channels(self):
        for scenario in ('complete', 'balanced-loss-replay', 'rear-corrupt', 'incomplete', 'host-failed'):
            with self.subTest(scenario=scenario):
                data = bytearray(namespace['marker_pcm'](288000, 4))
                if scenario == 'balanced-loss-replay': data[800000:800008] = data[799992:800000]
                if scenario == 'rear-corrupt': data[800006:800008] = b'\x01\x00'
                if scenario == 'incomplete': data = data[:-8]
                def run(command, **options):
                    if command[0] == 'arecord':
                        return Mock(returncode=0, stdout=namespace['marker_pcm'](288000, 2), stderr=b'')
                    self.assertEqual(command[0], 'aplay')
                    self.assertEqual(len(options['input']), 288000 * 8)
                    return Mock(returncode=1 if scenario == 'host-failed' else 0, stderr=b'')
                with patch.dict(namespace, native_playback_identity=Mock(return_value=(1, 11)),
                                bounded_native_capture=Mock(return_value=(bytes(data), b'', {'exit_status':1, 'termination_requested':True}))), \
                     patch.object(namespace['subprocess'], 'run', side_effect=run):
                    result = namespace['run_native_playback_trial'](1, 'dualsense', 'virtualgamepad.12.1.playback')
                self.assertEqual(result['passed'], scenario == 'complete')
                if scenario == 'balanced-loss-replay':
                    self.assertEqual(result['exact_pattern'], 144000)
                    self.assertGreater(result['pattern_gaps'], 0)


    def test_native_playback_cannot_hide_failed_microphone_or_partial_frames(self):
        for scenario in ('complete', 'microphone-silence', 'microphone-extra-byte', 'playback-extra-byte', 'native-source-failed'):
            with self.subTest(scenario=scenario):
                captured = namespace['marker_pcm'](288000, 2)
                if scenario == 'playback-extra-byte': captured += b'\x01'
                def run(command, **options):
                    if command[0] == 'arecord':
                        microphone = bytearray(namespace['marker_pcm'](288000, 1))
                        if scenario == 'microphone-silence': microphone[200000:200004] = bytes(4)
                        if scenario == 'microphone-extra-byte': microphone += b'\x01'
                        return Mock(returncode=0, stdout=bytes(microphone), stderr=b'')
                    return Mock(returncode=1 if scenario == 'native-source-failed' and command[0]=='pw-cat' else 0, stderr=b'')
                with patch.dict(namespace, native_playback_identity=Mock(return_value=(1, 11)),
                                native_microphone_identity=Mock(return_value=(2, 12)),
                                bounded_native_capture=Mock(return_value=(captured, b'', {'exit_status':1, 'termination_requested':True}))), \
                     patch.object(namespace['subprocess'], 'run', side_effect=run):
                    if scenario in ('microphone-extra-byte', 'playback-extra-byte'):
                        with self.assertRaisesRegex(ValueError, 'partial captured PCM frame'):
                            namespace['run_native_playback_trial'](1, 'dualshock4', 'virtualgamepad.12.1.playback', 'virtualgamepad.12.1.microphone')
                        continue
                    result = namespace['run_native_playback_trial'](1, 'dualshock4', 'virtualgamepad.12.1.playback', 'virtualgamepad.12.1.microphone')
                self.assertEqual(result['passed'], scenario == 'complete')
                if scenario == 'microphone-silence': self.assertFalse(result['microphone']['passed'])
    def test_native_playback_identity_rejects_foreign_ambiguous_and_hardware_nodes(self):
        node = 'virtualgamepad.12.1.playback'
        target = dict(id=1, type='PipeWire:Interface:Node', info=dict(props={'node.name':node, 'object.serial':11}))
        with patch.dict(namespace, shared_graph=Mock(return_value=[target])):
            self.assertEqual(namespace['native_playback_identity'](node), (1, 11))
            for invalid in ('default', '../node', 'virtualgamepad.12.1.microphone'):
                with self.assertRaises(ValueError): namespace['native_playback_identity'](invalid)
        for objects in ([], [target, target], [target, dict(info=dict(props={'device.api':'alsa'}))]):
            with patch.dict(namespace, shared_graph=Mock(return_value=objects)):
                with self.assertRaises(ValueError): namespace['native_playback_identity'](node)

    def test_native_analysis_follows_client_completion_and_precedes_decoding(self):
        completed = set()
        analysis = []
        decode = namespace['decode_capture']
        def capture(*args):
            completed.add('capture')
            return namespace['marker_pcm'](288000, 2), b'', {}
        def run(command, **options):
            completed.add(command[0])
            return Mock(returncode=0, stdout=namespace['marker_pcm'](288000, 1), stderr=b'')
        def schedule():
            self.assertEqual(completed, {'capture', 'aplay', 'arecord'})
            analysis.append(True)
            return {'policy': 0}
        def decode_after_analysis(*args):
            self.assertEqual(analysis, [True])
            return decode(*args)
        with patch.dict(namespace, native_playback_identity=Mock(return_value=(1, 11)),
                        bounded_native_capture=capture, marker_analysis_scheduling=schedule,
                        decode_capture=decode_after_analysis), \
             patch.object(namespace['subprocess'], 'run', side_effect=run):
            receipt = namespace['run_native_playback_trial'](1, 'dualshock4', 'virtualgamepad.12.1.playback')
        self.assertTrue(receipt['passed'])
        self.assertEqual(receipt['analysis_scheduling'], {'policy': 0})

    def test_marker_analysis_drops_only_current_thread_realtime_scheduling(self):
        calls = []
        with patch.object(namespace['os'], 'sched_getscheduler', side_effect=[2, 0]), \
             patch.object(namespace['os'], 'sched_setscheduler', side_effect=lambda pid, policy, parameter: calls.append((pid, policy, parameter.sched_priority))), \
             patch.object(namespace['os'], 'sched_getparam', return_value=os.sched_param(0)):
            receipt = namespace['marker_analysis_scheduling']()
        self.assertEqual(calls, [(0, os.SCHED_OTHER, 0)])
        self.assertEqual(receipt, dict(previous_policy=2, policy=0, priority=0,
                                      scope='post-capture marker analysis only'))
        with patch.object(namespace['os'], 'sched_setscheduler', side_effect=PermissionError('synthetic scheduling refusal')):
            with self.assertRaisesRegex(PermissionError, 'synthetic scheduling refusal'):
                namespace['marker_analysis_scheduling']()

    def test_bounded_native_capture_reaps_complete_truncated_and_stalled_children(self):
        import sys
        command = [sys.executable, '-I', '-c', "import os;os.write(1,b'sanitized')"]
        self.assertEqual(namespace['bounded_native_capture'](command, 9)[:2], (b'sanitized', b''))
        with self.assertRaisesRegex(RuntimeError, 'before complete production'):
            namespace['bounded_native_capture'](command, 10)
        stalled = [sys.executable, '-I', '-c', 'import time;time.sleep(30)']
        children = []
        original = namespace['subprocess'].Popen
        def spawn(*args, **kwargs):
            child = original(*args, **kwargs); children.append(child); return child
        with patch.dict(namespace, time=Mock(monotonic=Mock(side_effect=[0, 15]))), \
             patch.object(namespace['subprocess'], 'Popen', side_effect=spawn), \
             self.assertRaisesRegex(RuntimeError, 'deadline exceeded'):
            namespace['bounded_native_capture'](stalled, 9)
        self.assertEqual(len(children), 1)
        self.assertIsNotNone(children[0].returncode)
        self.assertTrue(children[0].stdout.closed)
        for invalid in (0, -1, True, namespace['MAX_CAPTURE_FRAMES'] * 8 + 1):
            with self.assertRaises(ValueError): namespace['bounded_native_capture'](command, invalid)

    def test_expected_record_stop_is_narrow_and_retains_raw_status(self):
        import sys
        command = [sys.executable, '-I', '-c', "import os,signal,time;signal.signal(signal.SIGTERM,lambda *args:exit(1));os.write(1,b'sanitized');time.sleep(30)"]
        data, diagnostic, lifecycle = namespace['bounded_native_capture'](command, 9, 1)
        self.assertEqual((data, diagnostic), (b'sanitized', b''))
        self.assertEqual(lifecycle, dict(exit_status=1, termination_requested=True, expected_record_stop_status=1, captured_bytes=9))
        with self.assertRaisesRegex(RuntimeError, 'native capture child failed'):
            namespace['bounded_native_capture'](command, 9)
        noisy = [sys.executable, '-I', '-c', "import os,signal,time;signal.signal(signal.SIGTERM,lambda *args:exit(1));os.write(2,b'synthetic client error');os.write(1,b'sanitized');time.sleep(30)"]
        with self.assertRaisesRegex(RuntimeError, 'synthetic client error'):
            namespace['bounded_native_capture'](noisy, 9, 1)

    def test_complete_stdout_cannot_hide_nonzero_native_client_exit(self):
        read, write = os.pipe()
        os.write(write, b'sanitized'); os.close(write)
        child = Mock(stdout=os.fdopen(read, 'rb'), returncode=1)
        child.poll.return_value = 1
        with patch.object(namespace['subprocess'], 'Popen', return_value=child), \
             self.assertRaisesRegex(RuntimeError, 'native capture child failed'):
            namespace['bounded_native_capture'](['sanitized-command'], 9, 1)
        child.terminate.assert_not_called()
        child.wait.assert_called_once_with(timeout=3)
        self.assertTrue(child.stdout.closed)
    def test_native_microphone_admission_rejects_foreign_or_ambiguous_nodes(self):
        node = 'virtualgamepad.12.1.microphone'
        target = dict(id=1, type='PipeWire:Interface:Node', info=dict(props={
            'node.name':node, 'object.serial':11}))
        with patch.dict(namespace, shared_graph=Mock(return_value=[target])):
            self.assertEqual(namespace['native_microphone_identity'](node), (1,11))
            for invalid in ('default', '../node', 'virtualgamepad.12.1.playback'):
                with self.assertRaises(ValueError):namespace['native_microphone_identity'](invalid)
        for objects in ([], [target,target], [target,dict(info=dict(props={'device.api':'alsa'}))]):
            with patch.dict(namespace, shared_graph=Mock(return_value=objects)):
                with self.assertRaises(ValueError):namespace['native_microphone_identity'](node)

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
