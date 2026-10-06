import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec=importlib.util.spec_from_file_location('packet',Path(__file__).parents[1]/'package-alpha-provider-lab.py')
packet=importlib.util.module_from_spec(spec);spec.loader.exec_module(packet)


class ProviderPacket(unittest.TestCase):
    def test_identity_validation_never_accepts_root_or_repeated_identity(self):
        with patch.object(packet.pwd,'getpwuid') as lookup:
            for ids in [(0,2,3),(1,1,3),(1,-1,3),(True,2,3)]:
                with self.assertRaises(ValueError):packet.validate_identities(*ids)
            lookup.assert_not_called()
            packet.validate_identities(1,2,3)
            self.assertEqual(lookup.call_count,3)

    def test_only_four_explicit_sorted_high_speed_ports_are_authorized(self):
        for ports in [[0,1,2],[0,1,2,15],[1,0,2,3],[0,1,2,2],[False,1,2,3]]:
            with self.assertRaises(ValueError):packet.validate_ports(ports)
        packet.validate_ports([0,1,2,3])

    def test_payloads_cannot_be_symlinks_or_block_on_fifo(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);(root/'file').write_bytes(b'synthetic')
            self.assertEqual(packet.read_image(root/'file'),b'synthetic')
            (root/'link').symlink_to(root/'file');os.mkfifo(root/'fifo')
            with self.assertRaises(OSError):packet.read_image(root/'link')
            with self.assertRaises(ValueError):packet.read_image(root/'fifo')

    def test_generated_root_scripts_compile_and_expose_only_named_actions(self):
        config=dict(client_uid=1001,client_name='synthetic',phases=packet.PHASES)
        helper=packet.HELPER.replace('__CONFIG__',repr(config))
        installer=packet.INSTALLER.replace('__CONFIG__',repr(config))
        compile(helper,'helper.py','exec');compile(installer,'install.py','exec')
        self.assertIn("argv[1] in CONFIG['phases']",helper)
        self.assertIn('payload hash mismatch',installer)
        self.assertIn('existing policy is not the reviewed three-action scope',installer)
        self.assertIn('previous-sudoers',installer)
        self.assertNotIn('NOPASSWD: ALL',installer)

    def installer(self, root):
        import hashlib
        import types
        config=dict(stage=str(root/'stage'),revision='a'*40,client_uid=1001,client_name='synthetic',
                    phases=packet.PHASES,install_hashes={'helper.py':hashlib.sha256(b'synthetic helper').hexdigest()},
                    previous_helper_hash=hashlib.sha256(b'previous helper').hexdigest())
        namespace={'__name__':'test_installer','__file__':str(root/'source/install.py')}
        exec(packet.INSTALLER.replace('__CONFIG__',repr(config)),namespace)
        namespace['SOURCE']=root/'source';namespace['SOURCE'].mkdir()
        (namespace['SOURCE']/'helper.py').write_bytes(b'synthetic helper')
        namespace['HELPER']=root/'bin/helper';namespace['HELPER'].parent.mkdir()
        namespace['HELPER'].write_bytes(b'previous helper')
        namespace['POLICY']=root/'sudoers/policy';namespace['POLICY'].parent.mkdir()
        namespace['POLICY'].write_text(''.join('synthetic ALL=(root) NOPASSWD: '+str(namespace['HELPER'])+' '+action+'\n'
                                              for action in ('status','run','receipt')))
        namespace['sys']=types.SimpleNamespace(flags=types.SimpleNamespace(isolated=True),argv=['install.py'])
        namespace['root_path']=lambda *_:None  # Synthetic paths owned by the test UID.
        return namespace

    def test_bad_payload_and_broad_existing_policy_reject_before_privileged_state(self):
        for failure in ('hash','policy'):
            with tempfile.TemporaryDirectory() as directory:
                namespace=self.installer(Path(directory));stage=namespace['STAGE']
                if failure=='hash':(namespace['SOURCE']/'helper.py').write_bytes(b'changed')
                else:namespace['POLICY'].write_text('synthetic ALL=(root) NOPASSWD: ALL\n')
                with patch.object(packet.os,'geteuid',return_value=0):
                    with self.assertRaises(ValueError):namespace['main']()
                self.assertFalse(stage.exists())
                self.assertEqual(namespace['HELPER'].read_bytes(),b'previous helper')

    def test_post_install_validation_failure_restores_both_scoped_files(self):
        import subprocess
        with tempfile.TemporaryDirectory() as directory:
            namespace=self.installer(Path(directory))
            before=namespace['POLICY'].read_bytes()
            with patch.object(packet.os,'geteuid',return_value=0), \
                 patch.object(namespace['subprocess'],'run',side_effect=[None,subprocess.CalledProcessError(1,['visudo'])]):
                with self.assertRaises(subprocess.CalledProcessError):namespace['main']()
            self.assertEqual(namespace['HELPER'].read_bytes(),b'previous helper')
            self.assertEqual(namespace['POLICY'].read_bytes(),before)
            self.assertEqual((namespace['STAGE']/'previous-sudoers').read_bytes(),before)
            self.assertFalse(list(namespace['POLICY'].parent.glob('.virtualgamepad-alpha-*')))

    def test_existing_three_actions_accept_split_or_combined_rules(self):
        for account in ('synthetic', '#1001'):
            for combined in (False, True):
                with self.subTest(account=account, combined=combined), tempfile.TemporaryDirectory() as directory:
                    namespace=self.installer(Path(directory))
                    commands=[str(namespace['HELPER'])+' '+action for action in ('status','run','receipt')]
                    prefix=account+' ALL=(root) NOPASSWD: '
                    lines=[prefix+', '.join(commands)] if combined else [prefix+command for command in commands]
                    namespace['POLICY'].write_text('# sanitized scope\n'+'\n'.join(lines)+'\n')
                    with patch.object(packet.os,'geteuid',return_value=0), patch.object(namespace['subprocess'],'run'):
                        namespace['main']()
                    self.assertTrue(namespace['STAGE'].is_dir())

    def test_combined_rules_reject_extra_commands_duplicates_and_changed_scope(self):
        for suffix in ('ALL', '/synthetic/foreign status', 'run extra', 'status', 'receipt'):
            with self.subTest(suffix=suffix), tempfile.TemporaryDirectory() as directory:
                namespace=self.installer(Path(directory))
                helper=str(namespace['HELPER'])
                commands=', '.join(helper+' '+action for action in ('status','run','receipt'))
                namespace['POLICY'].write_text('synthetic ALL=(root) NOPASSWD: '+commands+', '+
                                              (suffix if suffix.startswith('/') or suffix=='ALL' else helper+' '+suffix)+'\n')
                before=namespace['POLICY'].read_bytes()
                with patch.object(packet.os,'geteuid',return_value=0):
                    with self.assertRaises(ValueError):namespace['main']()
                self.assertFalse(namespace['STAGE'].exists())
                self.assertEqual(namespace['POLICY'].read_bytes(),before)
                self.assertEqual(namespace['HELPER'].read_bytes(),b'previous helper')
