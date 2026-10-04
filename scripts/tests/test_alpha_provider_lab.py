import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock, patch

spec = importlib.util.spec_from_file_location('provider_lab', Path(__file__).parents[1] / 'run-alpha-provider-lab.py')
lab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lab)


class Fake:
    def __init__(self, failures=()):
        self.events = []
        self.failures = failures

    def operation(self, name):
        self.events.append(name)
        if name in self.failures:
            raise RuntimeError(name + ' failed')

    def snapshot(self):
        self.operation('snapshot')
        return 'original-state'

    def preflight(self, saved):
        assert saved == 'original-state'
        self.operation('preflight')

    def stop_original_socket(self): self.operation('stop_socket')
    def assert_idle(self, saved): self.operation('idle')
    def stop_original_service(self): self.operation('stop_service')
    def prepare(self): self.operation('prepare')
    def start_candidate(self): self.operation('start')
    def execute(self): self.operation('execute')
    def stop_candidate(self): self.operation('stop_candidate')
    def cleanup(self): self.operation('cleanup')
    def restore(self, saved):
        assert saved == 'original-state'
        self.operation('restore')


class ReversibleMaintenance(unittest.TestCase):
    def test_client_privilege_drop_is_explicit_before_the_validator(self):
        args = Mock(client_uid=1001, timeout=30, command=['/synthetic/validator'])
        host = lab.Host(args)
        host.instance = 'synthetic-instance'
        host.root = Path('/synthetic/lab')
        host.run = Mock()
        with patch.object(lab.pwd, 'getpwuid', return_value=Mock(pw_uid=1001, pw_gid=1002)):
            host.execute()
        command = host.run.call_args.args[0]
        self.assertIn('/usr/bin/setpriv', command)
        for flag in ('--clear-groups', '--bounding-set=-all', '--inh-caps=-all', '--ambient-caps=-all', '--no-new-privs'):
            self.assertLess(command.index(flag), command.index('/synthetic/validator'))

    def test_original_image_receipt_detects_same_inode_content_changes(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'fake-image'
            path.write_bytes(b'fake-one')
            with patch.object(lab, 'trusted'):
                initial = lab.fingerprint(path)
                path.write_bytes(b'fake-two')
                changed = lab.fingerprint(path)
            self.assertEqual(initial['inode'], changed['inode'])
            self.assertNotEqual(initial['sha256'], changed['sha256'])
    def test_preflight_failure_stops_nothing(self):
        host = Fake(['preflight'])
        with self.assertRaises(RuntimeError): lab.maintenance(host)
        self.assertEqual(host.events, ['snapshot', 'preflight'])

    def test_idle_is_rechecked_after_socket_stop_before_service_stop(self):
        host = Fake(['idle'])
        with self.assertRaises(RuntimeError): lab.maintenance(host)
        self.assertNotIn('stop_service', host.events)
        self.assertEqual(host.events[-3:], ['stop_candidate', 'cleanup', 'restore'])

    def test_partial_preparation_start_timeout_and_interrupt_restore(self):
        for failure in ('stop_socket', 'stop_service', 'prepare', 'start', 'execute'):
            host = Fake([failure])
            with self.assertRaisesRegex(RuntimeError, failure): lab.maintenance(host)
            self.assertEqual(host.events[-3:], ['stop_candidate', 'cleanup', 'restore'])

    def test_signal_interruption_restores_and_retains_cause(self):
        class Interrupted(Fake):
            def execute(self):
                self.events.append('execute')
                raise InterruptedError('synthetic SIGTERM')
        host = Interrupted()
        with self.assertRaisesRegex(RuntimeError, 'synthetic SIGTERM'): lab.maintenance(host)
        self.assertEqual(host.events[-3:], ['stop_candidate', 'cleanup', 'restore'])

    def test_cleanup_errors_do_not_prevent_restoration_or_hide_initiating_error(self):
        host = Fake(['execute', 'stop_candidate', 'cleanup'])
        with self.assertRaises(RuntimeError) as caught: lab.maintenance(host)
        for error in ('execute failed', 'stop_candidate failed', 'cleanup failed'):
            self.assertIn(error, str(caught.exception))
        self.assertEqual(host.events[-1], 'restore')

    def test_success_still_stops_candidate_and_restores(self):
        host = Fake()
        lab.maintenance(host)
        self.assertEqual(host.events[-3:], ['stop_candidate', 'cleanup', 'restore'])

    def test_occupied_duplicate_or_malformed_ports_are_rejected(self):
        header = 'hub port sta spd dev sockfd local_busid\n'
        lab.free_port(header + 'hs 2 4 0 00000000 0 0-0', 2)
        for rows in ('hs 2 6 3 00010001 5 1-1', 'hs 2 4 0 0 0 0-0\nhs 2 4 0 0 0 0-0',
                     'hs 2 malformed', 'ss 2 4 0 0 0 0-0'):
            with self.assertRaises((RuntimeError, ValueError)): lab.free_port(header + rows, 2)

    def test_changed_identity_and_repeated_cleanup(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'owned'
            path.write_text('fake')
            expected = lab.identity(path)
            replacement = Path(directory) / 'foreign'
            replacement.write_text('foreign')
            replacement.replace(path)
            with self.assertRaises(RuntimeError): lab.remove_owned(path, expected)
            self.assertEqual(path.read_text(), 'foreign')
            lab.remove_owned(path, lab.identity(path))
            lab.remove_owned(path, lab.identity(Path(directory)))


if __name__ == '__main__':
    unittest.main()
