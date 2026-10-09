import hashlib
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec=importlib.util.spec_from_file_location('updater_package',Path(__file__).parents[1]/'package-alpha-lab-updater.py')
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)


class UpdaterTests(unittest.TestCase):
    def fixture(self):
        config=dict(revision='a'*40,tree='b'*40,stage='/var/lib/virtualgamepad-codex-lab-'+'a'*12,
                    client_uid=1001,worker_uid=1002,unauthorized_uid=1003,ports=[0,1,2,3],
                    phases=list(module.packet.PHASES),hashes={})
        images={name:b'synthetic' for name in (*module.packet.FILES,'gr-privileged-broker','gr-audio-worker')}
        config['hashes']={name:hashlib.sha256(data).hexdigest() for name,data in images.items()}
        helper=dict(config);helper['phases']=module.packet.PHASES
        images['helper.py']=module.packet.HELPER.replace('__CONFIG__',repr(helper)).encode()
        config.update(install_hashes={name:hashlib.sha256(data).hexdigest() for name,data in images.items()},
                      previous_helper_hash='c'*64,client_name='synthetic',compiler='synthetic compiler',lockfile_sha256='d'*64)
        source=module.UPDATER.replace('__CONFIG__',repr(config)).replace('__INSTALLER__',repr(module.packet.INSTALLER)).replace('__HELPER__',repr(module.packet.HELPER)).replace('__FILES__',repr(module.packet.FILES)).replace('__PHASES__',repr(module.packet.PHASES))
        namespace={'__name__':'fixture'};exec(compile(source,'synthetic updater','exec'),namespace)
        return config,images,namespace

    def test_valid_packet_and_fixed_helper_template(self):
        config,images,namespace=self.fixture();namespace['validate'](config,images)
        self.assertEqual(namespace['helper_config'](images['helper.py'])['client_uid'],1001)
        images['helper.py']+=b'\n# changed\n'
        config['install_hashes']['helper.py']=hashlib.sha256(images['helper.py']).hexdigest()
        with self.assertRaisesRegex(ValueError,'fixed template'):namespace['validate'](config,images)

    def test_manifest_cannot_change_destination_identity_ports_phases_or_payload_scope(self):
        for key,value in [('stage','/tmp/foreign'),('revision','../escape'),('client_uid',True),
                          ('worker_uid',0),('unauthorized_uid',7),('ports',[4,5,6,7]),
                          ('phases',['shell']),('client_name','foreign'),('unknown','field'),
                          ('hashes',{}),('compiler','x'*257)]:
            config,images,namespace=self.fixture();config[key]=value
            with self.subTest(key=key),self.assertRaises(ValueError):namespace['validate'](config,images)

    def test_capture_refuses_links_fifo_path_escape_and_oversized_files(self):
        _,_,namespace=self.fixture()
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);(root/'file').write_bytes(b'fake');(root/'link').symlink_to(root/'file');os.mkfifo(root/'fifo')
            fd=os.open(root,os.O_RDONLY|os.O_DIRECTORY)
            try:
                self.assertEqual(namespace['capture'](fd,'file',4),b'fake')
                for name,limit in [('link',4),('fifo',4),('../file',4),('file',3)]:
                    with self.subTest(name=name),self.assertRaises((OSError,ValueError)):namespace['capture'](fd,name,limit)
            finally:os.close(fd)

    def test_revoke_refuses_changed_policy_and_is_repeatable(self):
        import types
        config,_,namespace=self.fixture()
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);state=root/'state';state.mkdir();stage=root/'stage';stage.mkdir()
            helper=root/'helper';helper.write_text("CONFIG={'stage':'"+str(stage)+"'}")
            policy=root/'policy';policy.write_bytes(b'fixed policy')
            (state/'update-sudoers').write_bytes(b'fixed policy')
            (state/'current-helper-sha256').write_text(hashlib.sha256(helper.read_bytes()).hexdigest())
            namespace.update(STATE=state,HELPER=helper,POLICY=policy,__file__=str(root/'updater'),trusted=lambda *_:None,
                             sys=types.SimpleNamespace(flags=types.SimpleNamespace(isolated=True),argv=['updater','revoke']))
            # Synthetic paths substitute the production fixed-stage validation.
            original=namespace['re'].fullmatch
            def matches(pattern,value):
                return True if pattern.startswith('/var/lib/') else original(pattern,value)
            with patch.dict(os.environ,SUDO_UID=str(config['client_uid'])),patch.object(os,'geteuid',return_value=0),patch.object(namespace['re'],'fullmatch',side_effect=matches),patch.object(namespace['subprocess'],'run') as run:
                policy.write_bytes(b'changed')
                with self.assertRaisesRegex(ValueError,'policy changed'):namespace['main']()
                policy.write_bytes(b'fixed policy');namespace['main']();namespace['main']()
                self.assertFalse(policy.exists());self.assertEqual(run.call_count,1)
                self.assertTrue(helper.exists())

    def test_occupied_lab_lock_blocks_update_or_revocation(self):
        import fcntl
        import types
        config,_,namespace=self.fixture()
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);state=root/'state';state.mkdir();stage=root/'stage';stage.mkdir()
            helper=root/'helper';helper.write_text("CONFIG={'stage':'"+str(stage)+"'}")
            (state/'current-helper-sha256').write_text(hashlib.sha256(helper.read_bytes()).hexdigest())
            namespace.update(STATE=state,HELPER=helper,POLICY=root/'policy',__file__=str(root/'updater'),trusted=lambda *_:None,
                             sys=types.SimpleNamespace(flags=types.SimpleNamespace(isolated=True),argv=['updater','update']))
            with (stage/'lock').open('a+b') as lock,patch.dict(os.environ,SUDO_UID=str(config['client_uid'])),patch.object(os,'geteuid',return_value=0),patch.object(namespace['re'],'fullmatch',return_value=True):
                fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
                with self.assertRaises(BlockingIOError):namespace['main']()

    def test_bootstrap_compiles_without_general_sudo(self):
        source=module.BOOTSTRAP.replace('__CONFIG__',repr({'previous_helper_hash':'a'*64,'client_uid':1001})).replace('__IMAGE__',repr(b'synthetic'))
        compile(source,'bootstrap','exec')
        self.assertNotIn('NOPASSWD: ALL',source)
        self.assertIn("('update','revoke')",source)

    def test_revocation_validation_failure_restores_exact_policy(self):
        import subprocess
        import types
        config,_,namespace=self.fixture()
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);state=root/'state';state.mkdir();stage=root/'stage';stage.mkdir()
            helper=root/'helper';helper.write_text("CONFIG={'stage':'"+str(stage)+"'}")
            policy=root/'policy';policy.write_bytes(b'fixed policy');(state/'update-sudoers').write_bytes(b'fixed policy')
            (state/'current-helper-sha256').write_text(hashlib.sha256(helper.read_bytes()).hexdigest())
            namespace.update(STATE=state,HELPER=helper,POLICY=policy,__file__=str(root/'updater'),trusted=lambda *_:None,
                             sys=types.SimpleNamespace(flags=types.SimpleNamespace(isolated=True),argv=['updater','revoke']))
            with patch.dict(os.environ,SUDO_UID=str(config['client_uid'])),patch.object(os,'geteuid',return_value=0),patch.object(namespace['re'],'fullmatch',return_value=True),patch.object(namespace['subprocess'],'run',side_effect=subprocess.CalledProcessError(1,['visudo'])):
                with self.assertRaises(subprocess.CalledProcessError):namespace['main']()
            self.assertEqual(policy.read_bytes(),b'fixed policy')

    def test_bootstrap_partial_install_failure_removes_only_created_files(self):
        import subprocess
        import types
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);helper=root/'helper';helper.write_bytes(b'original lab')
            config=dict(client_uid=1001,previous_helper_hash=hashlib.sha256(helper.read_bytes()).hexdigest())
            source=module.BOOTSTRAP.replace('__CONFIG__',repr(config),1).replace('__IMAGE__',repr(b'fake updater'))
            namespace={'__name__':'fixture'};exec(source,namespace)
            namespace.update(STATE=root/'state',UPDATER=root/'updater',POLICY=root/'policy',HELPER=helper,
                             trusted=lambda *_:None,sys=types.SimpleNamespace(flags=types.SimpleNamespace(isolated=True),argv=['install.py']))
            with patch.object(os,'geteuid',return_value=0),patch.object(namespace['subprocess'],'run',side_effect=[None,subprocess.CalledProcessError(1,['visudo'])]):
                with self.assertRaises(subprocess.CalledProcessError):namespace['main']()
            self.assertEqual(helper.read_bytes(),b'original lab')
            for name in ('state','updater','policy'):self.assertFalse((root/name).exists())
            self.assertFalse(list(root.glob('.alpha-updater-*')))

    def test_update_installs_captured_payloads_with_pinned_installer(self):
        import types
        config,images,namespace=self.fixture()
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);state=root/'state';state.mkdir();stage=root/'stage';stage.mkdir();mailbox=root/'mailbox';mailbox.mkdir()
            helper=root/'helper';helper.write_text("CONFIG={'stage':'"+str(stage)+"'}")
            config['previous_helper_hash']=hashlib.sha256(helper.read_bytes()).hexdigest()
            (state/'current-helper-sha256').write_text(config['previous_helper_hash'])
            policy=root/'policy';policy.write_bytes(b'fixed policy');(state/'update-sudoers').write_bytes(b'fixed policy')
            (mailbox/'manifest.json').write_text(json.dumps(config))
            for name,data in images.items():(mailbox/name).write_bytes(data)
            (mailbox/'install.py').write_text("raise AssertionError('writable installer executed')")
            namespace.update(STATE=state,HELPER=helper,POLICY=policy,__file__=str(root/'updater'),trusted=lambda *_:None,
                             sys=types.SimpleNamespace(flags=types.SimpleNamespace(isolated=True),argv=['updater','update']))
            namespace['CONFIG']['mailbox']=str(mailbox)
            namespace['validate_mailbox']=lambda _:None  # Synthetic account differs from test process UID.
            # The production installer has its own transaction/rollback tests.
            # Here verify the root updater hands it only captured immutable bytes.
            namespace['INSTALLER']="CONFIG=__CONFIG__\nfrom pathlib import Path\ndef main():\n p=Path(__file__).parent\n assert p.parent.name=='state'\n assert not (p/'install.py').exists()\n assert (p/'gr-audio-worker').read_bytes()==b'synthetic'\n"
            with patch.dict(os.environ,SUDO_UID=str(config['client_uid'])),patch.object(os,'geteuid',return_value=0),patch.object(namespace['re'],'fullmatch',return_value=True):
                saved=namespace['INSTALLER']
                namespace['INSTALLER']="def main():raise RuntimeError('synthetic interrupted installation')"
                with self.assertRaisesRegex(RuntimeError,'interrupted'):namespace['main']()
                self.assertEqual((state/'current-helper-sha256').read_text(),config['previous_helper_hash'])
                self.assertFalse(list(state.glob('alpha-update-*')))
                namespace['INSTALLER']=saved
                stale=dict(config,previous_helper_hash='e'*64)
                (mailbox/'manifest.json').write_text(json.dumps(stale))
                with self.assertRaisesRegex(ValueError,'stale packet'):namespace['main']()
                (mailbox/'manifest.json').write_text(json.dumps(config))
                (state/'current-helper-sha256').write_text('f'*64)
                with self.assertRaisesRegex(ValueError,'helper changed'):namespace['main']()
                (state/'current-helper-sha256').write_text(config['previous_helper_hash'])
                namespace['main']()
            self.assertEqual((state/'current-helper-sha256').read_text().strip(),hashlib.sha256(images['helper.py']).hexdigest())
            self.assertFalse(list(state.glob('alpha-update-*')))

    def test_mailbox_requires_fixed_owner_and_private_write_access(self):
        import types
        _,_,namespace=self.fixture()
        namespace['validate_mailbox'](types.SimpleNamespace(st_uid=1001,st_mode=0o700))
        for uid,mode in [(0,0o700),(1002,0o700),(1001,0o720),(1001,0o702)]:
            with self.assertRaisesRegex(ValueError,'private client-owned'):
                namespace['validate_mailbox'](types.SimpleNamespace(st_uid=uid,st_mode=mode))
