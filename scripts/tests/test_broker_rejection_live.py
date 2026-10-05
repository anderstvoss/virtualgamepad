import array
import importlib.util
import os
import io
from pathlib import Path
import socket
import struct
import unittest
from unittest.mock import Mock, patch

spec = importlib.util.spec_from_file_location('rejection', Path(__file__).parents[1] / 'validate-broker-rejection-live.py')
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


class Peer:
    def __init__(self, chunks):
        self.chunks = iter(chunks)

    def recvmsg(self, *args):
        return next(self.chunks)


class RejectionReceipt(unittest.TestCase):
    def test_exact_reply_handles_fragmentation_and_checks_acknowledgement(self):
        reply = probe.frame(0x81, b'fake error')
        chunks = [(reply[:2], [], 0, None), (reply[2:4], [], 0, None), (reply[4:], [], 0, None)]
        probe.expect_error(Peer(chunks), b'fake error')
        reply = probe.frame(0x80, b'fake error')
        with self.assertRaisesRegex(RuntimeError, 'incorrect rejection'):
            probe.expect_error(Peer([(reply[:4], [], 0, None), (reply[4:], [], 0, None)]), b'fake error')

    def test_oversized_reply_is_rejected_before_reading_body(self):
        for length in (0, 2, 260, 2**32 - 1):
            with self.assertRaisesRegex(RuntimeError, 'unbounded'):
                probe.expect_error(Peer([(struct.pack('<I', length), [], 0, None)]), b'fake')

    def test_unexpected_received_descriptors_are_closed_even_on_truncation(self):
        reader, writer = os.pipe()
        try:
            descriptor = os.dup(writer)
            ancillary = [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array('i', [descriptor]).tobytes())]
            with self.assertRaisesRegex(RuntimeError, 'unexpected descriptors'):
                probe.exact(Peer([(b'x', ancillary, socket.MSG_CTRUNC, None)]), 1)
            with self.assertRaises(OSError):
                os.fstat(descriptor)
        finally:
            os.close(reader)
            os.close(writer)

    def test_premature_eof_cannot_be_counted_as_a_valid_reply(self):
        with self.assertRaisesRegex(RuntimeError, 'premature EOF'):
            probe.exact(Peer([(b'', [], 0, None)]), 4)

    def test_unauthorized_admission_closes_before_any_request_is_sent(self):
        peer = Mock()
        peer.__enter__ = Mock(return_value=peer)
        peer.__exit__ = Mock(return_value=False)
        peer.recv.return_value = b''
        status = 'CapEff: 0\nCapPrm: 0\nCapAmb: 0\nCapBnd: 0\nNoNewPrivs: 1\n'
        with patch.object(probe.os, 'geteuid', return_value=1003), patch.object(probe.os, 'getegid', return_value=1003), patch.object(probe.os, 'getgroups', return_value=[]), patch.object(probe, 'connect', return_value=peer), patch('builtins.open', return_value=io.StringIO(status)):
            self.assertEqual(probe.probe(True)['checks'], 1)
        peer.sendall.assert_not_called()
        peer.recv.assert_called_once_with(1)

    def test_unauthorized_receipt_rejects_pending_or_successful_connection(self):
        peer = Mock()
        peer.recv.return_value = b'x'
        with self.assertRaisesRegex(RuntimeError, 'not terminated'):
            probe.closed(peer)
        peer.recv.side_effect = TimeoutError('still open')
        with self.assertRaises(TimeoutError):
            probe.closed(peer)
