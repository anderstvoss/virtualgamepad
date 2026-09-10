import copy
import importlib.util
import os
import subprocess
import tempfile
import unittest
from types import SimpleNamespace
from pathlib import Path
from unittest.mock import patch


def module(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).resolve().parents[1] / (name + '.py'))
    loaded = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loaded)
    return loaded


HELPER = module('virtualgamepad-host-helper')
INSTALLER = module('install-host-helper')
UID = 42
ORIGINAL = HELPER.acl_text(HELPER.parse_acl('user::rw-\ngroup::---\nother::---\n'))


class Host:
    def __init__(self):
        self.current = ORIGINAL
        self.node = {'dev': 1, 'ino': 2, 'rdev': 3, 'owner': 0, 'group': 0}
        self.actions = []
        self.fail = None

    def prepare(self):
        self.actions.append('prepare')
        if self.fail == 'prepare':
            raise ValueError('synthetic missing mechanism')
        return True

    def identity(self):
        return dict(self.node)

    def acl(self):
        return self.current

    def set_acl(self, acl):
        self.actions.append('set-acl')
        if self.fail == 'before-write':
            raise OSError('synthetic denied write')
        self.current = acl
        if self.fail == 'after-write':
            raise OSError('synthetic interrupted writer')

    def refresh_policy(self):
        self.actions.append('udev-policy')
        if self.fail == 'udev':
            raise OSError('synthetic udev failure')
        self.current = HELPER.acl_text(HELPER.parse_acl('user::rw-\ngroup::rw-\nother::---\n'))

    def run(self, arguments):
        self.actions.append(arguments)
        return 'synthetic output'


class Journal:
    def __init__(self):
        self.value = None
        self.fail_granted = False

    def load(self):
        return copy.deepcopy(self.value)

    def save(self, value):
        if self.fail_granted and value['phase'] == 'granted':
            raise OSError('synthetic journal failure')
        self.value = copy.deepcopy(value)

    def clear(self):
        self.value = None


