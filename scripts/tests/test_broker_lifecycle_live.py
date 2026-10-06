import importlib.util
from pathlib import Path
import struct
import unittest
from unittest.mock import MagicMock, Mock, patch

spec = importlib.util.spec_from_file_location('lifecycle', Path(__file__).parents[1] / 'validate-broker-lifecycle-live.py')
lab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lab)


class LifecycleProbe(unittest.TestCase):
    def session(self):
        session = lab.Session.__new__(lab.Session)
        session.closed = False
        session.generation = 7
        session.port = 0
        session.ownership = (17, '4-1')
        session.broker = Mock()
        session.broker.recv.return_value = b''
        session.channels = [Mock() for _ in range(3)]
        return session

    def test_normal_close_keeps_channels_alive_until_generation_ack_and_is_idempotent(self):
        session = self.session()
        def acknowledge(_):
            for channel in session.channels: channel.close.assert_not_called()
            return (2, 0x80, struct.pack('<Q', 7))
        with patch.object(lab.audio, 'reply', side_effect=acknowledge), \
             patch.object(lab.audio, 'message') as send, patch.object(lab, 'wait_detached') as detached:
            session.close()
            session.close()
        send.assert_called_once_with(session.broker, 2, 2, struct.pack('<Q', 7))
        detached.assert_called_once_with(0, (17, '4-1'))
        for channel in session.channels: channel.close.assert_called_once()
        session.broker.close.assert_called_once()

    def test_bad_ack_and_attachment_cleanup_error_are_both_retained(self):
        session = self.session()
        with patch.object(lab.audio, 'reply', return_value=(2, 0x80, struct.pack('<Q', 8))), \
             patch.object(lab.audio, 'message'), \
             patch.object(lab, 'wait_detached', side_effect=TimeoutError('synthetic timeout')):
            with self.assertRaises(RuntimeError) as error: session.close()
        self.assertIn('acknowledgement', str(error.exception))
        self.assertIn('synthetic timeout', str(error.exception))
        for channel in session.channels: channel.close.assert_called_once()
        session.broker.close.assert_called_once()

    def test_abandonment_does_not_send_an_application_close(self):
        session = self.session()
        with patch.object(lab.audio, 'message') as send, patch.object(lab, 'wait_detached'):
            session.close(abandon=True)
        send.assert_not_called()
        self.assertTrue(session.closed)

    def test_worker_death_requires_terminal_error_and_eof_without_an_application_close(self):
        for reply, accepted in [((2, 0x81, b'audio worker exited: signal: 9 (SIGKILL)'), True),
                                ((2, 0x80, b'ack'), False), ((2, 0x81, b'other error'), False)]:
            session = self.session(); session.device = 17; session.generation = 7
            session.close = Mock()
            for channel in session.channels: channel.recv.return_value = b''
            supervisor = MagicMock()
            supervisor.__enter__.return_value = supervisor
            supervisor.getsockopt.return_value = struct.pack('3i', 42, 0, 0)
            supervisor.recv.return_value = b'K'
            with patch.object(lab, 'Session', return_value=session), \
                 patch.object(lab, 'fd_count', return_value=4), \
                 patch.object(lab.socket, 'socket', return_value=supervisor), \
                 patch.object(lab.audio, 'reply', return_value=reply):
                if accepted:
                    result = lab.worker_death('dualsense','lab',0,Path('/synthetic/socket'))
                    self.assertTrue(result['cleanup'])
                else:
                    with self.assertRaises(RuntimeError): lab.worker_death('dualsense','lab',0,Path('/synthetic/socket'))
            session.close.assert_called_once_with(abandon=True)

    def test_invalid_expected_instance_rejects_before_creation(self):
        with patch.object(lab.audio, 'opened') as opened:
            with self.assertRaises(ValueError): lab.Session('dualsense', '../escape', 0)
        opened.assert_not_called()


class SiblingAdmission(unittest.TestCase):
    def session(self, generation):
        session = Mock()
        session.generation = generation
        session.isolation = {'serial': 'synthetic-' + str(generation)}
        session.channels = [Mock()]
        return session

    def test_removal_preserves_siblings_and_recovers_capacity_with_fresh_identity(self):
        sessions = [self.session(i) for i in range(1, 6)]
        with patch.object(lab, 'Session', side_effect=sessions) as create, \
             patch.object(lab, 'fd_count', return_value=8), \
             patch.object(lab, 'capacity_rejection') as reject, \
             patch.object(lab.audio, 'worker_diagnostics', return_value={'alive': True}) as diagnostics:
            receipts = lab.siblings_and_admission('lab', [0, 1, 2, 3])
        reject.assert_called_once()
        self.assertEqual(create.call_args_list[-1].args, ('dualshock4', 'lab', 1))
        self.assertEqual([r['generation'] for r in receipts], [1, 5, 3, 4])
        self.assertEqual(diagnostics.call_count, 7)
        for session in sessions: self.assertGreaterEqual(session.close.call_count, 1)

    def test_partial_construction_preserves_initiating_and_cleanup_errors(self):
        first = self.session(1)
        first.close.side_effect = RuntimeError('synthetic cleanup')
        with patch.object(lab, 'Session', side_effect=[first, RuntimeError('synthetic construction')]), \
             patch.object(lab, 'fd_count', return_value=8), \
             patch.object(lab, 'capacity_rejection') as reject:
            with self.assertRaises(RuntimeError) as error: lab.siblings_and_admission('lab', [0,1,2,3])
        reject.assert_not_called()
        self.assertIn('synthetic construction', str(error.exception))
        self.assertIn('synthetic cleanup', str(error.exception))
        first.close.assert_called_once()


class AttachmentFailureEvidence(unittest.TestCase):
    def test_pending_reply_is_bounded_and_not_consumed(self):
        import socket
        left,right=socket.socketpair()
        try:
            right.sendall(b'synthetic terminal reply')
            with patch.object(lab.Path,'read_text',return_value='header\nhs 0000 004 000 00000000 000000 0-0\n'):
                result=lab.attachment_snapshot(0,left)
            self.assertEqual(bytes.fromhex(result['broker_reply_hex']),b'synthetic terminal reply')
            self.assertFalse(result['broker_eof'])
            self.assertEqual(left.recv(100),b'synthetic terminal reply')
            self.assertEqual(result['vhci_rows'],['hs 0000 004 000 00000000 000000 0-0'])
        finally:left.close();right.close()

    def test_eof_and_unreadable_peer_are_distinguished(self):
        import socket
        left,right=socket.socketpair()
        try:
            with patch.object(lab.Path,'read_text',side_effect=OSError('synthetic unavailable')):
                self.assertFalse(lab.attachment_snapshot(0,left)['broker_readable'])
                right.close()
                result=lab.attachment_snapshot(0,left)
                self.assertTrue(result['broker_eof'])
                self.assertIn('vhci_unavailable',result)
        finally:left.close();right.close()
