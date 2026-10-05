import importlib.util
from pathlib import Path
import struct
import unittest
from unittest.mock import Mock, patch

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

    def test_invalid_expected_instance_rejects_before_creation(self):
        with patch.object(lab.audio, 'opened') as opened:
            with self.assertRaises(ValueError): lab.Session('dualsense', '../escape', 0)
        opened.assert_not_called()
