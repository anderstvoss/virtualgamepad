"""No privileged I/O: port admission and readiness use fake inventory/pipes."""
import importlib.util
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import MagicMock, patch
from types import SimpleNamespace

spec = importlib.util.spec_from_file_location('usb_audio_lab', Path(__file__).parents[1] / 'run-usb-audio-lab.py')
lab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lab)
HEADER = 'hub port sta spd dev sockfd local_busid\n'
FREE = 'hs 0000 004 000 00000000 000000 0-0\n'


class Admission(unittest.TestCase):
    def test_only_explicit_free_high_speed_port(self):
        self.assertEqual(lab.available_port(HEADER + FREE, 0), 0)
        for inventory, port in [
            (HEADER + FREE, 1),
            (HEADER + FREE.replace('hs', 'ss'), 0),
            (HEADER + FREE.replace('004', '006'), 0),
            (HEADER + FREE.replace('00000000', '00000001'), 0),
            (HEADER + FREE.replace('000000 ', '000001 '), 0),
        ]:
            with self.subTest(inventory=inventory, port=port), self.assertRaises(ValueError):
                lab.available_port(inventory, port)

    def test_malformed_and_duplicate_inventory_fail_closed(self):
        for inventory in ['', FREE, HEADER + FREE + FREE, HEADER + 'hs 0\n',
                          HEADER + FREE.replace('0000 ', '-1 ', 1)]:
            with self.subTest(inventory=inventory), self.assertRaises(ValueError):
                lab.available_port(inventory, 0)

    def test_worker_readiness_is_exact_and_bounded(self):
        for output, success in [('READY\n', True), ('BAD\n', False), ('X'*65, False), ('', False)]:
            process = subprocess.Popen([sys.executable, '-c', 'import sys; sys.stdout.write(sys.argv[1])', output], stdout=subprocess.PIPE)
            try:
                if success:
                    lab.wait_ready(process)
                else:
                    with self.assertRaises(RuntimeError):
                        lab.wait_ready(process)
            finally:
                process.wait(timeout=2)
                process.stdout.close()

    def test_worker_readiness_timeout(self):
        process = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(2)'], stdout=subprocess.PIPE)
        try:
            with self.assertRaises(TimeoutError):
                lab.wait_ready(process, timeout=0.01)
        finally:
            process.terminate()
            process.wait(timeout=2)
            process.stdout.close()

    def test_root_worker_is_rejected_before_exec(self):
        from unittest.mock import patch
        with patch.object(lab.os, 'getuid', return_value=0):
            with self.assertRaises(RuntimeError):
                lab.child_limits(1)


class Lifecycle(unittest.TestCase):
    def exercise(self, failure=None):
        worker = MagicMock()
        worker.resolve.return_value = worker
        worker.is_file.return_value = True
        args = SimpleNamespace(worker=worker, profile='dualsense', port=0, seconds=1)
        kernel, peer = MagicMock(), MagicMock()
        kernel.fileno.return_value = 42
        process = MagicMock()
        process.poll.return_value = None
        process.wait.return_value = 0
        inventory = MagicMock()
        (inventory / 'status').read_text.return_value = HEADER + FREE
        with patch.object(lab, 'VHCI', inventory), \
             patch.object(lab.os, 'geteuid', return_value=0), \
             patch.dict(lab.os.environ, {'SUDO_UID': '1234'}), \
             patch.object(lab.pwd, 'getpwuid', return_value=SimpleNamespace(pw_gid=1234)), \
             patch.object(lab.os, 'open', return_value=8), \
             patch.object(lab.os, 'fstat', return_value=SimpleNamespace(st_mode=lab.stat.S_IFREG)), \
             patch.object(lab.os, 'close') as close, \
             patch.object(lab.os, 'write', side_effect=lambda _, data: len(data)) as write, \
             patch.object(lab.socket, 'socketpair', return_value=(kernel, peer)), \
             patch.object(lab.subprocess, 'Popen', return_value=process) as spawn, \
             patch.object(lab, 'wait_ready', side_effect=failure), \
             patch.object(lab, 'supervise', return_value=0), \
             patch('builtins.print'):
            if failure:
                with self.assertRaises(RuntimeError):
                    lab.run(args)
                write.assert_not_called()
            else:
                lab.run(args)
                self.assertEqual(write.call_count, 1)
                self.assertTrue(write.call_args.args[1].startswith(b'0 42 '))
            kernel.shutdown.assert_called_once_with(lab.socket.SHUT_RDWR)
            process.terminate.assert_called_once()
            process.stdout.close.assert_called_once()
            close.assert_called_once_with(8)
            kwargs = spawn.call_args.kwargs
            self.assertEqual((kwargs['user'], kwargs['group'], kwargs['extra_groups']), (1234, 1234, []))
            self.assertTrue(kwargs['close_fds'])
            self.assertEqual(kwargs['env'], {'PATH': '/usr/bin:/bin', 'LANG': 'C'})
            self.assertIs(kwargs['stdin'], peer)

    def test_success_uses_owned_socket_cleanup_and_unprivileged_worker(self):
        self.exercise()

    def test_readiness_failure_rolls_back_without_attach(self):
        self.exercise(RuntimeError('not ready'))


if __name__ == '__main__':
    unittest.main()
