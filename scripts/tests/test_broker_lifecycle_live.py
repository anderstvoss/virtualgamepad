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

    def test_broker_confirmation_waits_for_paced_restoration_and_keeps_eof_checks(self):
        session = self.session(); session.device = 17; session.close = Mock()
        for channel in session.channels: channel.recv.return_value = b''
        supervisor = MagicMock(); supervisor.__enter__.return_value = supervisor
        supervisor.getsockopt.return_value = struct.pack('3i', 42, 0, 0)
        timeout = [None]
        supervisor.settimeout.side_effect = lambda value: timeout.__setitem__(0, value)
        def restored(_):
            # Five activation waits alone exceed the former ten-second limit.
            if timeout[0] <= 5 * 2.1: raise TimeoutError('restoration still in progress')
            return b'B'
        supervisor.recv.side_effect = restored
        with patch.object(lab, 'Session', return_value=session), \
             patch.object(lab, 'fd_count', return_value=4), \
             patch.object(lab.socket, 'socket', return_value=supervisor):
            result = lab.worker_death('dualsense', 'lab', 0, Path('/synthetic/socket'), broker_death=True)
        self.assertTrue(result['cleanup'])
        self.assertEqual(supervisor.settimeout.call_args_list[0].args, (10,))
        self.assertLessEqual(timeout[0], 60)
        session.broker.settimeout.assert_called_once_with(5)
        session.broker.recv.assert_called_once_with(1)
        for channel in session.channels:
            channel.settimeout.assert_called_once_with(5)
            channel.recv.assert_called_once_with(1)
        session.close.assert_called_once_with(abandon=True)

    def test_fault_confirmation_failure_still_closes_session(self):
        for broker_death, response in [(False, b'B'), (True, b'K'), (True, TimeoutError('bounded recovery timeout'))]:
            with self.subTest(broker_death=broker_death, response=response):
                session = self.session(); session.device = 17; session.close = Mock()
                supervisor = MagicMock(); supervisor.__enter__.return_value = supervisor
                supervisor.getsockopt.return_value = struct.pack('3i', 42, 0, 0)
                if isinstance(response, Exception): supervisor.recv.side_effect = response
                else: supervisor.recv.return_value = response
                with patch.object(lab, 'Session', return_value=session), \
                     patch.object(lab, 'fd_count', return_value=4), \
                     patch.object(lab.socket, 'socket', return_value=supervisor):
                    with self.assertRaises((RuntimeError, TimeoutError)):
                        lab.worker_death('dualsense', 'lab', 0, Path('/synthetic/socket'), broker_death=broker_death)
                self.assertEqual(supervisor.settimeout.call_args_list[-1].args, (45 if broker_death else 10,))
                session.close.assert_called_once_with(abandon=True)

    def test_invalid_expected_instance_rejects_before_creation(self):
        with patch.object(lab.audio, 'opened') as opened:
            with self.assertRaises(ValueError): lab.Session('dualsense', '../escape', 0)
        opened.assert_not_called()

    def test_fault_confirmation_and_cleanup_failures_are_both_preserved(self):
        session = self.session(); session.device = 17
        session.close = Mock(side_effect=RuntimeError('synthetic cleanup failure'))
        supervisor = MagicMock(); supervisor.__enter__.return_value = supervisor
        supervisor.getsockopt.return_value = struct.pack('3i', 42, 0, 0)
        supervisor.recv.return_value = b'wrong confirmation'
        with patch.object(lab, 'Session', return_value=session), \
             patch.object(lab, 'fd_count', return_value=4), \
             patch.object(lab.socket, 'socket', return_value=supervisor):
            with self.assertRaises(RuntimeError) as error:
                lab.worker_death('dualsense', 'lab', 0, Path('/synthetic/socket'))
        self.assertIn('did not confirm injection', str(error.exception))
        self.assertIn('synthetic cleanup failure', str(error.exception))
        self.assertIsInstance(error.exception.__cause__, RuntimeError)
        session.close.assert_called_once_with(abandon=True)


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
        self.assertEqual(create.call_args_list[-1].args, ('dualshock4', 'lab', [0, 1, 2, 3]))
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

    def test_forced_worker_failure_keeps_siblings_and_recovers_fourth_slot(self):
        sessions = [self.session(i) for i in range(1,5)]
        for index, session in enumerate(sessions):
            session.port=index; session.ownership=(index+17, 'synthetic-bus')
        failed = dict(generation=5, cleanup=True)
        with patch.object(lab, 'Session', side_effect=sessions) as create, \
             patch.object(lab, 'fd_count', return_value=8), \
             patch.object(lab, 'worker_death', return_value=failed) as kill, \
             patch.object(lab, 'capacity_rejection') as reject, \
             patch.object(lab.Path, 'read_text', return_value='synthetic inventory'), \
             patch.object(lab, 'resolve_session_port', side_effect=[0,1,2]), \
             patch.object(lab.audio, 'worker_diagnostics', return_value={'alive':True}) as diagnostics:
            result=lab.forced_sibling_worker_death('xbox360','lab',[0,1,2,3],Path('/synthetic/socket'))
        kill.assert_called_once_with('xbox360','lab',[0,1,2,3],Path('/synthetic/socket'))
        self.assertEqual([r['generation'] for r in result['survivors']],[1,2,3])
        self.assertEqual(result['replacement_generation'],4)
        self.assertEqual(create.call_args_list[-1].args,('xbox360','lab',[0,1,2,3]))
        reject.assert_called_once(); self.assertEqual(diagnostics.call_count,7)
        for session in sessions: self.assertEqual(session.close.call_count,2)

    def test_forced_failure_retains_injection_and_all_owned_cleanup_errors(self):
        sessions=[self.session(i) for i in range(1,4)]
        sessions[0].close.side_effect=RuntimeError('synthetic sibling cleanup')
        with patch.object(lab, 'Session', side_effect=sessions), \
             patch.object(lab, 'fd_count', return_value=8), \
             patch.object(lab, 'worker_death', side_effect=TimeoutError('synthetic fault timeout')), \
             patch.object(lab, 'capacity_rejection') as reject:
            with self.assertRaises(RuntimeError) as error:
                lab.forced_sibling_worker_death('dualsense','lab',[0,1,2,3],Path('/synthetic/socket'))
        self.assertIn('synthetic fault timeout',str(error.exception))
        self.assertIn('synthetic sibling cleanup',str(error.exception))
        reject.assert_not_called()
        for session in sessions: session.close.assert_called()

    def test_changed_sibling_identity_rejects_before_replacement_and_cleans_owned_sessions(self):
        sessions=[self.session(i) for i in range(1,4)]
        for index, session in enumerate(sessions):session.port=index
        with patch.object(lab, 'Session', side_effect=sessions) as create, \
             patch.object(lab, 'fd_count', return_value=8), \
             patch.object(lab, 'worker_death', return_value=dict(generation=5)), \
             patch.object(lab.Path, 'read_text', return_value='synthetic inventory'), \
             patch.object(lab, 'resolve_session_port', return_value=3), \
             patch.object(lab, 'capacity_rejection') as reject:
            with self.assertRaisesRegex(RuntimeError,'attachment identity changed'):
                lab.forced_sibling_worker_death('dualsense','lab',[0,1,2,3],Path('/synthetic/socket'))
        self.assertEqual(create.call_count,3);reject.assert_not_called()
        for session in sessions:self.assertEqual(session.close.call_count,2)

    def test_forced_sibling_invalid_allowlist_creates_nothing(self):
        for ports in [(0,), (0,0,1,2), (0,1,2,True)]:
            with patch.object(lab,'Session') as create:
                with self.assertRaises(ValueError):
                    lab.forced_sibling_worker_death('dualsense','lab',ports,Path('/synthetic/socket'))
            create.assert_not_called()

    def test_replacement_cannot_reuse_failed_identity_or_hide_descriptor_leak(self):
        for replacement_generation, counts, message in [(5,[8,8],'reused an identity'),
                                                        (4,[8,9],'descriptors did not return')]:
            sessions=[self.session(i) for i in [1,2,3,replacement_generation]]
            for index, session in enumerate(sessions):session.port=index
            with patch.object(lab,'Session',side_effect=sessions), \
                 patch.object(lab,'fd_count',side_effect=counts), \
                 patch.object(lab,'worker_death',return_value=dict(generation=5)), \
                 patch.object(lab.Path,'read_text',return_value='synthetic inventory'), \
                 patch.object(lab,'resolve_session_port',side_effect=[0,1,2]), \
                 patch.object(lab,'capacity_rejection'), \
                 patch.object(lab.audio,'worker_diagnostics',return_value={'alive':True}):
                with self.assertRaisesRegex(RuntimeError,message):
                    lab.forced_sibling_worker_death('dualsense','lab',[0,1,2,3],Path('/synthetic/socket'))
            for session in sessions:self.assertEqual(session.close.call_count,2)


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


