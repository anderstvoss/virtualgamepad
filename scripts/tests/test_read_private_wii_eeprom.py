"""Read-only EEPROM transport tests using synthetic reports exclusively."""
import importlib.util
from pathlib import Path
import struct
import io
import os
import stat
import tempfile
from unittest import mock
import unittest
import zlib

SCRIPT = Path(__file__).resolve().parents[1] / 'read-private-wii-eeprom.py'
SPEC = importlib.util.spec_from_file_location('reader', SCRIPT)
reader = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(reader)


class ReadOnlyEepromTests(unittest.TestCase):
    def test_exact_read_only_request_sequence_and_demo_envelope(self):
        pending = []
        sent = []
        expected = bytes(i % 256 for i in range(reader.EXTENT))

        def send(report):
            sent.append(report)
            self.assertEqual(report[0:3], bytes([0x17, 0, 0]))
            address = int.from_bytes(report[3:5], 'big')
            self.assertEqual(report[5:], bytes([0, 16]))
            pending.extend([b'\x30\0\0', bytes([0x21, 0, 0, 0xf0]) + report[3:5] + expected[address:address + 16]])
            return len(report)

        actual = reader.read_image(send, lambda _timeout: pending.pop(0))
        self.assertEqual(actual, expected)
        self.assertEqual(len(sent), reader.EXTENT // 16)
        self.assertEqual(sent[-1], bytes([0x17, 0, 0, 0x16, 0xf0, 0, 16]))
        for revision, code in reader.REVISIONS.items():
            envelope = reader.envelope(actual, revision)
            self.assertEqual(envelope[:8], reader.MAGIC)
            self.assertEqual(struct.unpack('<HBBIB', envelope[8:17]), (1, 1, code, reader.EXTENT, 0))
            self.assertEqual(envelope[17:-4], expected)
            self.assertEqual(struct.unpack('<I', envelope[-4:])[0], zlib.crc32(envelope[:-4]))

    def test_any_error_stops_without_another_request(self):
        for response in [b'', b'\x21', bytes([0x21, 0, 0, 0xf8]) + bytes(18),
                         bytes([0x21, 0, 0, 0xf0, 0, 16]) + bytes(16),
                         bytes([0x21, 0, 0, 0xe0]) + bytes(18)]:
            sent = []
            with self.assertRaises(RuntimeError):
                reader.read_image(lambda report: sent.append(report) or len(report), lambda _: response)
            self.assertEqual(len(sent), 1)
        sent = []
        with self.assertRaises(RuntimeError):
            reader.read_image(lambda report: sent.append(report) or 0, lambda _: self.fail('receive after short write'))
        self.assertEqual(len(sent), 1)

    def test_owner_kernel_stream_is_complete_private_and_never_overwrites(self):
        for size in [0, reader.EXTENT - 1, reader.EXTENT, reader.EXTENT + 1]:
            with tempfile.TemporaryDirectory() as home:
                stream = io.TextIOWrapper(io.BytesIO(bytes(size)))
                output = io.StringIO()
                with mock.patch.object(reader.Path, 'home', return_value=Path(home)), \
                     mock.patch.object(reader.os, 'geteuid', return_value=1000), \
                     mock.patch('sys.argv', ['reader', '--from-stdin', '--revision', 'early-plus', '--name', 'synthetic']), \
                     mock.patch('sys.stdin', stream), mock.patch('sys.stdout', output), mock.patch('sys.stderr', output):
                    result = reader.main()
                destination = Path(home) / '.local/share/virtualgamepad-private/wii-eeprom/synthetic.wii-eeprom'
                self.assertEqual(result, int(size != reader.EXTENT))
                self.assertEqual(destination.exists(), size == reader.EXTENT)
                if destination.exists():
                    self.assertEqual(destination.read_bytes(), reader.envelope(bytes(size), 'early-plus'))
                    self.assertEqual(stat.S_IMODE(destination.stat().st_mode), 0o600)
                    self.assertEqual(stat.S_IMODE(destination.parent.stat().st_mode), 0o700)
                    self.assertEqual(destination.stat().st_uid, os.getuid())
                    previous = destination.read_bytes()
                    with mock.patch.object(reader.Path, 'home', return_value=Path(home)), \
                         mock.patch.object(reader.os, 'geteuid', return_value=1000), \
                         mock.patch('sys.argv', ['reader', '--from-stdin', '--revision', 'early-plus', '--name', 'synthetic']), \
                         mock.patch('sys.stdin', io.TextIOWrapper(io.BytesIO(bytes([7]) * size))), \
                         mock.patch('sys.stderr', output):
                        self.assertEqual(reader.main(), 1)
                    self.assertEqual(destination.read_bytes(), previous)
                self.assertNotIn(str(home), output.getvalue())

    def test_bounds_never_allow_registers_or_out_of_range_probing(self):
        for address, count in [(-1, 1), (reader.EXTENT, 1), (reader.EXTENT - 1, 2), (0, 0), (0, 17), (0xa40000, 16)]:
            with self.assertRaises(ValueError):
                reader.request(address, count)
        with self.assertRaises(ValueError):
            reader.envelope(bytes(16), 'original')


if __name__ == '__main__':
    unittest.main()
