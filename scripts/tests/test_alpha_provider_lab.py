import importlib.util
from pathlib import Path
import tempfile
import unittest
import subprocess
import select
import sys
import os
from unittest.mock import Mock, patch

spec = importlib.util.spec_from_file_location('provider_lab', Path(__file__).parents[1] / 'run-alpha-provider-lab.py')
lab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lab)


class BrokerConfiguration(unittest.TestCase):
    def test_one_and_four_ports_have_exactly_one_identity_and_worker_pair(self):
        for ports in ((0,), (0, 1, 2, 3)):
            with self.subTest(ports=ports):
                expected = ['allow_uid=1001', 'instance=lab-test']
                expected += [f'allow_vhci_port={port}' for port in ports]
                expected += ['worker_uid=1002', 'worker_gid=1003']
                config = lab.broker_config(1001, 'lab-test', ports, 1002, 1003)
                self.assertEqual(config, '\n'.join(expected) + '\n')
                self.assertEqual(config.count('allow_uid='), 1)
                self.assertEqual(config.count('instance='), 1)
                self.assertEqual(config.count('worker_uid='), 1)
                self.assertEqual(config.count('worker_gid='), 1)

    def test_invalid_instance_is_rejected_before_config_generation(self):
        for instance in ('', 'Uppercase', 'lab\nallow_uid=1002', 'x' * 33):
            with self.subTest(instance=instance):
                with self.assertRaises(RuntimeError):
                    lab.broker_config(1001, instance, (0,), 1002, 1003)


class AudioIsolationPolicy(unittest.TestCase):
    def test_serial_and_platform_require_separate_matching_ancestors(self):
        lines = lab.audio_isolation_rule('lab-123').splitlines()
        self.assertEqual(len(lines), 2)
        self.assertIn('ATTRS{serial}=="vg-lab-123-*"', lines[0])
        self.assertNotIn('ACP_IGNORE', lines[0])
        self.assertNotIn('KERNELS', lines[0])
        self.assertIn('ENV{VG_ALPHA_AUDIO_INSTANCE}=="lab-123"', lines[1])
        self.assertIn('SUBSYSTEMS=="platform", KERNELS=="vhci_hcd.0"', lines[1])
        self.assertIn('ENV{ACP_IGNORE}="1"', lines[1])
        self.assertNotIn('ATTRS{serial}', lines[1])

    def test_rule_injection_and_wildcard_instances_reject(self):
        for instance in ['', 'a_b', '../escape', '*', 'a"', 'a\n', 'A', 'a'*33]:
            with self.subTest(instance=instance):
                with self.assertRaisesRegex(RuntimeError, 'invalid audio isolation'):
                    lab.audio_isolation_rule(instance)


