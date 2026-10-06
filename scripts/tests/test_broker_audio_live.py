import importlib.util
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
