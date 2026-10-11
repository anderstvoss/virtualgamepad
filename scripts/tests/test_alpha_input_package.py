import contextlib
import hashlib
import importlib.util
import io
from pathlib import Path
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('input_package', Path(__file__).parents[1] / 'package-alpha-input-lab.py')
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


def generated(template, config, location):
    namespace = {'__name__': 'fixture', '__file__': str(location)}
    exec(compile(package.render(template, config), str(location), 'exec'), namespace)
    return namespace


class InputPackage(unittest.TestCase):
    def config(self):
        with patch.object(package.pwd, 'getpwuid'), patch.object(package.grp, 'getgrgid'):
            return package.configuration('a' * 40, 'b' * 40, 1001, 1002)

    def test_only_predefined_scenarios_are_accepted(self):
        helper = generated(package.HELPER, self.config(), Path('/sanitized/helper'))
        for phase in package.PHASES:
            self.assertEqual(helper['scenario'](['run', phase]), phase)
        for arguments in ([], ['run'], ['run', 'gui'], ['run', 'steam-hid', '/bin/sh'],
                          ['run', '../steam-hid'], ['run', 'steam-hid;id'], ['status']):
            with self.subTest(arguments=arguments), self.assertRaises(ValueError):
                helper['scenario'](arguments)

    def test_payloads_are_pinned_and_packet_is_never_overwritten(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'packet'
            images = dict(zip(package.FILES, (b'sanitized script', b'sanitized binary')))
            config = package.write_packet(output, self.config(), images, {'revision': 'a' * 40})
            for name, digest in config['install_hashes'].items():
                self.assertEqual(hashlib.sha256((output / name).read_bytes()).hexdigest(), digest)
            for name in ('helper.py', 'install.py', 'revoke.py'):
                source = (output / name).read_bytes()
                self.assertTrue(source.startswith(b'#!/usr/bin/python3 -I\n'))
                compile(source, name, 'exec')
            with self.assertRaises(FileExistsError):
                package.write_packet(output, self.config(), images, {})
            with self.assertRaises(ValueError):
                package.write_packet(Path(directory) / 'invalid', self.config(), {'foreign': b'x'}, {})

    def test_symlink_fifo_and_oversized_payloads_reject_without_waiting(self):
        import os
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            regular = root / 'regular'; regular.write_bytes(b'sanitized')
            self.assertEqual(package.read_image(regular), b'sanitized')
            link = root / 'link'; link.symlink_to(regular)
            fifo = root / 'fifo'; os.mkfifo(fifo)
            huge = root / 'huge'
            with huge.open('wb') as image:
                image.truncate(128 * 1024 * 1024 + 1)
            for path in (link, fifo, huge):
                with self.subTest(path=path), self.assertRaises((OSError, ValueError)):
                    package.read_image(path)

    def test_invalid_identity_or_revision_never_produces_configuration(self):
        for uid in (0, -1, True, '1001', 2**32 - 1):
            with self.assertRaises(ValueError): package.configuration('a' * 40, 'b' * 40, uid, 1002)
        with patch.object(package.pwd, 'getpwuid'), patch.object(package.grp, 'getgrgid'):
            for revision in ('HEAD', 'a' * 39, '../' + 'a' * 40):
                with self.assertRaises(ValueError): package.configuration(revision, 'b' * 40, 1001, 1002)

    def test_helper_runs_only_pinned_nonroot_fixture_and_rejects_stale_resources(self):
        for phase in package.PHASES:
            with self.subTest(phase=phase), tempfile.TemporaryDirectory() as directory:
                root = Path(directory); config = self.config(); config['stage'] = str(root)
                config['hashes'] = {}
                for name in package.FILES:
                    (root / name).write_bytes(b'sanitized')
                    config['hashes'][name] = hashlib.sha256(b'sanitized').hexdigest()
                (root / 'workspace').mkdir()
                helper = generated(package.HELPER, config, root / 'helper.py')
                helper['trusted'] = lambda *args: None
                with patch.object(helper['os'], 'geteuid', return_value=0), \
                     patch.object(helper['sys'], 'flags', SimpleNamespace(isolated=True)), \
                     patch.object(helper['sys'], 'argv', ['helper', 'run', phase]), \
                     patch.dict(helper['os'].environ, {'SUDO_UID': '1001'}), \
                     patch.object(helper['subprocess'], 'call', return_value=0) as call, contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(helper['main'](), 0)
                    command = call.call_args.args[0]
                    self.assertEqual(command[:4], ['/usr/bin/python3', '-I', str(root / package.FILES[0]), '--apply'])
                    self.assertEqual(command[command.index('--uid') + 1], '1001')
                    self.assertEqual(command[command.index('--input-gid') + 1], '1002')
                    self.assertEqual(command[command.index('--timeout') + 1], '900')
                    self.assertEqual('--evdev' in command, phase == 'steam-evdev')
                    self.assertEqual(call.call_args.kwargs['env'], {'PATH': '/usr/bin:/bin', 'LANG': 'C'})
                    (root / 'workspace' / ('control-' + phase)).mkdir()
                    with self.assertRaisesRegex(ValueError, 'previous control resources'):
                        helper['main']()
                    self.assertEqual(call.call_count, 1)
                    (root / package.FILES[0]).write_bytes(b'changed')
                    with self.assertRaisesRegex(ValueError, 'installed image changed'):
                        helper['main']()
                    self.assertEqual(call.call_count, 1)

    def test_revocation_refuses_resources_and_preserves_snapshot(self):
        import os
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); config = self.config(); config['stage'] = str(root)
            config['uid'] = os.getuid()
            config['helper'] = str(root / 'helper'); config['policy'] = str(root / 'policy')
            config['install_hashes'] = {'helper.py': hashlib.sha256(b'sanitized').hexdigest()}
            for name in ('helper', 'policy'): (root / name).write_bytes(b'sanitized')
            workspace = root / 'workspace'; workspace.mkdir()
            resource = workspace / 'control-steam-hid'; resource.mkdir()
            revoke = generated(package.REVOKER, config, root / 'revoke.py')
            checked = []
            revoke['checked'] = lambda path, digest: checked.append(path)
            with patch.object(revoke['os'], 'geteuid', return_value=0), \
                 patch.object(revoke['sys'], 'flags', SimpleNamespace(isolated=True)), \
                 patch.object(revoke['sys'], 'argv', ['revoke.py']), contextlib.redirect_stdout(io.StringIO()):
                with self.assertRaisesRegex(ValueError, 'owned trial resources remain'): revoke['main']()
                self.assertTrue((root / 'policy').exists())
                resource.rmdir()
                revoke['main']()
            self.assertFalse((root / 'policy').exists())
            self.assertFalse((root / 'helper').exists())
            self.assertTrue(workspace.is_dir())
            self.assertEqual(checked, [root / 'helper', root / 'policy'] * 2)

    def installer_fixture(self, root):
        images = dict(zip(package.FILES, (b'sanitized script', b'sanitized binary')))
        config = self.config()
        for key in ('stage', 'helper', 'policy'): config[key] = str(root / key)
        output = root / 'packet'
        config = package.write_packet(output, config, images, {})
        namespace = generated(package.INSTALLER, config, output / 'install.py')
        namespace['trusted'] = lambda path: None  # No root permissions in this fixture.
        return namespace, config

    def test_installation_has_exact_two_policy_lines_and_nonroot_workspace(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); namespace, config = self.installer_fixture(root)
            policies = []
            def validate(command, **kwargs):
                if '-f' in command: policies.append(Path(command[-1]).read_text())
            with patch.object(namespace['os'], 'geteuid', return_value=0), \
                 patch.object(namespace['sys'], 'flags', SimpleNamespace(isolated=True)), \
                 patch.object(namespace['sys'], 'argv', ['install.py']), \
                 patch.dict(namespace['os'].environ, {'SUDO_UID': '1001'}), \
                 patch.object(namespace['os'], 'chown') as chown, \
                 patch.object(namespace['subprocess'], 'run', side_effect=validate), contextlib.redirect_stdout(io.StringIO()):
                namespace['main']()
            expected = ''.join('#1001 ALL=(root) NOPASSWD: ' + config['helper'] + ' run ' + phase + '\n' for phase in package.PHASES)
            self.assertEqual(policies, [expected])
            self.assertEqual(Path(config['policy']).read_text(), expected)
            self.assertEqual(len(expected.splitlines()), 2)
            chown.assert_called_once_with(Path(config['stage']) / 'workspace', 1001, -1)

    def test_preparation_and_final_policy_failure_restore_only_created_paths(self):
        for fail_call in (1, 2):
            with self.subTest(fail_call=fail_call), tempfile.TemporaryDirectory() as directory:
                root = Path(directory); namespace, config = self.installer_fixture(root)
                calls = 0
                def validate(*args, **kwargs):
                    nonlocal calls
                    calls += 1
                    if calls == fail_call: raise RuntimeError('sanitized validation failure')
                with patch.object(namespace['os'], 'geteuid', return_value=0), \
                     patch.object(namespace['sys'], 'flags', SimpleNamespace(isolated=True)), \
                     patch.object(namespace['sys'], 'argv', ['install.py']), \
                     patch.dict(namespace['os'].environ, {'SUDO_UID': '1001'}), \
                     patch.object(namespace['os'], 'chown'), \
                     patch.object(namespace['subprocess'], 'run', side_effect=validate), \
                     self.assertRaisesRegex(RuntimeError, 'sanitized validation failure'):
                    namespace['main']()
                for key in ('stage', 'helper', 'policy'): self.assertFalse(Path(config[key]).exists())
                self.assertEqual(list(root.glob('.alpha-input-*')), [])
                self.assertTrue((root / 'packet').is_dir())

    def test_partial_workspace_creation_is_registered_before_ownership_change(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); namespace, config = self.installer_fixture(root)
            with patch.object(namespace['os'], 'geteuid', return_value=0), \
                 patch.object(namespace['sys'], 'flags', SimpleNamespace(isolated=True)), \
                 patch.object(namespace['sys'], 'argv', ['install.py']), \
                 patch.dict(namespace['os'].environ, {'SUDO_UID': '1001'}), \
                 patch.object(namespace['os'], 'chown', side_effect=OSError('sanitized ownership failure')), \
                 patch.object(namespace['subprocess'], 'run'), self.assertRaisesRegex(OSError, 'sanitized ownership failure'):
                namespace['main']()
            for key in ('stage', 'helper', 'policy'): self.assertFalse(Path(config[key]).exists())

    def test_generated_policy_passes_real_sudoers_parser(self):
        import subprocess
        parser = Path('/usr/sbin/visudo')
        if not parser.exists(): self.skipTest('sudoers parser unavailable')
        config = self.config()
        policy = ''.join('#1001 ALL=(root) NOPASSWD: ' + config['helper'] + ' run ' + phase + '\n' for phase in package.PHASES)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'policy'; path.write_text(policy); path.chmod(0o440)
            result = subprocess.run([str(parser), '-c', '-f', str(path)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)

    def test_changed_payload_is_rejected_before_any_installation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); namespace, config = self.installer_fixture(root)
            (root / 'packet' / package.FILES[0]).write_bytes(b'changed')
            with patch.object(namespace['os'], 'geteuid', return_value=0), \
                 patch.object(namespace['sys'], 'flags', SimpleNamespace(isolated=True)), \
                 patch.object(namespace['sys'], 'argv', ['install.py']), \
                 patch.dict(namespace['os'].environ, {'SUDO_UID': '1001'}), \
                 self.assertRaisesRegex(ValueError, 'approved image changed'):
                namespace['main']()
            for key in ('stage', 'helper', 'policy'): self.assertFalse(Path(config[key]).exists())


if __name__ == '__main__': unittest.main()