class HelperTests(unittest.TestCase):
    def test_authorization_and_argument_boundaries(self):
        policy = HELPER.parse_policy('allow_uid=42\nallow_module=uhid\n')
        self.assertEqual(HELPER.authorize(policy, '42'), 42)
        for uid in ['0', '43', '-1', '', '42;command', str(2**32)]:
            with self.assertRaises(ValueError):
                HELPER.authorize(policy, uid)
        for args in [[], ['uhid-grant', '/dev/other'], ['run-job', '../escape'],
                     ['module-load', 'uhid', 'parameter=x'], ['sh'], ['run-job', 'job;sh']]:
            with self.assertRaises(ValueError):
                HELPER.parse_action(args)
        host = Host()
        with self.assertRaises(ValueError):
            HELPER.run_operation('module-load', 'unapproved', policy, host)
        self.assertEqual(host.actions, [])
        HELPER.run_operation('module-load', 'uhid', policy, host)
        self.assertEqual(host.actions, [['/usr/sbin/modprobe', 'uhid']])

    def test_acl_mask_expansion_preserves_other_effective_permissions(self):
        acl = 'user::rw-\nuser:7:rw-\ngroup::rwx\ngroup:8:rw-\nmask::r--\nother::---\n'
        result = HELPER.parse_acl(HELPER.grant_acl(acl, UID))
        self.assertEqual(result['user', '42'], 'rw-')
        self.assertEqual(result['user', '7'], 'r--')
        self.assertEqual(result['group', ''], 'r--')
        self.assertEqual(result['group', '8'], 'r--')
        self.assertEqual(result['other', ''], '---')
        for invalid in ['# file: /outside\n' + acl, acl + 'user:7:r--\n', 'user::rw-\n', acl.replace('mask::r--\n', '')]:
            with self.assertRaises(ValueError):
                HELPER.parse_acl(invalid)

    def test_udev_settles_before_baseline_and_timeout_leaves_acl_untouched(self):
        for fail_settle in [False, True]:
            with tempfile.TemporaryDirectory() as temporary:
                class PreparingHost(HELPER.Host):
                    def __init__(self):
                        super().__init__()
                        self.registration = Path(temporary) / 'registration'
                        self.device = Path(temporary) / 'device'
                        self.calls = []

                    def run(self, argv, data=None):
                        self.calls.append(argv)
                        if argv[0] == '/usr/sbin/modprobe':
                            self.registration.touch()
                            self.device.touch()
                        elif argv[0] == '/usr/bin/udevadm' and fail_settle:
                            raise OSError('synthetic udev timeout')
                        return ''

                    def identity(self):
                        self.calls.append('baseline')
                        return {'group': 7}

                    def acl(self):
                        return ORIGINAL

                    def set_acl(self, acl):
                        self.calls.append('write')
                        raise OSError('stop synthetic run after baseline')

                host, journal = PreparingHost(), Journal()
                with self.assertRaises(OSError):
                    HELPER.execute('uhid-grant', UID, host, journal)
                self.assertEqual(host.calls[:2], [['/usr/sbin/modprobe', 'uhid'], ['/usr/bin/udevadm', 'settle', '--timeout=10']])
                if fail_settle:
                    self.assertNotIn('baseline', host.calls)
                    self.assertNotIn('write', host.calls)
                    self.assertEqual(journal.value['phase'], 'preparing')
                else:
                    self.assertEqual(journal.value['identity'], {'group': 7})
                    self.assertIn('write', host.calls)

    def test_real_acl_encoding_roundtrip_on_private_synthetic_file(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'fixture'
            path.write_text('synthetic')
            path.chmod(0o600)
            host = HELPER.Host()
            host.device = path
            original = host.acl()
            expected = HELPER.grant_acl(original, UID)
            host.set_acl(expected)
            self.assertEqual(host.acl(), expected)
            host.set_acl(original)
            self.assertEqual(host.acl(), original)

    def test_grant_repeated_polling_restore_and_idempotence(self):
        host, journal = Host(), Journal()
        HELPER.execute('uhid-grant', UID, host, journal)
        self.assertNotEqual(host.current, ORIGINAL)
        actions = list(host.actions)
        HELPER.execute('uhid-grant', UID, host, journal)
        for _ in range(3):
            self.assertTrue(HELPER.execute('status', UID, host, journal)['active'])
        self.assertEqual(host.actions, actions)
        HELPER.execute('uhid-restore', UID, host, journal)
        self.assertEqual(host.current, ORIGINAL)
        self.assertIsNone(journal.value)
        actions = list(host.actions)
        HELPER.execute('uhid-restore', UID, host, journal)
        self.assertEqual(host.actions, actions)

    def test_oversized_journal_is_rejected_before_filesystem_mutation(self):
        with patch.object(HELPER.tempfile, 'mkstemp') as create:
            with self.assertRaisesRegex(ValueError, 'too large'):
                HELPER.Journal().save({'payload': 'x' * 20000})
            create.assert_not_called()

    def test_failed_preparation_and_interrupted_grants_recover(self):
        for failure in ['prepare', 'before-write', 'after-write', 'journal']:
            with self.subTest(failure=failure):
                host, journal = Host(), Journal()
                host.fail = failure
                journal.fail_granted = failure == 'journal'
                with self.assertRaises((ValueError, OSError)):
                    HELPER.execute('uhid-grant', UID, host, journal)
                self.assertIsNotNone(journal.value)
                host.fail = None
                HELPER.execute('uhid-restore', UID, host, journal)
                self.assertEqual(host.current, ORIGINAL)
                self.assertIsNone(journal.value)

    def test_group_race_recovery_reapplies_policy_and_retries_failed_udev(self):
        host, journal = Host(), Journal()
        host.fail = 'after-write'
        with self.assertRaises(OSError):
            HELPER.execute('uhid-grant', UID, host, journal)
        host.node['group'] = 7
        host.fail = 'udev'
        with self.assertRaises(OSError):
            HELPER.execute('uhid-recover-udev', UID, host, journal)
        self.assertEqual(journal.value['phase'], 'reconciling')
        self.assertEqual(host.current, ORIGINAL)
        host.fail = None
        HELPER.execute('uhid-recover-udev', UID, host, journal)
        self.assertIsNone(journal.value)
        self.assertNotIn(('user', str(UID)), HELPER.parse_acl(host.current))
        self.assertEqual(HELPER.parse_acl(host.current)['group', ''], 'rw-')
        HELPER.execute('uhid-recover-udev', UID, host, journal)

    def test_group_race_repair_still_refuses_unrelated_changes(self):
        for change in ['inode', 'acl', 'preexisting-named-acl']:
            host, journal = Host(), Journal()
            if change == 'preexisting-named-acl':
                host.current = HELPER.grant_acl(ORIGINAL, 7)
            host.fail = 'after-write'
            with self.assertRaises(OSError):
                HELPER.execute('uhid-grant', UID, host, journal)
            host.fail = None
            host.node['group'] = 7
            if change == 'inode':
                host.node['ino'] += 1
            elif change == 'acl':
                host.current = HELPER.grant_acl(host.current, 77)
            actions = list(host.actions)
            with self.assertRaises(ValueError):
                HELPER.execute('uhid-recover-udev', UID, host, journal)
            self.assertEqual(host.actions, actions)

    def test_other_research_changes_and_replaced_nodes_are_not_overwritten(self):
        for change in ['node', 'acl', 'owner']:
            host, journal = Host(), Journal()
            HELPER.execute('uinput-grant', UID, host, journal)
            if change == 'node':
                host.node['ino'] += 1
            elif change == 'acl':
                host.current = HELPER.grant_acl(host.current, 77)
            else:
                journal.value['uid'] = 77
            actions = list(host.actions)
            with self.assertRaises(ValueError):
                HELPER.execute('uinput-restore', UID, host, journal)
            self.assertEqual(host.actions, actions)
            self.assertIsNotNone(journal.value)

    def test_jobs_use_only_installed_fixed_arguments(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            jobs = root / 'host-jobs'
            jobs.mkdir()
            (jobs / 'probe.json').write_text('{"argv":["/usr/bin/true","fixed"]}')
            policy = HELPER.parse_policy('allow_uid=42\nallow_job=probe\n')
            host = Host()
            with patch.object(HELPER, 'CONFIG', root / 'config'), patch.object(HELPER, 'trusted') as trusted:
                HELPER.run_operation('run-job', 'probe', policy, host)
                self.assertEqual(host.actions, [['/usr/bin/true', 'fixed']])
                self.assertEqual(trusted.call_count, 2)
                with self.assertRaises(ValueError):
                    HELPER.run_operation('run-job', 'unapproved', policy, host)

    def test_privileged_paths_reject_writable_or_non_root_ancestors(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'untrusted'
            path.write_text('synthetic')
            with self.assertRaises(ValueError):
                HELPER.trusted(path)
            with self.assertRaises(ValueError):
                INSTALLER.directory(Path(temporary))

    def test_sudoers_and_bootstrap_exclude_general_root_commands(self):
        rule = INSTALLER.sudo_rule(UID)
        self.assertIn(b'#42 ALL=(root) NOPASSWD: /usr/local/libexec/virtualgamepad-host-helper', rule)
        for forbidden in [b'NOPASSWD: ALL', b'/usr/bin/python', b'install-host-helper', b'/bin/sh']:
            self.assertNotIn(forbidden, rule)
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'rule'
            path.write_bytes(rule)
            result = subprocess.run(['/usr/sbin/visudo', '-cf', str(path)], capture_output=True)
            self.assertEqual(result.returncode, 0, result.stderr)
        with self.assertRaises(ValueError):
            INSTALLER.sudo_rule(0)

    def test_update_preserves_policy_and_journals_and_rejects_unsafe_state(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            config, rule, state, executable = [root / name for name in ('config', 'rule', 'state', 'helper')]
            config.write_text('allow_uid=42\nallow_job=custom\n')
            config.chmod(0o600)
            rule.write_text('existing rule')
            rule.chmod(0o440)
            state.mkdir(mode=0o700)
            journal = state / 'uhid.json'
            journal.write_text('synthetic active journal')
            executable.write_bytes(b'old helper')
            executable.chmod(0o755)
            original_lstat = Path.lstat

            def root_metadata(path):
                metadata = original_lstat(path)
                return SimpleNamespace(st_uid=0, st_mode=metadata.st_mode)

            with patch.multiple(INSTALLER, CONFIG=config, RULE=rule, STATE=state, EXECUTABLE=executable), \
                    patch.object(INSTALLER, 'directory'), patch.object(Path, 'lstat', root_metadata):
                INSTALLER.update_helper(42, b'new helper')
                self.assertEqual(executable.read_bytes(), b'new helper')
                self.assertEqual(config.read_text(), 'allow_uid=42\nallow_job=custom\n')
                self.assertEqual(rule.read_text(), 'existing rule')
                self.assertEqual(journal.read_text(), 'synthetic active journal')
                with self.assertRaises(ValueError):
                    INSTALLER.update_helper(43, b'wrong account')
                state.chmod(0o755)
                with self.assertRaises(ValueError):
                    INSTALLER.update_helper(42, b'unsafe state')
                state.chmod(0o700)
                rule.chmod(0o666)
                with self.assertRaises(ValueError):
                    INSTALLER.update_helper(42, b'unsafe rule')
                self.assertEqual(executable.read_bytes(), b'new helper')

    def test_installer_does_not_overwrite_existing_files(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'installed'
            with patch.object(INSTALLER, 'directory'):
                INSTALLER.install_file(path, b'approved', 0o600)
                self.assertEqual(path.read_bytes(), b'approved')
                with self.assertRaises(ValueError):
                    INSTALLER.install_file(path, b'replacement', 0o600)
                self.assertEqual(path.read_bytes(), b'approved')


if __name__ == '__main__':
    unittest.main()
