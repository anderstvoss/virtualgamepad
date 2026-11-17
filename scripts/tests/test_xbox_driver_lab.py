import importlib.util
from pathlib import Path
import socket
import stat
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location(
    'xbox_driver_lab', Path(__file__).resolve().parents[1] / 'xbox-driver-lab.py')
lab = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(lab)


class DriverLab(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        (self.root / 'devices').mkdir()
        usb = patch.object(lab, 'USB', self.root)
        usb.start()
        self.addCleanup(usb.stop)
        self.interface = self.device('1-1', '0b12')

    def device(self, port, product):
        device = self.root / 'physical' / port
        device.mkdir(parents=True)
        (self.root / 'devices' / port).symlink_to(device)
        for field, value in {'idVendor': '045e', 'idProduct': product,
                             'bConfigurationValue': '1', 'devnum': '9'}.items():
            (device / field).write_text(value)
        interface = device / f'{port}:1.0'
        interface.mkdir()
        (interface / 'bInterfaceNumber').write_text('00')
        link = self.root / 'devices' / interface.name
        link.symlink_to(interface)
        return link

    def test_exact_usb_identity_and_ambiguity(self):
        self.device('1-2', '028e')
        self.assertEqual(lab.handle('status'), '1-1:1.0: unbound')
        self.device('1-3', '0b12')
        with self.assertRaisesRegex(ValueError, 'exactly one'):
            lab.handle('xone')

    def test_failed_switch_restores_previous_binding(self):
        link = self.interface / 'driver'
        link.symlink_to('xpad')
        steps = []

        def bind(reference, name, action):
            self.assertEqual(reference, (self.interface, '9'))
            steps.append((name, action))
            if action == 'unbind':
                link.unlink()
            elif name == 'xone-wired':
                raise OSError('probe failed')
            else:
                link.symlink_to(name)

        with patch.object(lab, 'load'), patch.object(lab, 'binding', side_effect=bind):
            with self.assertRaisesRegex(OSError, 'probe failed'):
                lab.handle('xone')
        self.assertEqual(steps, [('xpad', 'unbind'), ('xone-wired', 'bind'), ('xpad', 'bind')])
        self.assertEqual(lab.driver(self.interface), 'xpad')

    def test_loading_an_already_bound_driver_does_not_disconnect(self):
        (self.interface / 'driver').symlink_to('xpad')
        with patch.object(lab, 'run') as run, patch.object(lab, 'binding') as binding:
            self.assertEqual(lab.handle('xpad'), '1-1:1.0: xpad')
        run.assert_called_once_with('modprobe', 'xpad')
        binding.assert_not_called()

    def test_global_unload_refuses_another_bound_controller(self):
        directory = self.root / 'drivers' / 'xpad'
        directory.mkdir(parents=True)
        other = self.device('1-2', '028e')
        (directory / other.name).symlink_to(other)
        with patch.object(lab, 'run') as run, patch.object(Path, 'exists', return_value=True):
            with self.assertRaisesRegex(ValueError, 'another USB device'):
                lab.handle('unload-xpad')
        run.assert_not_called()

    def test_unrecognized_request_never_reaches_sysfs(self):
        with patch.object(lab, 'target') as target:
            with self.assertRaisesRegex(ValueError, 'unknown request'):
                lab.handle('unload-snd')
        target.assert_not_called()

    def hid_fixture(self):
        hid = self.root / 'hid'
        modules = self.root / 'modules'
        directory = hid / 'drivers' / 'xpadneo'
        directory.mkdir(parents=True)
        modules.mkdir()
        return hid, modules, directory

    def test_xpadneo_load_and_repeated_unload_need_no_wired_reference(self):
        hid, modules, _ = self.hid_fixture()
        self.interface.parent.joinpath('1-1').unlink()
        calls = []

        def execute(command, name):
            self.assertEqual(name, 'hid_xpadneo')
            calls.append(command)
            module = modules / name
            if command == 'modprobe':
                module.mkdir()
            else:
                module.rmdir()

        with patch.object(lab, 'HID', hid), patch.object(lab, 'MODULE_SYSFS', modules), \
                patch.object(lab, 'run', side_effect=execute):
            self.assertEqual(lab.handle('load-xpadneo'), 'xpadneo: loaded')
            self.assertEqual(lab.handle('unload-xpadneo'), 'xpadneo: unloaded')
            self.assertEqual(lab.handle('unload-xpadneo'), 'xpadneo: unloaded')
        self.assertEqual(calls, ['modprobe', 'rmmod'])

    def test_xpadneo_unload_allows_one_series_and_refuses_other_devices(self):
        hid, modules, directory = self.hid_fixture()
        (modules / 'hid_xpadneo').mkdir()
        device = hid / 'devices' / '0005:045E:0B13.0001'
        device.mkdir(parents=True)
        (device / 'uevent').write_text('HID_ID=0005:0000045E:00000B13\n')
        (directory / device.name).symlink_to(device)
        with patch.object(lab, 'HID', hid), patch.object(lab, 'MODULE_SYSFS', modules), \
                patch.object(lab, 'run') as run:
            lab.handle('unload-xpadneo')
            run.assert_called_once_with('rmmod', 'hid_xpadneo')
            run.reset_mock()
            (device / 'uevent').write_text('HID_ID=0005:0000045E:0000028E\n')
            lab.handle('unload-xpadneo')
            run.assert_called_once_with('rmmod', 'hid_xpadneo')
            run.reset_mock()
            (directory / '0005:045E:0B13.0002').symlink_to(device)
            with self.assertRaisesRegex(ValueError, 'additional or unsupported'):
                lab.handle('unload-xpadneo')
            run.assert_not_called()
            (directory / '0005:045E:0B13.0002').unlink()
            (device / 'uevent').write_text('HID_ID=0005:0000045E:00000B22\n')
            with self.assertRaisesRegex(ValueError, 'additional or unsupported'):
                lab.handle('unload-xpadneo')
            run.assert_not_called()

    def test_installer_update_is_explicit_and_rejects_symlinks(self):
        destination = self.root / 'installed.py'
        destination.write_bytes(b'old')
        trusted = SimpleNamespace(st_mode=stat.S_IFREG | 0o644, st_uid=0)
        with patch.object(lab, 'trusted_parent'), patch.object(Path, 'lstat', return_value=trusted):
            with self.assertRaisesRegex(ValueError, '--replace'):
                lab.install_file(destination, b'new', False)
            self.assertEqual(destination.read_bytes(), b'old')
            lab.install_file(destination, b'new', True)
            self.assertEqual(destination.read_bytes(), b'new')
            lab.install_file(destination, b'new', False)
        untrusted = SimpleNamespace(st_mode=stat.S_IFLNK | 0o777, st_uid=0)
        with patch.object(lab, 'trusted_parent'), patch.object(Path, 'lstat', return_value=untrusted):
            with self.assertRaisesRegex(ValueError, 'unsafe existing'):
                lab.install_file(destination, b'newer', True)

    def test_socket_peer_uid_checked_before_dispatch(self):
        server, client = socket.socketpair()
        with server, client:
            with patch.object(lab.socket, 'socket', return_value=server), patch.object(lab, 'handle') as handle:
                lab.serve(2**32 - 2)
            self.assertIn(b'unauthorized', client.recv(128))
        handle.assert_not_called()