class SessionPortIdentity(unittest.TestCase):
    header='hub port sta spd dev sockfd local_busid\n'
    free='hs 0000 004 000 00000000 000000 0-0\n'
    owned='hs 0001 006 003 00000011 000009 4-2\n'

    def test_owned_second_port_is_resolved_instead_of_assuming_first(self):
        self.assertEqual(lab.resolve_session_port(self.header+self.free+self.owned,(0,1),(17,'4-2')),1)
        foreign='hs 0000 006 003 00000022 000010 4-1\n'
        self.assertEqual(lab.resolve_session_port(self.header+foreign+self.owned,(0,1),(17,'4-2')),1)

    def test_wrong_identity_unauthorized_port_and_ambiguous_inventory_reject(self):
        for status,ports,identity in [
            (self.header+self.owned,(0,),(17,'4-2')),
            (self.header+self.owned,(0,1),(18,'4-2')),
            (self.header+self.owned,(0,1),(17,'4-1')),
            (self.header+self.owned+self.owned,(0,1),(17,'4-2')),
            (self.header+'malformed\n'+self.owned,(0,1),(17,'4-2')),
            (self.header+self.owned+'hs 0002 006 003 00000011 000009 4-2\n',(1,2),(17,'4-2'))]:
            with self.subTest(status=status,ports=ports,identity=identity):
                with self.assertRaises(ValueError):lab.resolve_session_port(status,ports,identity)

    def test_invalid_allowlist_rejects_before_open(self):
        for ports in ([],[0,0],[True],[-1],[65536]):
            with patch.object(lab.audio,'opened') as opened:
                with self.assertRaises(ValueError):lab.Session('dualsense','lab',ports)
                opened.assert_not_called()

    def test_constructor_and_repeated_close_follow_handed_off_second_port(self):
        broker=Mock();broker.recv.return_value=b''
        channels=[Mock() for _ in range(3)]
        with patch.object(lab.audio,'opened',return_value=(broker,7,17,'4-2',1,channels)),              patch.object(lab.Path,'read_text',return_value=self.header+self.free+self.owned),              patch.object(lab.subprocess,'run'),              patch.object(lab.audio.live,'resolve_card',return_value=((17,'4-2'),9)) as resolve,              patch.object(lab.audio,'reserve_direct_alsa',return_value={'serial':'synthetic'}),              patch.object(lab.audio,'worker_diagnostics',return_value={}),              patch.object(lab.audio,'message'),              patch.object(lab.audio,'reply',return_value=(2,0x80,struct.pack('<Q',7))),              patch.object(lab,'wait_detached') as detached:
            session=lab.Session('dualsense','lab',(0,1))
            self.assertEqual(session.port,1)
            resolve.assert_called_once_with(1,'dualsense',(17,'4-2'))
            session.close();session.close()
            detached.assert_called_once_with(1,(17,'4-2'))
            broker.close.assert_called_once()
            for channel in channels:channel.close.assert_called_once()
