import copy
import importlib.util
import os
import subprocess
import tempfile
import unittest
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
