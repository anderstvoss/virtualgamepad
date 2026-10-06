import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('steam_lab', Path(__file__).parents[1] / 'run-alpha-steam-lab.py')
lab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lab)


class SteamNamespace(unittest.TestCase):
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
