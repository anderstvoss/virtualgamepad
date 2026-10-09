import importlib.util
import hashlib
import fcntl
import tempfile
from pathlib import Path
from unittest.mock import Mock, patch
import socket
import array
import os
import struct
import threading
import unittest

spec = importlib.util.spec_from_file_location('broker_live',Path(__file__).parents[1]/'validate-broker-audio-live.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class BrokerLiveTests(unittest.TestCase):
    def test_probe_image_capture_rejects_corruption_non_elf_and_non_regular(self):
        for data, expected in ((b'not ELF', hashlib.sha256(b'not ELF').digest()),
                               (b'\x7fELFsynthetic', bytes(32))):
            with tempfile.TemporaryFile() as source:
                source.write(data); source.flush()
                with self.assertRaises(ValueError): module.sealed_probe_image(source.fileno(), expected)
        read, write = os.pipe()
        try:
            with self.assertRaises(ValueError): module.sealed_probe_image(read, bytes(32))
        finally: os.close(read); os.close(write)

    def test_probe_image_is_sealed_and_survives_original_change(self):
        data = b'\x7fELFsynthetic receipt only; never executed'
        with tempfile.TemporaryFile() as source:
            source.write(data); source.flush()
            frozen = module.sealed_probe_image(source.fileno(), hashlib.sha256(data).digest())
            try:
                source.seek(0); source.write(b'changed'); source.flush()
                self.assertEqual(os.pread(frozen, len(data), 0), data)
                for operation in (lambda: os.write(frozen, b'x'), lambda: os.ftruncate(frozen, 1)):
                    with self.assertRaises(OSError): operation()
                self.assertEqual(fcntl.fcntl(frozen, fcntl.F_GET_SEALS),
                                 fcntl.F_SEAL_WRITE | fcntl.F_SEAL_GROW | fcntl.F_SEAL_SHRINK | fcntl.F_SEAL_SEAL)
            finally: os.close(frozen)

    def test_probe_admission_root_unknown_request_and_zero_tests_cannot_pass(self):
        with patch.object(module.os, 'geteuid', return_value=0):
            with self.assertRaisesRegex(ValueError, 'without root'): module.public_factory_probe(Mock())
        peer = Mock(); peer.recv.return_value = b'bad'
        with patch.object(module.os, 'geteuid', return_value=1000):
            with self.assertRaisesRegex(ValueError, 'unknown'): module.public_factory_probe(peer)
            peer.recv.return_value = b''
            self.assertEqual(module.public_factory_probe(peer), dict(status='not-requested'))
        with tempfile.TemporaryFile() as source:
            source.write(b'\x7fELFfake'); source.flush()
            descriptor = os.dup(source.fileno())
            peer.recv.return_value = b'R'
            peer.recvmsg.return_value = (bytes(32), [(socket.SOL_SOCKET, socket.SCM_RIGHTS,
                                                     array.array('i', [descriptor]).tobytes())], 0, None)
            frozen = os.dup(source.fileno())
            with patch.object(module.os, 'geteuid', return_value=1000), \
                 patch.object(module, 'sealed_probe_image', return_value=frozen), \
                 patch.object(module.subprocess, 'run', return_value=Mock(returncode=0)), \
                 patch('builtins.print'):
                with self.assertRaisesRegex(ValueError, 'selected zero'): module.public_factory_probe(peer)
            peer.sendall.assert_called_with(b'E')
            for closed in (descriptor, frozen):
                with self.assertRaises(OSError): os.fstat(closed)

    def test_probe_timeout_and_malformed_rights_close_every_owned_descriptor(self):
        with tempfile.TemporaryFile() as source:
            source.write(b'\x7fELFfake'); source.flush()
            for timeout in (False, True):
                descriptor = os.dup(source.fileno()); frozen = os.dup(source.fileno())
                peer = Mock(); peer.recv.return_value = b'R'
                ancillary = [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array('i', [descriptor]).tobytes())]
                if not timeout: ancillary.insert(0, (123, 456, b'bad'))
                peer.recvmsg.return_value = (bytes(32), ancillary, 0, None)
                with patch.object(module.os, 'geteuid', return_value=1000), \
                     patch.object(module, 'sealed_probe_image', return_value=frozen) as capture, \
                     patch.object(module.subprocess, 'run', side_effect=module.subprocess.TimeoutExpired('probe', 45)):
                    with self.assertRaises((ValueError, module.subprocess.TimeoutExpired)):
                        module.public_factory_probe(peer)
                with self.assertRaises(OSError): os.fstat(descriptor)
                if timeout:
                    with self.assertRaises(OSError): os.fstat(frozen)
                else:
                    capture.assert_not_called(); os.close(frozen)

    def test_final_accounting_uses_quiescent_credit_not_last_producer_poll(self):
        counters = dict(capture_frames=240096, abandoned_capture_frames=0,
                        microphone_silence_frames=11760)
        control = Mock()
        response = (1, 7, struct.pack('<QQQ', 7, 240096, 11760))
        with patch.object(module, 'message') as send, \
             patch.object(module, 'reply', return_value=response), \
             patch.object(module, 'microphone_credit', return_value=228336), \
             patch.object(module, 'worker_diagnostics', return_value=counters):
            result, observed = module.final_microphone_accounting(control, 7, 228720)
        self.assertEqual(result, dict(host_frames=240096, consumed_frames=228336,
                                     silence_frames=11760, submitted_frames=228720,
                                     unconsumed_frames=384, completed_frames=240096,
                                     abandoned_frames=0, quiescent=True))
        self.assertEqual(observed, counters)
        self.assertEqual(send.call_count, 2)
        send.assert_called_with(control, 1, 7, struct.pack('<Q', 7))

    def test_final_accounting_retries_motion_but_rejects_unbounded_drain(self):
        for counts, succeeds in [([48, 96, 96], True), ([48, 96, 144], False)]:
            with self.subTest(counts=counts):
                replies = [(1, 7, struct.pack('<QQQ', 7, frames, 0)) for frames in counts]
                counters = [dict(capture_frames=frames, abandoned_capture_frames=0,
                                 microphone_silence_frames=0) for frames in counts]
                with patch.object(module, 'message'), \
                     patch.object(module, 'reply', side_effect=replies) as receive, \
                     patch.object(module, 'microphone_credit', side_effect=counts), \
                     patch.object(module, 'worker_diagnostics', side_effect=counters):
                    if succeeds:
                        result, _ = module.final_microphone_accounting(Mock(), 7, 144)
                        self.assertEqual(result['consumed_frames'], 96)
                    else:
                        with self.assertRaisesRegex(ValueError, 'quiescent'):
                            module.final_microphone_accounting(Mock(), 7, 144)
                    self.assertEqual(receive.call_count, 3)

    def test_final_accounting_rejects_inconsistent_and_foreign_snapshots(self):
        baseline = dict(capture_frames=96, abandoned_capture_frames=0,
                        microphone_silence_frames=0)
        cases = [(7, 96, 0, 97, 144, baseline),  # consumed beyond host
                 (7, 96, 0, 96, 95, baseline),   # consumed beyond production
                 (7, 96, 1, 95, 144, baseline),  # disagreeing silence samples
                 (7, 96, 0, 96, 144, {**baseline, 'capture_frames': 48}),
                 (8, 96, 0, 96, 144, baseline)]
        for generation, host, silence, consumed, submitted, counters in cases:
            with self.subTest(case=(generation, host, silence, consumed, submitted, counters)), \
                 patch.object(module, 'message'), \
                 patch.object(module, 'reply', return_value=(1, 7, struct.pack('<QQQ', generation, host, silence))), \
                 patch.object(module, 'microphone_credit', return_value=consumed), \
                 patch.object(module, 'worker_diagnostics', return_value=counters):
                with self.assertRaises(ValueError):
                    module.final_microphone_accounting(Mock(), 7, submitted)

    def test_final_accounting_retains_abandoned_capture_separately(self):
        counters = dict(capture_frames=48, abandoned_capture_frames=48,
                        microphone_silence_frames=0)
        with patch.object(module, 'message'), \
             patch.object(module, 'reply', return_value=(1, 7, struct.pack('<QQQ', 7, 96, 0))), \
             patch.object(module, 'microphone_credit', return_value=96), \
             patch.object(module, 'worker_diagnostics', return_value=counters):
            result, _ = module.final_microphone_accounting(Mock(), 7, 144)
        self.assertEqual(result['abandoned_frames'], 48)
        self.assertEqual(result['completed_frames'], 48)

    def test_microphone_credit_checks_exact_reply_and_generation(self):
        for version, operation, data in [(1, 5, struct.pack('<QQ', 7, 96)),
                                         (1, 5, struct.pack('<QQ', 8, 96)),
                                         (2, 5, struct.pack('<QQ', 7, 96)),
                                         (1, 7, struct.pack('<QQ', 7, 96)),
                                         (1, 5, bytes(8))]:
            with self.subTest(response=(version, operation, data)), \
                 patch.object(module, 'message') as send, \
                 patch.object(module, 'reply', return_value=(version, operation, data)):
                peer = Mock()
                if (version, operation, data) == (1, 5, struct.pack('<QQ', 7, 96)):
                    self.assertEqual(module.microphone_credit(peer, 7), 96)
                else:
                    with self.assertRaises(ValueError): module.microphone_credit(peer, 7)
                send.assert_called_once_with(peer, 1, 5, struct.pack('<Q', 7))

    def test_invalid_ancillary_metadata_closes_all_delivered_fds(self):
        for malformed in ('unexpected', 'truncated', 'marker', 'count', 'partial'):
            with self.subTest(malformed=malformed):
                originals = [socket.socketpair() for _ in range(3)]
                received = [os.dup(pair[0].fileno()) for pair in originals]
                peer = Mock()
                ancillary = [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array('i', received).tobytes())]
                flags = 0
                marker = b'\xa2'
                if malformed == 'unexpected': ancillary.insert(0, (999, 999, b'fake'))
                if malformed == 'truncated': flags = socket.MSG_CTRUNC
                if malformed == 'marker': marker = b'bad'
                if malformed == 'count': ancillary[0] = (socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array('i', received[:2]).tobytes()); os.close(received.pop())
                if malformed == 'partial': ancillary[0] = (socket.SOL_SOCKET, socket.SCM_RIGHTS, ancillary[0][2] + b'x')
                peer.recvmsg.return_value = (marker, ancillary, flags, None)
                try:
                    with self.assertRaisesRegex(ValueError, 'handoff'): module.channel_handoff(peer)
                    for descriptor in received:
                        with self.assertRaises(OSError): os.fstat(descriptor)
                finally:
                    for pair in originals:
                        for channel in pair: channel.close()

    def test_valid_handoff_preserves_channel_order_and_owned_cleanup(self):
        originals = [socket.socketpair() for _ in range(3)]
        received = [os.dup(pair[0].fileno()) for pair in originals]
        peer = Mock()
        peer.recvmsg.return_value = (b'\xa2', [(socket.SOL_SOCKET, socket.SCM_RIGHTS,
                                             array.array('i', received).tobytes())], 0, None)
        channels = []
        try:
            channels = module.channel_handoff(peer)
            self.assertEqual([channel.fileno() for channel in channels], received)
            for index, channel in enumerate(channels):
                originals[index][1].sendall(bytes([index]))
                self.assertEqual(channel.recv(1), bytes([index]))
        finally:
            for channel in channels: channel.close()
            for pair in originals:
                for channel in pair: channel.close()

    def test_open_failure_always_closes_the_broker_socket(self):
        for response in [(2, 0x80, b'partial'), (2, 0x80, bytes(12) + b'4-1'), (2, 0xff, b'rejected')]:
            peer = Mock()
            peer.getsockopt.return_value = struct.pack('3i', 42, 0, 0)
            with patch.object(module.socket, 'socket', return_value=peer), \
                 patch.object(module, 'reply', return_value=response), patch.object(module, 'message'):
                with self.assertRaises(ValueError): module.opened('dualsense')
            peer.close.assert_called_once()
            peer.recvmsg.assert_not_called()

    def test_worker_counters_must_prove_host_transfer_and_no_reported_loss(self):
        counters = dict(playback_frames=240000, capture_frames=240000,
                        lost_outputs=0, microphone_queue_dropped_frames=0)
        self.assertTrue(module.functional_worker_evidence(counters, 240000))
        for key, bad in [('playback_frames', 239999), ('capture_frames', 0),
                         ('lost_outputs', 1), ('microphone_queue_dropped_frames', 1)]:
            with self.subTest(key=key):
                self.assertFalse(module.functional_worker_evidence({**counters, key: bad}, 240000))

    def test_close_keeps_required_channels_until_ack_and_closes_on_error(self):
        for fail in [False, True]:
            broker = Mock()
            channels = [Mock() for _ in range(3)]
            def acknowledge(_):
                for channel in channels: channel.close.assert_not_called()
                if fail: raise EOFError('synthetic broker loss')
                return (2, 0x80, struct.pack('<Q', 7))
            with patch.object(module, 'message'), patch.object(module, 'reply', side_effect=acknowledge):
                errors = module.close_broker_session(broker, channels, 7)
            self.assertEqual(errors, ['synthetic broker loss'] if fail else [])
            for channel in channels: channel.close.assert_called_once()
            broker.close.assert_called_once()

    def test_test_only_microphone_fill_stays_below_one_pcm_queue(self):
        self.assertEqual(module.microphone_fill_frames(8),384)
        self.assertEqual(module.microphone_fill_frames(12),576)
        self.assertEqual(module.microphone_fill_frames(16),768)
        for invalid in (0,17,-1):
            with self.assertRaises(ValueError): module.microphone_fill_frames(invalid)

    def test_worker_timing_reply_is_generation_checked_and_distinct_from_usb_lateness(self):
        def exchange(generation_in_timing):
            server,client = socket.socketpair()
            def serve():
                try:
                    for tag,body in ((3,struct.pack('<Q9Q',7,*range(9))),
                                     (6,struct.pack('<QQ',generation_in_timing,1234))):
                        self.assertEqual(module.reply(server),(1,tag,struct.pack('<Q',7)))
                        module.message(server,1,tag,body)
                finally:
                    server.close()
            task = threading.Thread(target=serve)
            task.start()
            try: return module.worker_diagnostics(client,7)
            finally:
                client.close()
                task.join(2)
                self.assertFalse(task.is_alive())
        result = exchange(7)
        self.assertEqual(result['maximum_audio_lateness_us'],7)
        self.assertEqual(result['microphone_queue_dropped_frames'],8)
        self.assertEqual(result['maximum_pcm_pump_lateness_us'],1234)
        with self.assertRaisesRegex(ValueError,'PCM pump timing'):
            exchange(8)

    def test_refill_diagnostics_are_bounded_and_preserve_frame_correlation(self):
        delays = module.RefillDelays()
        self.assertEqual(delays.summary(),[])
        started = 0
        for index in range(100):
            started += index*1000
            delays.record(started,started+2000,index*48)
        result = delays.summary()
        self.assertEqual(len(result),8)
        self.assertEqual(result[0],dict(interval_us=99,credit_roundtrip_us=2,consumed_frame=99*48))
        self.assertEqual(result[-1]['interval_us'],92)
        # Analysis after capture may hold the interpreter; no consumption means
        # those delays must not displace observations from active streaming.
        delays.record(started+1_000_000_000,started+1_000_000_001,99*48)
        self.assertEqual(delays.summary(),result)
        with self.assertRaises(ValueError): delays.record(0,1,0)
        with self.assertRaises(ValueError): module.RefillDelays().record(2,1,0)

    def test_owned_card_selector_rejects_physical_same_card_and_ambiguity(self):
        def device(path):
            return {'id':7,'type':'PipeWire:Interface:Device','info':{'props':{'api.alsa.card':2,'device.bus-path':path}}}
        physical = device('pci-0000:00:02.0-usb-0:6:1.0')
        owned = device('platform-vhci_hcd.0-usb-0:1:1.0')
        self.assertIsNone(module.owned_pipewire_device([physical],2))
        self.assertIs(module.owned_pipewire_device([physical,owned],2),owned)
        self.assertIsNone(module.owned_pipewire_device([owned],1))
        with self.assertRaises(ValueError):
            module.owned_pipewire_device([owned,owned],2)

    def test_bounded_protocol_rejects_oversized_and_partial_reply(self):
        for data in [struct.pack('<I',260),struct.pack('<I',3)+b'\x02']:
            a,b = socket.socketpair()
            with a,b:
                a.sendall(data); a.shutdown(socket.SHUT_WR)
                with self.assertRaises((ValueError,EOFError)):
                    module.reply(b)
