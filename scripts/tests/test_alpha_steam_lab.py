import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('steam_lab', Path(__file__).parents[1] / 'run-alpha-steam-lab.py')
lab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lab)


class SteamNamespace(unittest.TestCase):
    def test_main_interrupt_writes_failure_and_restores_canary_profile_and_handlers(self):
        import contextlib, io, json, os, signal, sys
        from types import SimpleNamespace
        from unittest.mock import patch
        original = {number: signal.getsignal(number) for number in (signal.SIGTERM, signal.SIGINT)}
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); home = root / 'home'; home.mkdir()
            account = SimpleNamespace(pw_dir=str(home), pw_uid=os.getuid(), pw_gid=os.getgid())
            completed = SimpleNamespace(returncode=0, stdout='{"scope":"fake-namespace"}', stderr='')
            def interrupt(*_args, **_kwargs):
                os.kill(os.getpid(), signal.SIGTERM)
            output = io.StringIO()
            with patch.object(sys, 'argv', ['lab','--apply','--bootstrap','--xvfb',sys.executable,'--work-directory',str(root/'work')]), patch.object(lab.pwd,'getpwuid',return_value=account), patch.object(lab.shutil,'which',return_value='/usr/bin/bwrap'), patch.object(lab.subprocess,'run',return_value=completed), patch.object(lab,'run_bootstrap',side_effect=interrupt), contextlib.redirect_stdout(output):
                self.assertEqual(lab.main(),1)
            receipt = json.loads(output.getvalue())
            self.assertEqual(receipt['status'],'failed')
            self.assertIn('SIGTERM',receipt['initiating'])
            self.assertEqual(receipt['cleanup'],[])
            self.assertEqual(list(home.iterdir()),[])
            self.assertEqual(list((root/'work').iterdir()),[])
        self.assertEqual({number: signal.getsignal(number) for number in original},original)

    def test_interrupt_inside_child_poll_is_not_swallowed_as_oserror(self):
        import os, signal, subprocess, sys
        from unittest.mock import patch
        handlers = {number: signal.signal(number,lab.interrupted) for number in (signal.SIGTERM,signal.SIGINT)}
        child = subprocess.Popen([sys.executable,'-I','-c','import time;time.sleep(30)'],start_new_session=True)
        waitpid = subprocess._del_safe.waitpid
        def interrupt_poll(pid, flags):
            if pid == child.pid:
                os.kill(os.getpid(),signal.SIGTERM)
            return waitpid(pid,flags)
        try:
            with patch.object(subprocess._del_safe,'waitpid',side_effect=interrupt_poll):
                with self.assertRaisesRegex(lab.LabInterrupted,'SIGTERM'):
                    child.poll()
            self.assertIsNone(child.returncode)
            self.assertEqual(signal.getsignal(signal.SIGTERM),signal.SIG_IGN)
            self.assertEqual(signal.getsignal(signal.SIGINT),signal.SIG_IGN)
        finally:
            child.kill(); child.wait(timeout=5)
            for number,handler in handlers.items():signal.signal(number,handler)

    def test_bootstrap_interrupt_reaps_owned_children_and_ignores_repeat_during_cleanup(self):
        import os, signal, subprocess, sys, threading
        from types import SimpleNamespace
        from unittest.mock import patch
        original_popen = subprocess.Popen; original_exists = Path.exists
        handlers = {number: signal.signal(number,lab.interrupted) for number in (signal.SIGTERM,signal.SIGINT)}
        children=[]; timer=None; calls=[]
        try:
            with tempfile.TemporaryDirectory() as directory:
                root=Path(directory); ready=root/'private-display-ready'
                server = [sys.executable,'-I','-c',
                    'import signal,time,sys;from pathlib import Path; p=Path(sys.argv[1]);'
                    'signal.signal(signal.SIGTERM,lambda *_:sys.exit(0));p.touch();'
                    '\ntry:time.sleep(60)\nfinally:p.unlink(missing_ok=True)',str(ready)]
                worker=[sys.executable,'-I','-c','import time;print("fake ready",flush=True);time.sleep(60)']
                def exists(path):
                    if str(path)=='/tmp/.X11-unix/X200':return original_exists(ready)
                    if str(path)=='/tmp/.X200-lock':return False
                    return original_exists(path)
                def spawn(command,**kwargs):
                    nonlocal timer
                    child=original_popen(command,**kwargs);children.append(child);calls.append(kwargs)
                    if len(children)==2:
                        timer=threading.Timer(.1,lambda:os.kill(os.getpid(),signal.SIGTERM));timer.start()
                    return child
                def restore(command,**_kwargs):
                    if command[:3]==['systemctl','--user','stop']:
                        os.kill(os.getpid(),signal.SIGTERM);os.kill(os.getpid(),signal.SIGINT)
                        if len(children)==2 and children[1].poll() is None:os.killpg(children[1].pid,signal.SIGTERM)
                    return SimpleNamespace(returncode=0,stderr=b'')
                account=SimpleNamespace(pw_uid=os.getuid(),pw_gid=os.getgid())
                with patch.object(Path,'exists',exists),patch.object(lab,'display_command',return_value=server),patch.object(lab,'bootstrap_unit_command',return_value=worker),patch.object(lab.subprocess,'Popen',side_effect=spawn),patch.object(lab.subprocess,'run',side_effect=restore):
                    result=lab.run_bootstrap(root,root/'synthetic-home',account,'.vg-alpha-sentinel-fake','marker',Path(sys.executable),30)
                self.assertIn('SIGTERM',result['initiating'])
                self.assertEqual(result['cleanup'],[])
                self.assertFalse(result['accepted'])
                self.assertEqual(len(children),2)
                self.assertTrue(all(child.poll() is not None for child in children))
                self.assertTrue(all(call['start_new_session'] for call in calls))
                self.assertFalse(ready.exists())
        finally:
            if timer is not None:timer.join(timeout=2)
            for child in children:
                if child.poll() is None:os.killpg(child.pid,signal.SIGKILL);child.wait(timeout=5)
            for number,handler in handlers.items():signal.signal(number,handler)

    def test_classic_inventory_keeps_only_owned_js_nodes_and_rejects_replacement(self):
        import os, stat
        from types import SimpleNamespace
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory); inputs=root/'input'; inputs.mkdir(); raw=root/'raw'; raw.mkdir()
            dev=root/'dev'; (dev/'input').mkdir(parents=True)
            for name,owner,minor in [('js0','2a',0),('js1','2b',1),('event0','2a',64)]:
                entry=inputs/name; (entry/'device').mkdir(parents=True)
                (entry/'device/phys').write_text('virtualgamepad/uhid/dualsense/p'+owner+'-i0')
                (entry/'dev').write_text('13:'+str(minor)); (dev/'input'/name).write_text('synthetic')
            original=Path.lstat
            def meta(path):
                if path.parent==dev/'input':
                    minor={'js0':0,'js1':1,'event0':64}[path.name]
                    return SimpleNamespace(st_mode=stat.S_IFCHR|0o660,st_dev=7,st_ino=8,st_rdev=os.makedev(13,minor))
                return original(path)
            with patch.object(lab,'input_owner_identity',return_value=(42,123)),patch.object(Path,'lstat',meta):
                owner,devices=lab.consumer_devices(42,inputs,raw,dev,classic=True)
                self.assertEqual([x['target'] for x in devices],['/dev/input/js0'])
                self.assertIn('/dev/input/js0',lab.consumer_device_settings(devices))
                lab.revalidate_consumer_devices(owner,devices)
                with self.assertRaisesRegex(ValueError,'changed'):
                    lab.consumer_device_settings([{**devices[0],'inode':99}])
                with self.assertRaisesRegex(ValueError,'boolean'):
                    lab.consumer_devices(42,inputs,raw,dev,classic='yes')

    def test_dynamic_classic_aliases_retire_only_missing_owned_nodes(self):
        from unittest.mock import patch
        devices = [dict(target='/dev/input/js0'), dict(target='/dev/input/js1')]
        with tempfile.TemporaryDirectory() as directory:
            view, registry = lab.prepare_consumer_input_view(Path(directory), devices)
            lab.check_consumer_input_view(view, registry)
            with patch.object(lab, 'revalidate_consumer_devices', return_value=['/dev/input/js0']):
                self.assertEqual(lab.retire_consumer_input_aliases((42,123), devices, view, registry), ['/dev/input/js0'])
                lab.retire_consumer_input_aliases((42,123), devices, view, registry)
            self.assertFalse((view/'js0').is_symlink())
            self.assertEqual(str((view/'js1').readlink()), '/owned-input-bindings/input/js1')
            self.assertEqual(registry['retired'], {'/dev/input/js0'})
            with patch.object(lab, 'revalidate_consumer_devices', side_effect=ValueError('owner changed')):
                with self.assertRaisesRegex(ValueError, 'owner changed'):
                    lab.retire_consumer_input_aliases((42,123), devices, view, registry)
            self.assertTrue((view/'js1').is_symlink())

    def test_dynamic_view_rejects_replacement_and_foreign_aliases_without_unlink(self):
        from unittest.mock import patch
        for change in ('directory', 'target', 'foreign'):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as directory:
                workspace=Path(directory); devices=[dict(target='/dev/input/js0')]
                view, registry=lab.prepare_consumer_input_view(workspace, devices)
                if change == 'directory':
                    view.rename(workspace/'original'); view.mkdir(mode=0o700)
                    (view/'js0').symlink_to('/foreign')
                elif change == 'target':
                    (view/'js0').unlink(); (view/'js0').symlink_to('/foreign')
                else: (view/'foreign').write_text('sanitized foreign fixture')
                with patch.object(lab, 'revalidate_consumer_devices', return_value=['/dev/input/js0']):
                    with self.assertRaises(ValueError):
                        lab.retire_consumer_input_aliases((42,123),devices,view,registry)
                self.assertTrue((view/'js0').is_symlink())
                if change=='foreign':self.assertEqual((view/'foreign').read_text(),'sanitized foreign fixture')

    def test_dynamic_view_inventory_and_backing_paths_are_closed(self):
        import os,stat
        from types import SimpleNamespace
        from unittest.mock import patch
        for targets in ([], ['/dev/input/event0'], ['/dev/input/js0','/dev/input/js0'], ['/etc/passwd']):
            with tempfile.TemporaryDirectory() as directory:
                with self.assertRaises(ValueError):
                    lab.prepare_consumer_input_view(Path(directory),[dict(target=t) for t in targets])
                self.assertFalse((Path(directory)/'consumer-input').exists())
        with tempfile.TemporaryDirectory() as directory:
            source=Path(directory)/'source'; source.write_text('sanitized')
            device=dict(target='/dev/input/js0',source=str(source),device=7,inode=8,rdev=os.makedev(13,0))
            view,registry=lab.prepare_consumer_input_view(Path(directory),[device])
            original=Path.lstat
            def meta(path):
                if path==source:return SimpleNamespace(st_mode=stat.S_IFCHR|0o600,st_dev=7,st_ino=8,st_rdev=device['rdev'])
                return original(path)
            with patch.object(Path,'lstat',meta):
                args=lab.consumer_device_settings([device],(view,registry))
                ordinary=lab.consumer_device_settings([device])
            self.assertIn('/owned-input-bindings/input/js0',args)
            self.assertIn(str(view),args)
            self.assertEqual(ordinary[ordinary.index('--dev-bind')+2],'/dev/input/js0')

    def test_namespace_alias_verifies_canonical_target_before_opening_backing(self):
        import os,stat
        from types import SimpleNamespace
        from unittest.mock import patch
        device=dict(target='/dev/input/js0',device=7,inode=8,rdev=os.makedev(13,0))
        char=SimpleNamespace(st_mode=stat.S_IFCHR,st_dev=7,st_ino=8,st_rdev=device['rdev'])
        def glob(path,pattern):return [Path('/dev/input/js0')] if pattern=='js*' else []
        with patch.object(Path,'glob',glob), patch.object(Path,'lstat',return_value=SimpleNamespace(st_mode=stat.S_IFLNK)), \
             patch.object(Path,'stat',return_value=char), patch.object(Path,'readlink',return_value=Path('/owned-input-bindings/input/js0')), \
             patch.object(lab.os,'open',return_value=99) as opened, patch.object(lab.os,'fstat',return_value=char), patch.object(lab.os,'close'):
            self.assertEqual(lab.verify_consumer_namespace([device]),['/dev/input/js0'])
            self.assertEqual(opened.call_args.args[0],'/owned-input-bindings/input/js0')
            with patch.object(Path,'readlink',return_value=Path('/foreign')):
                with self.assertRaisesRegex(ValueError,'alias target changed'):lab.verify_consumer_namespace([device])
            self.assertEqual(opened.call_count,1)

    def test_classic_bootstrap_hints_are_scoped_and_default_is_unchanged(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            workspace=Path(directory)
            with patch.object(lab,'consumer_device_settings',return_value=[]):
                classic=lab.bootstrap_command(workspace,workspace/'synthetic-home',42,43,'.vg-alpha-sentinel-fake','marker',222,60,devices=[{'target':'/dev/input/js0'}])
                ordinary=lab.bootstrap_command(workspace,workspace/'synthetic-home',42,43,'.vg-alpha-sentinel-fake','marker',222,60,devices=[{'target':'/dev/input/event0'}])
            for hint in ['SDL_LINUX_JOYSTICK_CLASSIC','SDL_JOYSTICK_LINUX_CLASSIC','SDL_JOYSTICK_DISABLE_UDEV']:
                self.assertIn(hint,classic); self.assertEqual(classic[classic.index(hint)+1],'1')
                self.assertNotIn(hint,ordinary)


    def test_owned_udev_snapshot_excludes_foreign_records_and_private_properties(self):
        import os
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); data = root/'data'; data.mkdir(); workspace = root/'work'; workspace.mkdir()
            (data/'c13:64').write_text('E:ID_INPUT=1\nE:ID_INPUT_JOYSTICK=1\nE:ID_SEAT=seat-private\nE:PRIVATE_TOKEN=synthetic\nG:seat\n')
            (data/'c13:65').write_text('E:ID_INPUT=1\n')
            devices = [dict(rdev=os.makedev(13,64))]
            with patch.object(lab, 'consumer_device_settings', return_value=[]):
                snapshot = lab.prepare_consumer_udev(workspace, devices, data)
            self.assertEqual([p.name for p in snapshot.iterdir()], ['c13:64'])
            text = (snapshot/'c13:64').read_text()
            self.assertIn('ID_INPUT_JOYSTICK=1', text)
            self.assertIn('ID_SEAT=seat-private', text)
            self.assertNotIn('PRIVATE_TOKEN', text)
            with patch.object(lab, 'consumer_device_settings', return_value=[]):
                command = lab.command(workspace, workspace/'synthetic-home', 42, 43, '.vg-alpha-sentinel-fake', 'marker', devices=devices)
            self.assertIn(str(snapshot), command)
            self.assertIn('/run/udev/data', command)
            self.assertNotIn('/run/udev/data/c13:65', command)


    def test_owned_udev_snapshot_rejects_symlinks_oversize_and_changed_devices(self):
        import os
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); data = root/'data'; data.mkdir()
            record = data/'c13:64'; foreign = root/'foreign'; foreign.write_text('E:ID_INPUT=1\n')
            record.symlink_to(foreign)
            devices = [dict(rdev=os.makedev(13,64))]
            workspace = root/'one'; workspace.mkdir()
            with patch.object(lab, 'consumer_device_settings', return_value=[]):
                with self.assertRaises(OSError): lab.prepare_consumer_udev(workspace,devices,data)
            record.unlink(); record.write_bytes(b'x'*65537)
            workspace = root/'two'; workspace.mkdir()
            with patch.object(lab, 'consumer_device_settings', return_value=[]):
                with self.assertRaisesRegex(ValueError,'bounded regular'): lab.prepare_consumer_udev(workspace,devices,data)
            record.write_text('E:ID_INPUT=1\n'); workspace = root/'three'; workspace.mkdir()
            with patch.object(lab, 'consumer_device_settings', side_effect=[[], ValueError('changed')]):
                with self.assertRaisesRegex(ValueError,'changed'): lab.prepare_consumer_udev(workspace,devices,data)

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

    def test_bootstrap_rebuilds_public_runtime_without_overwriting_retained_profile(self):
        import io
        import os
        import subprocess
        import tarfile
        import shlex
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder); snap = root/'public'; snap.mkdir()
            (snap/'fex_config').mkdir(); (snap/'fex_config'/'settings').write_text('fake public config')
            with tarfile.open(snap/'x86_rootfs.tar.gz', 'w:gz') as archive:
                member = tarfile.TarInfo('x86_rootfs/fake-runtime'); data = b'fake public runtime'
                member.size = len(data); archive.addfile(member, io.BytesIO(data))
            profile = root/'retained'; profile.mkdir(); (profile/'x86_rootfs').mkdir()
            (profile/'x86_rootfs'/'historical').write_text('sanitized prior runtime')
            (profile/'synthetic-login').write_text('fake retained state')
            sentinel = root/'sentinel.py'; sentinel.write_text('pass')
            runtime_parent = root/'fresh'; runtime_parent.mkdir()
            source = lab.BOOTSTRAP.replace('/sentinel.py', str(sentinel)).replace(
                '/extract-rootfs.py', shlex.quote(str(Path(lab.__file__).with_name('extract-alpha-steam-rootfs.py')))).replace(
                '/tmp/virtualgamepad-fex-runtime.', str(runtime_parent/'runtime.')).replace(
                'exec /usr/bin/dbus-run-session -- "$SNAP/usr/bin/FEXBash" -c "$SNAP/usr/bin/steam -no-cef-sandbox"',
                'printf "%s\\n" "$FEX_ROOTFS"')
            paths = []
            for _ in range(2):
                result = subprocess.run(['/bin/bash','-c', source, 'bootstrap','sentinel','marker'],
                    env={**os.environ, 'SNAP':str(snap), 'SNAP_USER_COMMON':str(profile),
                         'SNAP_USER_DATA':str(root/'data')}, check=True, capture_output=True, text=True)
                paths.append(Path(result.stdout.splitlines()[-1]))
            self.assertNotEqual(paths[0], paths[1])
            for path in paths:
                self.assertEqual((path/'fake-runtime').read_bytes(), b'fake public runtime')
                self.assertEqual((path.parent/'fex_config'/'settings').read_text(), 'fake public config')
            self.assertEqual((profile/'synthetic-login').read_text(), 'fake retained state')
            self.assertEqual((profile/'x86_rootfs'/'historical').read_text(), 'sanitized prior runtime')

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