class AudioIsolationLifecycle(unittest.TestCase):
    def policy(self, directory, failures=()):
        calls = []
        def run(command):
            calls.append(command)
            if len(calls) in failures:
                raise RuntimeError('synthetic preparation/restoration failure')
        return lab.AudioIsolation('lab-test', run, Path(directory) / 'rules', lambda _: []), calls

    def test_verification_failure_restores_owned_rule_and_directory(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(lab, 'trusted'):
            policy, calls = self.policy(directory, failures=(1,))
            with self.assertRaises(RuntimeError): policy.prepare()
            self.assertTrue(policy.path.exists())
            policy.restore()
            self.assertFalse(policy.directory.exists())
            self.assertEqual(calls[-1], ['udevadm', 'control', '--reload-rules'])
            policy.restore()
            self.assertEqual(len(calls), 2)

    def test_changed_content_or_identity_keeps_rule(self):
        for replacement in (False, True):
            with tempfile.TemporaryDirectory() as directory, patch.object(lab, 'trusted'):
                policy, _ = self.policy(directory)
                policy.prepare()
                if replacement:
                    foreign = policy.directory / 'foreign'
                    foreign.write_text('synthetic replacement')
                    foreign.replace(policy.path)
                else:
                    policy.path.write_text('synthetic changed content')
                with self.assertRaisesRegex(RuntimeError, 'rule changed'): policy.restore()
                self.assertTrue(policy.path.exists())

    def test_active_device_refuses_rule_removal_then_restores_repeatedly(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(lab, 'trusted'):
            policy, calls = self.policy(directory)
            policy.prepare()
            policy.inventory = lambda _: ['synthetic-bus']
            with self.assertRaisesRegex(RuntimeError, 'devices remain'): policy.restore(timeout=0)
            self.assertTrue(policy.path.exists())
            policy.inventory = lambda _: []
            policy.restore()
            policy.restore()
            self.assertEqual(len(calls), 3)

    def test_delayed_kernel_removal_waits_without_touching_live_rule(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(lab, 'trusted'):
            policy, _ = self.policy(directory)
            policy.prepare()
            observations=[]
            def inventory(_):
                observations.append(policy.path.exists())
                return ['synthetic-owned-device'] if len(observations)==1 else []
            policy.inventory=inventory
            with patch.object(lab.time, 'sleep') as pause:
                policy.restore()
                pause.assert_called_once_with(.05)
            self.assertEqual(observations,[True,True])
            self.assertFalse(policy.path.exists())
            policy.restore()

    def test_delayed_removal_identity_change_still_refuses_cleanup(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(lab, 'trusted'):
            policy, _ = self.policy(directory)
            policy.prepare()
            calls=[]
            def inventory(_):
                calls.append(True)
                return ['synthetic-owned-device'] if len(calls)==1 else []
            policy.inventory=inventory
            with patch.object(lab.time, 'sleep', side_effect=lambda _: policy.path.write_text('synthetic changed rule')):
                with self.assertRaisesRegex(RuntimeError, 'rule changed'):
                    policy.restore()
            self.assertTrue(policy.path.exists())

    def test_reload_failure_retains_pending_restoration_and_retries(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(lab, 'trusted'):
            policy, _ = self.policy(directory, failures=(3,))
            policy.prepare()
            with self.assertRaises(RuntimeError): policy.restore()
            self.assertFalse(policy.path.exists())
            self.assertTrue(policy.reload_pending)
            policy.restore()
            self.assertFalse(policy.directory.exists())

    def test_changed_directory_retains_original_rule_and_foreign_directory(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(lab, 'trusted'):
            policy, _ = self.policy(directory)
            policy.prepare()
            original = Path(directory) / 'original'
            policy.directory.rename(original)
            policy.directory.mkdir()
            with self.assertRaisesRegex(RuntimeError, 'directory changed'): policy.restore()
            self.assertTrue((original / policy.path.name).exists())
            self.assertTrue(policy.directory.exists())

    def test_interrupted_preparation_still_has_restoration_identity(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(lab, 'trusted'):
            policy, _ = self.policy(directory)
            with patch.object(policy, 'run', side_effect=InterruptedError('synthetic interruption')):
                with self.assertRaises(InterruptedError): policy.prepare()
            policy.restore()
            self.assertFalse(policy.directory.exists())

    def test_occupied_rule_is_never_replaced_or_removed(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(lab, 'trusted'):
            policy, calls = self.policy(directory)
            policy.directory.mkdir()
            policy.path.write_text('synthetic foreign rule')
            with self.assertRaises(FileExistsError): policy.prepare()
            policy.restore()
            self.assertEqual(policy.path.read_text(), 'synthetic foreign rule')
            self.assertEqual(calls, [])


class PredefinedProviderPhases(unittest.TestCase):
    def test_only_closed_phase_names_and_compiled_arguments_are_accepted(self):
        images = Path('/synthetic/immutable')
        for phase in lab.PHASES:
            command = lab.phase_command(phase, images, 'lab', 0, (1,2,3))
            self.assertEqual(command[:2], ['/usr/bin/python3', '-I'])
            self.assertTrue(Path(command[2]).is_relative_to(images))
            self.assertNotIn('/bin/sh', command)
        for phase in ['', '../escape', 'provider-lifecycle;reboot', 'steam', 'arbitrary']:
            with self.assertRaises(ValueError): lab.phase_command(phase, images, 'lab', 0)
        for port in [-1, True, 65536]:
            with self.assertRaises(ValueError): lab.phase_command('usb-functional', images, 'lab', port)

    def test_explicit_port_allowlist_rejects_duplicates_overflow_and_bools(self):
        self.assertEqual(lab.selected_ports(0,(1,2,3)), (0,1,2,3))
        for first, additional in [(0,(0,)), (0,(1,2,3,4)), (-1,()), (0,(True,)), (0,(65536,))]:
            with self.assertRaises(ValueError): lab.selected_ports(first,additional)
        with self.assertRaises(ValueError): lab.phase_command('provider-siblings-admission', Path('/synthetic'), 'lab',0)

    def test_invalid_phase_refuses_even_dry_run_without_privileges(self):
        completed = subprocess.run([sys.executable, '-I', str(Path(lab.__file__)),
                                    '--phase', 'arbitrary'], capture_output=True, text=True)
        self.assertEqual(completed.returncode, 2)
        self.assertIn('invalid choice', completed.stderr)


class WorkerFaultOwnership(unittest.TestCase):
    def snapshot(self):
        return dict(start=123, parent=42, uids=['997']*4, caps=0, nnp='1', groups=[],
                    image=(1,2), cgroups=['0::/system.slice/owned.service'],
                    argv=[b'worker', b'dualsense', b'9', b'7', b'020102030405', b'lab', b'11', b'12', b'13'])

    def verify(self, snapshot):
        lab.validate_worker(snapshot, 42, 997, (1,2), '/system.slice/owned.service', 'lab', 7, 9)

    @unittest.skipUnless(hasattr(os, 'pidfd_open') and hasattr(lab.signal, 'pidfd_send_signal'), 'Linux pidfds unavailable')
    def test_actual_pidfd_terminates_only_a_child_owned_by_this_test(self):
        child = subprocess.Popen([sys.executable, '-I', '-c',
            "import time; print('READY', flush=True); time.sleep(30)"], stdout=subprocess.PIPE)
        held = None
        try:
            # Popen's exec handshake is not application readiness. Reserve only
            # after the owned child has completed interpreter initialization.
            if not select.select([child.stdout], [], [], 3)[0]:
                self.fail('owned child did not become ready within three seconds')
            self.assertEqual(child.stdout.readline(6), b'READY\n')
            def verify(snapshot):
                if snapshot['parent'] != os.getpid(): raise RuntimeError('synthetic child parent changed')
            held = lab.PinnedWorker(child.pid, verify)
            held.kill()
            self.assertEqual(child.wait(timeout=3), -lab.signal.SIGKILL)
        finally:
            if held is not None: held.close()
            if child.poll() is None: child.kill(); child.wait(timeout=3)
            child.stdout.close()

    def test_identity_requires_root_unit_image_parent_credentials_and_generation(self):
        original = self.snapshot(); self.verify(original)
        for key, changed in [('parent',43), ('uids',['0']*4), ('caps',1), ('nnp','0'), ('groups',['1']),
                             ('image',(1,3)), ('cgroups',['0::/foreign']), ('argv',[b'foreign'])]:
            with self.subTest(key=key), self.assertRaisesRegex(RuntimeError, 'owned session'):
                self.verify({**original, key:changed})
        for index, changed in [(2,b'10'),(3,b'8'),(5,b'foreign')]:
            argv = original['argv'].copy(); argv[index]=changed
            with self.assertRaises(RuntimeError): self.verify({**original,'argv':argv})

    def test_pidfd_is_acquired_after_validation_and_is_closed_idempotently(self):
        with patch.object(lab, 'worker_snapshot', return_value=self.snapshot()), \
             patch.object(lab.os, 'pidfd_open', return_value=77) as opened, \
             patch.object(lab.os, 'close') as closed, patch.object(lab.signal, 'pidfd_send_signal') as sent:
            worker = lab.PinnedWorker(100, self.verify)
            worker.kill(); worker.close(); worker.close()
            opened.assert_called_once_with(100)
            sent.assert_called_once_with(77, lab.signal.SIGKILL)
            closed.assert_called_once_with(77)
            with self.assertRaises(RuntimeError): worker.kill()

    def test_identity_change_after_pidfd_open_refuses_signal_and_closes_handle(self):
        with patch.object(lab, 'worker_snapshot', side_effect=[self.snapshot(), {**self.snapshot(),'start':124}]), \
             patch.object(lab.os, 'pidfd_open', return_value=77), patch.object(lab.os, 'close') as closed, \
             patch.object(lab.signal, 'pidfd_send_signal') as sent:
            with self.assertRaisesRegex(RuntimeError, 'changed'): lab.PinnedWorker(100, self.verify)
            closed.assert_called_once_with(77); sent.assert_not_called()

    def test_foreign_identity_never_opens_a_process_capability(self):
        with patch.object(lab, 'worker_snapshot', return_value={**self.snapshot(),'image':(9,9)}), \
             patch.object(lab.os, 'pidfd_open') as opened:
            with self.assertRaises(RuntimeError): lab.PinnedWorker(100,self.verify)
            opened.assert_not_called()


class BrokerCrashRestoration(unittest.TestCase):
    def test_pending_restart_preserves_journal_and_operator_clear_requires_held_identity(self):
        for changed in (False,True):
            with self.subTest(changed=changed), tempfile.TemporaryDirectory() as directory:
                root=Path(directory); (root/'bin').mkdir(); (root/'bin/gr-privileged-broker').write_bytes(b'synthetic image')
                record=root/'record'; record.write_bytes(b'1 7 9 0\n'); expected=lab.identity(record)
                vhci=root/'status'; vhci.write_text('hub port sta spd dev sockfd local_busid\nhs 0 004 000 0 0 0-0\n')
                host=lab.Host(Mock(port=0,additional_port=[],client_uid=42,probe_directory=root))
                host.root=root; host.instance='lab'; host.service='owned.service'; host.socket='owned.socket'
                host.run=Mock(side_effect=lambda command: 'failed' if 'ActiveState' in command else '0')
                host.start_candidate=Mock()
                def rejected(uid,command,suffix):
                    self.assertEqual(record.read_bytes(),b'1 7 9 0\n')
                    self.assertIn('startup-rejection',command)
                    if changed: record.write_bytes(b'synthetic changed record')
                host.run_client=Mock(side_effect=rejected)
                worker=Mock(descriptor=77); broker=Mock()
                with patch.object(lab,'PinnedWorker',return_value=broker), \
                     patch.object(lab,'VHCI',vhci), patch('select.select',return_value=([77],[],[])):
                    if changed:
                        with self.assertRaisesRegex(RuntimeError,'journal preserved'):
                            host.broker_death(100,'/owned',worker,record,expected,7,9)
                        self.assertEqual(record.read_bytes(),b'synthetic changed record')
                    else:
                        host.broker_death(100,'/owned',worker,record,expected,7,9)
                        self.assertFalse(record.exists())
                        self.assertTrue(host.events[-1]['broker_death_recovery']['pending_restart_rejected'])
                broker.kill.assert_called_once(); broker.close.assert_called_once()
                self.assertIn((record,expected,False),host.owned)

    def test_worker_survival_retains_evidence_and_never_starts_recovery(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory); (root/'bin').mkdir(); (root/'bin/gr-privileged-broker').write_bytes(b'synthetic image')
            record=root/'record'; record.write_bytes(b'1 7 9 0\n')
            host=lab.Host(Mock(port=0,additional_port=[])); host.root=root; host.run=Mock()
            broker=Mock()
            with patch.object(lab,'PinnedWorker',return_value=broker),patch('select.select',return_value=([],[],[])):
                with self.assertRaisesRegex(RuntimeError,'survived'): host.broker_death(100,'/owned',Mock(descriptor=77),record,lab.identity(record),7,9)
            host.run.assert_not_called(); self.assertTrue(record.exists()); broker.close.assert_called_once()


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
    def test_fingerprint_uses_actual_configured_executable_not_package_alias(self):
        value = '{ path=/synthetic/installed/broker ; argv[]=/synthetic/installed/broker --config /synthetic/policy ; ignore_errors=no ; pid=42 ; code=(null) ; status=0/0 }'
        self.assertEqual(lab.installed_executable({'ExecStart': value}), Path('/synthetic/installed/broker'))
        for properties in ({}, {'ExecStart': ''}, {'ExecStart': value + value}, {'ExecStart': value.replace('/synthetic/installed/broker', 'relative')}):
            with self.assertRaises(RuntimeError):lab.installed_executable(properties)

    def test_temporary_names_map_to_valid_broker_instances(self):
        self.assertEqual(lab.instance_name(Path('/synthetic/virtualgamepad-alpha-a_b_c_d_')), 'virtualgamepad-alpha-a-b-c-d-')
        self.assertEqual(lab.instance_name(Path('/synthetic/virtualgamepad-alpha-12345678')), 'virtualgamepad-alpha-12345678')
        for name in ('Uppercase', 'a' * 33, 'dot.name'):
            with self.assertRaises(RuntimeError): lab.instance_name(Path('/synthetic') / name)

    def test_failed_command_preserves_bounded_stdout_and_stderr(self):
        host = lab.Host(Mock())
        with self.assertRaises(subprocess.CalledProcessError):
            host.run([sys.executable, '-c', 'import sys; print("fake output"); print("fake cause", file=sys.stderr); sys.exit(7)'])
        self.assertEqual(host.events[-1]['status'], 7)
        self.assertEqual(host.events[-1]['stdout'], 'fake output\n')
        self.assertEqual(host.events[-1]['stderr'], 'fake cause\n')

    def test_verified_journal_stdio_is_not_an_application_client(self):
        stdio = 'u_str ESTAB 0 0 * 101 * 202 users:(("fake",pid=42,fd=2),("fake",pid=42,fd=1))'
        journal = 'u_str ESTAB 0 0 /run/systemd/journal/stdout 202 * 101 users:(("journal",pid=7,fd=9))'
        lab.require_no_clients(stdio + '\n' + journal, 42, 7)
        for text, peer in ((stdio, 7), (stdio + '\n' + journal, None),
                           (stdio + '\n' + journal, 8),
                           (stdio.replace('fd=2', 'fd=4') + '\n' + journal, 7),
                           (stdio + '\n' + journal.replace('202', '303'), 7),
                           (stdio + '\n' + journal + '\n' + journal, 7)):
            with self.assertRaisesRegex(RuntimeError, 'connected clients'):
                lab.require_no_clients(text, 42, peer)
        client = 'u_str ESTAB 0 0 /run/virtualgamepad/broker.sock 303 * 404 users:(("fake",pid=42,fd=5))'
        with self.assertRaisesRegex(RuntimeError, 'connected clients'):
            lab.require_no_clients(stdio + '\n' + journal + '\n' + client, 42, 7)

    def test_staging_uses_trusted_executable_filesystem_not_run(self):
        with patch.object(lab, 'trusted'), patch.object(lab.os, 'statvfs', return_value=Mock(f_flag=0)), patch.object(lab.tempfile, 'mkdtemp', return_value='/synthetic/stage') as create:
            self.assertEqual(lab.allocate_staging_directory(), Path('/synthetic/stage'))
            self.assertEqual(create.call_args.kwargs['dir'], Path('/var/lib'))
        with patch.object(lab, 'trusted'), patch.object(lab.os, 'statvfs', return_value=Mock(f_flag=lab.os.ST_NOEXEC)), patch.object(lab.tempfile, 'mkdtemp') as create:
            with self.assertRaisesRegex(RuntimeError, 'noexec'):
                lab.allocate_staging_directory()
            create.assert_not_called()

    def test_noexec_preflight_never_reaches_service_or_resource_checks(self):
        host = lab.Host(Mock())
        with patch.object(lab, 'require_executable_staging', side_effect=RuntimeError('noexec')), patch.object(host, 'assert_idle') as idle:
            with self.assertRaisesRegex(RuntimeError, 'noexec'):
                host.preflight({})
            idle.assert_not_called()

    def test_verbose_validator_hits_output_quota_instead_of_unbounded_capture(self):
        host = lab.Host(Mock())
        with self.assertRaises(subprocess.CalledProcessError):
            host.run([sys.executable, '-c', 'print("x" * 2000000)'])
        self.assertNotEqual(host.events[-1]['status'], 0)
    def test_client_privilege_drop_is_explicit_before_the_validator(self):
        args = Mock(client_uid=1001, timeout=30, command=['/synthetic/validator'], unauthorized_probe=None, restart_empty=False, phase=None, additional_port=[])
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

    def test_unauthorized_identity_is_distinct_and_registered_before_startup(self):
        args = Mock(client_uid=1001, unauthorized_uid=1003, timeout=30,
                    command=['/synthetic/validator'], unauthorized_probe=Path('/synthetic/probe'), restart_empty=False, phase=None, additional_port=[])
        host = lab.Host(args)
        host.instance = 'synthetic-instance'
        host.root = Path('/synthetic/lab')
        host.run = Mock()
        with patch.object(lab.pwd, 'getpwuid', side_effect=lambda uid: Mock(pw_uid=uid, pw_gid=uid)):
            host.execute()
        self.assertEqual(host.client_units, ['synthetic-instance-client.service', 'synthetic-instance-unauthorized.service'])
        command = host.run.call_args.args[0]
        self.assertIn('--reuid=1003', command)
        self.assertIn('--unauthorized', command)
        self.assertIn('--clear-groups', command)

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


    def test_empty_candidate_restart_reconnects_and_records_new_identity(self):
        host = lab.Host(Mock(client_uid=1001, port=0, command=['/synthetic/probe']))
        host.instance = 'synthetic'; host.service = 'synthetic.service'; host.socket = 'synthetic.socket'
        host.owned = [(lab.STATE / name, (1, 2), True) for name in ('synthetic', 'synthetic.audio')]
        host.run = Mock(side_effect=['42\n', '', '', 'inactive\n', '43\n'])
        host.start_candidate = Mock(); host.run_client = Mock()
        with patch.object(lab, 'identity', return_value=(1, 2)), patch.object(Path, 'iterdir', side_effect=lambda: iter([])), patch.object(Path, 'read_text', return_value=''), patch.object(lab, 'free_port'), patch.object(lab, 'process_children', return_value=[]):
            host.restart_empty_candidate()
        host.run_client.assert_called_once_with(1001, ['/synthetic/probe'], 'reconnected')
        host.start_candidate.assert_called_once()
        self.assertEqual(host.events[-1]['empty_restart'], dict(previous_pid=42, current_pid=43))
        self.assertEqual(host.run.call_args_list[1].args[0], ['systemctl', 'stop', 'synthetic.socket'])

    def test_restart_refuses_changed_identity_nonempty_journal_or_children(self):
        for changed, entries, children in ((True, [], ''), (False, [Path('/synthetic/lease')], ''), (False, [], '7')):
            host = lab.Host(Mock(port=0)); host.instance = 'synthetic'; host.service = 'synthetic.service'
            host.owned = [(lab.STATE / name, (1, 2), True) for name in ('synthetic', 'synthetic.audio')]
            host.run = Mock(return_value='42\n'); host.start_candidate = Mock()
            with patch.object(lab, 'identity', return_value=(9, 9) if changed else (1, 2)), patch.object(Path, 'iterdir', side_effect=lambda: iter(entries)), patch.object(Path, 'read_text', return_value=children), patch.object(lab, 'free_port'), patch.object(lab, 'process_children', return_value=[7] if children else []):
                with self.assertRaisesRegex(RuntimeError, 'refusing restart|unexplained children'): host.restart_empty_candidate()
            self.assertFalse(any('stop' in call.args[0] for call in host.run.call_args_list))
            host.start_candidate.assert_not_called()

    def test_restart_failure_still_restores_original_service_state(self):
        host = Fake()
        def execute():
            host.operation('execute')
            host.operation('restart')
            raise RuntimeError('candidate restart failed')
        host.execute = execute
        with self.assertRaisesRegex(RuntimeError, 'candidate restart failed'):
            lab.maintenance(host)
        self.assertEqual(host.events[-3:], ['stop_candidate', 'cleanup', 'restore'])

    def test_child_on_nonleader_thread_prevents_false_idle(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / '42'; (root / 'task' / '42').mkdir(parents=True)
            (root / 'task' / '43').mkdir()
            (root / 'stat').write_text('42 (fake name) ' + ' '.join(['S'] + ['0'] * 18 + ['100']))
            (root / 'task' / '42' / 'children').write_text('')
            (root / 'task' / '43' / 'children').write_text('7 8')
            self.assertEqual(lab.process_children(42, Path(directory)), [7, 8])
            (root / 'task' / '43' / 'children').unlink()
            with self.assertRaisesRegex(RuntimeError, 'ownership could not be verified'):
                lab.process_children(42, Path(directory))

    def test_child_inspection_refuses_pid_reuse_or_task_change(self):
        root = Path('/synthetic/proc')
        task = root / '42' / 'task' / '42'
        def record(start):return '42 (fake) ' + ' '.join(['S'] + ['0'] * 18 + [str(start)])
        for stat, tasks in (([record(100), '', record(101)], [[task], [task]]), ([record(100), '', record(100)], [[task], []])):
            with patch.object(Path, 'read_text', side_effect=stat), patch.object(Path, 'iterdir', side_effect=tasks):
                with self.assertRaisesRegex(RuntimeError, 'ownership could not be verified'):
                    lab.process_children(42, root)


if __name__ == '__main__':
    unittest.main()


class UnitStopOwnership(unittest.TestCase):
    def test_replaced_candidate_unit_is_never_stopped(self):
        host = lab.Host(Mock())
        host.units = ['synthetic.service']
        host.unit_images = {'synthetic.service': {'sha256': 'owned'}}
        host.run = Mock(side_effect=['loaded', '/run/systemd/system/synthetic.service'])
        with patch.object(lab, 'fingerprint', return_value={'sha256': 'foreign'}):
            with self.assertRaisesRegex(RuntimeError, 'identity changed'): host.stop_candidate()
        self.assertFalse(any('stop' in call.args[0] for call in host.run.call_args_list))

    def test_owned_candidate_unit_stops_and_absent_unit_is_idempotent(self):
        host = lab.Host(Mock())
        host.units = ['synthetic.service']
        host.unit_images = {'synthetic.service': {'sha256': 'owned'}}
        host.run = Mock(side_effect=['loaded', '/run/systemd/system/synthetic.service', '', 'not-found'])
        with patch.object(lab, 'fingerprint', return_value={'sha256': 'owned'}):
            host.stop_candidate(); host.stop_candidate()
        self.assertEqual(sum('stop' in call.args[0] for call in host.run.call_args_list), 1)


class FaultJournalPort(unittest.TestCase):
    def test_root_journal_can_select_any_explicitly_authorized_port(self):
        self.assertEqual(lab.journal_port(b'1 7 17 1\n',7,17,(0,1,2,3)),1)
        for data in (b'1 7 17 4\n',b'1 8 17 1\n',b'1 7 18 1\n',b'2 7 17 1\n',b'1 7 17 1\n1 7 17 1\n'):
            with self.subTest(data=data):
                with self.assertRaises(RuntimeError):lab.journal_port(data,7,17,(0,1,2,3))

    def test_followup_normal_phase_carries_full_allowlist(self):
        command=lab.phase_command('provider-lifecycle',Path('/synthetic/images'),'lab',0,(1,2,3))
        self.assertEqual(command[-5:],['--ports','0','1','2','3'])
