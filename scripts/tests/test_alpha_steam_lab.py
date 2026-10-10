import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('steam_lab', Path(__file__).parents[1] / 'run-alpha-steam-lab.py')
lab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lab)


class SteamNamespace(unittest.TestCase):
    def test_input_owner_identity_rejects_foreign_dead_and_reused_processes(self):
        import os
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            proc = Path(directory); owner = proc/'42'; owner.mkdir()
            (owner/'status').write_text('Uid: '+(' '.join([str(os.getuid())]*4))+'\n')
            fields = ['S']+['0']*18+['123']
            (owner/'stat').write_text('42 (synthetic name) '+ ' '.join(fields))
            self.assertEqual(lab.input_owner_identity(42,proc),(42,123))
            (owner/'stat').write_text('42 (synthetic name) Z '+ ' '.join(fields[1:]))
            with self.assertRaisesRegex(ValueError,'not live'):lab.input_owner_identity(42,proc)
            (owner/'status').write_text('Uid: 0 0 0 0\n')
            with patch.object(lab.os,'getuid',return_value=42):
                with self.assertRaisesRegex(ValueError,'another identity'):lab.input_owner_identity(42,proc)
            for pid in [0,1,True,'42']:
                with self.assertRaises(ValueError):lab.input_owner_identity(pid,proc)

    def test_consumer_inventory_selects_only_exact_owned_character_nodes_and_pins_identity(self):
        import os,stat
        from types import SimpleNamespace
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); inputs=root/'input';inputs.mkdir();hidraw=root/'hidraw';hidraw.mkdir()
            devices=root/'dev';(devices/'input').mkdir(parents=True)
            for number,physical in [(0,'virtualgamepad/uhid/dualsense/p2a-i0'),
                                    (1,'virtualgamepad/uhid/dualsense/p2b-i0')]:
                entry=inputs/f'event{number}';(entry/'device').mkdir(parents=True)
                (entry/'device/phys').write_text(physical);(entry/'dev').write_text(f'13:{64+number}')
                (devices/'input'/entry.name).write_text('synthetic node')
            original=Path.lstat
            def metadata(path):
                if path.parent==devices/'input':
                    return SimpleNamespace(st_mode=stat.S_IFCHR|0o660,st_dev=7,st_ino=8,st_rdev=os.makedev(13,64))
                return original(path)
            with patch.object(lab,'input_owner_identity',return_value=(42,123)),patch.object(Path,'lstat',metadata):
                identity,selected=lab.consumer_devices(42,inputs,hidraw,devices)
                self.assertEqual(identity,(42,123));self.assertEqual(len(selected),1)
                self.assertEqual(selected[0]['target'],'/dev/input/event0')
                settings=lab.consumer_device_settings(selected)
                self.assertIn(str(devices/'input/event0'),settings)
                self.assertEqual(settings[settings.index('--dev-bind')+1:settings.index('--dev-bind')+3],[str(devices/'input/event0'),'/dev/input/event0'])
                self.assertNotIn(str(devices/'input/event1'),settings)
                with self.assertRaisesRegex(ValueError,'duplicate'):lab.consumer_device_settings(selected*2)
                with self.assertRaisesRegex(ValueError,'changed'):
                    lab.consumer_device_settings([{**selected[0],'inode':99}])
                with self.assertRaisesRegex(ValueError,'invalid'):
                    lab.consumer_device_settings([{**selected[0],'target':'/dev/input/../foreign'}])
            with patch.object(lab,'input_owner_identity',return_value=(42,123)):
                with self.assertRaisesRegex(ValueError,'character device'):lab.consumer_devices(42,inputs,hidraw,devices)
            with patch.object(lab,'input_owner_identity',side_effect=[(42,123),(42,124)]),patch.object(Path,'lstat',metadata):
                with self.assertRaisesRegex(ValueError,'identity changed'):lab.consumer_devices(42,inputs,hidraw,devices)

    def test_consumer_revalidation_allows_removal_but_rejects_replacement_and_owner_exit(self):
        import os,stat
        from types import SimpleNamespace
        from unittest.mock import patch
        device=dict(source='/synthetic/dev/input/event0',target='/dev/input/event0',device=7,inode=8,rdev=os.makedev(13,64))
        same=SimpleNamespace(st_mode=stat.S_IFCHR|0o660,st_dev=7,st_ino=8,st_rdev=device['rdev'])
        with patch.object(lab,'input_owner_identity',return_value=(42,123)):
            with patch.object(Path,'lstat',return_value=same):self.assertEqual(lab.revalidate_consumer_devices((42,123),[device]),[])
            with patch.object(Path,'lstat',side_effect=FileNotFoundError):
                self.assertEqual(lab.revalidate_consumer_devices((42,123),[device]),['/dev/input/event0'])
            with patch.object(Path,'lstat',return_value=SimpleNamespace(**{**vars(same),'st_ino':9})):
                with self.assertRaisesRegex(ValueError,'device identity changed'):lab.revalidate_consumer_devices((42,123),[device])
        with patch.object(lab,'input_owner_identity',return_value=(42,124)):
            with self.assertRaisesRegex(ValueError,'owner identity changed'):lab.revalidate_consumer_devices((42,123),[device])
        with patch.object(lab,'input_owner_identity',side_effect=FileNotFoundError):
            with self.assertRaises(FileNotFoundError):lab.revalidate_consumer_devices((42,123),[device])

    def test_namespace_proof_opens_exact_owned_devices_and_closes_changed_handles(self):
        import os,stat
        from types import SimpleNamespace
        from unittest.mock import patch
        device=dict(source='/synthetic/source',target='/dev/input/event0',device=7,inode=8,rdev=os.makedev(13,64))
        same=SimpleNamespace(st_mode=stat.S_IFCHR|0o660,st_dev=7,st_ino=8,st_rdev=device['rdev'])
        def inventory(path,pattern):return [Path(device['target'])] if path==Path('/dev/input') else []
        with patch.object(Path,'glob',inventory),patch.object(Path,'lstat',return_value=same),patch.object(lab.os,'open',return_value=73) as opened,patch.object(lab.os,'close') as closed:
            with patch.object(lab.os,'fstat',return_value=same):
                self.assertEqual(lab.verify_consumer_namespace([device]),['/dev/input/event0'])
                opened.assert_called_once_with('/dev/input/event0',os.O_RDWR|os.O_NONBLOCK|os.O_NOFOLLOW|os.O_CLOEXEC)
                closed.assert_called_once_with(73)
            closed.reset_mock()
            with patch.object(lab.os,'fstat',return_value=SimpleNamespace(**{**vars(same),'st_ino':9})):
                with self.assertRaisesRegex(ValueError,'opened consumer identity changed'):lab.verify_consumer_namespace([device])
                closed.assert_called_once_with(73)
            opened.reset_mock()
            with patch.object(Path,'glob',return_value=[Path('/dev/input/event99')]):
                with self.assertRaisesRegex(ValueError,'foreign or missing'):lab.verify_consumer_namespace([device])
                opened.assert_not_called()
            with self.assertRaisesRegex(ValueError,'invalid namespace'):lab.verify_consumer_namespace([device]*2)

    def test_rootfs_workspace_rejects_memory_backed_storage_and_low_disk(self):
        lab.require_disk('ext4', 8*1024**3)
        for filesystem, available in [('tmpfs', 8*1024**3), ('ramfs', 8*1024**3), ('ext4', 1024**3)]:
            with self.assertRaisesRegex(RuntimeError, 'disk-backed'): lab.require_disk(filesystem, available)

    def test_memory_guard_retains_reserve_and_rejects_pressure(self):
        healthy = dict(total_bytes=16*1024**3, available_bytes=12*1024**3, full_avg10=0)
        lab.require_memory(healthy)
        for sample in [{**healthy, 'available_bytes': 1024**3}, {**healthy, 'full_avg10': 5}]:
            with self.assertRaisesRegex(RuntimeError, 'memory'): lab.require_memory(sample)
        with tempfile.TemporaryDirectory() as directory:
            info = Path(directory) / 'meminfo'; pressure = Path(directory) / 'pressure'
            info.write_text('MemTotal: 10000000 kB\nMemAvailable: 9000000 kB\n')
            pressure.write_text('some avg10=0.00 avg60=0.00 avg300=0.00 total=0\nfull avg10=0.00 avg60=0.00 avg300=0.00 total=0\n')
            self.assertEqual(lab.memory_sample(info, pressure)['available_bytes'], 9000000*1024)
            info.write_text('MemTotal: 1 kB\nMemAvailable: 2 kB\n')
            with self.assertRaises(RuntimeError): lab.memory_sample(info, pressure)

    def test_actual_passwd_home_is_mapped_and_private_proc_dev_environment_are_required(self):
        command = lab.command(Path('/synthetic/workspace'), Path('/synthetic/account-home'), 42, 43,
                              '.vg-alpha-sentinel-test', 'synthetic')
        self.assertIn('--clearenv', command)
        self.assertIn('--unshare-pid', command)
        self.assertIn('--unshare-user', command)
        self.assertEqual(command[command.index('--cap-drop')+1], 'ALL')
        bind = command.index('--bind')
        self.assertEqual(command[bind+1:bind+3], ['/synthetic/workspace/home', '/synthetic/account-home'])
        self.assertNotIn('--ro-bind-try', command)
        self.assertIn('--dev', command)
        self.assertIn('--proc', command)

    def test_bootstrap_keeps_host_loader_libc_and_owned_display_private(self):
        command = lab.bootstrap_command(Path('/synthetic/workspace'), Path('/synthetic/account-home'),
                                        42, 43, '.vg-alpha-sentinel-test', 'marker', 200, 120)
        self.assertNotIn('LD_LIBRARY_PATH', command)
        self.assertNotIn('--tmpfs', command)
        self.assertIn('/synthetic/workspace/tmp', command)
        self.assertIn('/tmp/.X11-unix/X200', command)
        self.assertNotIn('/run/user/42/bus', command)
        self.assertIn('/auth', command)
        for display, seconds in [(0, 120), (True, 120), (300, 120), (200, 0), (200, 901)]:
            with self.assertRaises(ValueError):
                lab.bootstrap_command(Path('/synthetic/workspace'), Path('/synthetic/account-home'),
                                      42, 43, '.vg-alpha-sentinel-test', 'marker', display, seconds)

    def test_invalid_identity_or_sentinel_is_rejected(self):
        for home, uid, name in [(Path('/'), 42, '.vg-alpha-sentinel-a'),
                                (Path('/synthetic/account-home'), 0, '.vg-alpha-sentinel-a'),
                                (Path('relative'), 42, '.vg-alpha-sentinel-a'),
                                (Path('/synthetic/account-home'), 42, '.vg-alpha-sentinel-../b')]:
            with self.assertRaises(ValueError):
                lab.command(Path('/synthetic/workspace'), home, uid, 43, name, 'synthetic')

    def test_changed_sentinel_is_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            sentinel = Path(directory) / 'sentinel'
            sentinel.write_text('synthetic')
            info = sentinel.stat()
            expected = (info.st_dev, info.st_ino)
            foreign = Path(directory) / 'replacement'
            foreign.write_text('synthetic replacement')
            foreign.replace(sentinel)
            with self.assertRaises(RuntimeError): lab.remove_sentinel(sentinel, expected)
            self.assertTrue(sentinel.exists())

    def test_visible_display_uses_owned_geometry_and_cookie_without_host_auth_binding(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            server=Path(directory)/'server';server.write_text('synthetic')
            auth=Path(directory)/'private-auth'
            with patch.dict(lab.os.environ,{'DISPLAY':':0'}):
                command=lab.display_command(server,200,auth,True)
            self.assertEqual(command[:4],[str(server),':200','-screen','1280x900'])
            self.assertIn('Virtualgamepad alpha: disposable Steam session',command)
            self.assertEqual(command[-5:],['-nolisten','tcp','-auth',str(auth),'-noreset'])
            hidden=lab.display_command(server,200,auth)
            self.assertEqual(hidden[:5],[str(server),':200','-screen','0','1280x900x24'])
            with patch.dict(lab.os.environ,{},clear=True):
                with self.assertRaises(RuntimeError):lab.display_command(server,200,auth,True)
            with self.assertRaises(ValueError):lab.display_command(server,0,auth)

    def test_bootstrap_budget_is_bounded_and_requires_headroom_before_launch(self):
        sample = dict(total_bytes=24*1024**3, available_bytes=20*1024**3, full_avg10=0)
        self.assertEqual(lab.memory_budget(sample), dict(max_bytes=12*1024**3, high_bytes=9*1024**3))
        lab.require_memory(sample, starting=True)
        insufficient = {**sample, 'available_bytes': 5*1024**3}
        lab.require_memory(insufficient)
        with self.assertRaises(RuntimeError): lab.require_memory(insufficient, starting=True)
        command = lab.bootstrap_unit_command('synthetic-owned.service', ['/synthetic/client'], 120, lab.memory_budget(sample))
        self.assertIn('--property=MemoryHigh=9663676416', command)
        self.assertIn('--property=MemoryMax=12884901888', command)
        self.assertIn('--property=MemorySwapMax=0', command)
        self.assertIn('--property=RuntimeMaxSec=120', command)
        self.assertEqual(command[-2:], ['--', '/synthetic/client'])

    def test_oom_termination_cannot_be_reported_as_successful_bootstrap(self):
        self.assertEqual(lab.termination_reason(1, 'Finished with result: oom-kill\n'), 'oom-kill')
        self.assertEqual(lab.termination_reason(0, 'Finished with result: oom-kill\n'), 'oom-kill')
        self.assertEqual(lab.termination_reason(1, 'synthetic failure'), 'nonzero-exit')
        self.assertEqual(lab.termination_reason(0, ''), 'normal-exit')

    def test_only_private_identified_test_profile_can_be_retained_and_reused(self):
        import os
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); workspace = root / 'workspace'; workspace.mkdir()
            home = root / 'normal-home'; home.mkdir()
            profile = root / 'test-profile'
            path, reused = lab.profile_home(workspace, profile, home)
            self.assertEqual(path, profile); self.assertFalse(reused)
            (profile / 'synthetic-login-state').write_text('fake state')
            self.assertTrue(lab.profile_home(workspace, profile, home)[1])
            self.assertEqual((profile / 'synthetic-login-state').read_text(), 'fake state')
            with self.assertRaises(RuntimeError): lab.profile_home(workspace, home, home)
            foreign = root / 'foreign'; foreign.mkdir(mode=0o700)
            with self.assertRaises(OSError): lab.profile_home(workspace, foreign, home)
            alias = root / 'alias'; alias.symlink_to(profile, target_is_directory=True)
            with self.assertRaises(RuntimeError): lab.profile_home(workspace, alias, home)
            profile.chmod(0o755)
            with self.assertRaises(RuntimeError): lab.profile_home(workspace, profile, home)
            profile.chmod(0o700)
            first = lab.lock_profile(profile)
            try:
                with self.assertRaises(RuntimeError): lab.lock_profile(profile)
            finally: os.close(first)
            os.close(lab.lock_profile(profile))
            marker = profile / '.virtualgamepad-alpha-profile'; marker.unlink()
            marker.symlink_to(root / 'synthetic-foreign-marker')
            with self.assertRaises(OSError): lab.profile_home(workspace, profile, home)
            marker.unlink(); os.mkfifo(marker)
            with self.assertRaises(RuntimeError): lab.profile_home(workspace, profile, home)

    def test_namespace_binds_only_selected_test_profile_over_actual_home(self):
        args = lab.command(Path('/synthetic/work'), Path('/synthetic/normal-home'), 42, 43,
                           '.vg-alpha-sentinel-test', 'marker', Path('/synthetic/test-profile'))
        bind = args.index('--bind')
        self.assertEqual(args[bind+1:bind+3], ['/synthetic/test-profile', '/synthetic/normal-home'])
        self.assertNotIn('/synthetic/work/home', args)
