import importlib.util
import io
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('steam_rootfs', Path(__file__).parents[1] / 'extract-alpha-steam-rootfs.py')
lab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lab)


class RootfsExtraction(unittest.TestCase):
    def archive(self, root, names):
        path = root / 'fixture.tar.gz'
        with tarfile.open(path, 'w:gz') as target:
            for name in names:
                info = tarfile.TarInfo(name); info.size = 9
                target.addfile(info, io.BytesIO(b'synthetic'))
        return path

    def test_streamed_regular_file_discards_only_completed_owned_cache(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); destination = root / 'private'; destination.mkdir()
            archive = self.archive(root, ['x86_rootfs/bin/fake'])
            with patch.object(lab.os, 'fsync') as sync, patch.object(lab.os, 'posix_fadvise') as advise:
                self.assertEqual(lab.extract(archive, destination), 9)
            sync.assert_called_once()
            advise.assert_called_once()
            self.assertEqual((destination/'x86_rootfs/bin/fake').read_bytes(), b'synthetic')
            with self.assertRaisesRegex(ValueError, 'absent'): lab.extract(archive, destination)

    def test_absolute_guest_symlink_is_confined_to_disposable_guest_root(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); destination = root / 'private'; destination.mkdir()
            archive = root / 'fixture.tar.gz'
            with tarfile.open(archive, 'w:gz') as output:
                target = tarfile.TarInfo('x86_rootfs/usr/fake'); target.size = 9
                output.addfile(target, io.BytesIO(b'synthetic'))
                link = tarfile.TarInfo('x86_rootfs/bin/fake'); link.type = tarfile.SYMTYPE; link.linkname = '/usr/fake'
                output.addfile(link)
            lab.extract(archive, destination)
            self.assertEqual((destination/'x86_rootfs/bin/fake').resolve(), destination/'x86_rootfs/usr/fake')
            self.assertEqual((destination/'x86_rootfs/bin/fake').read_bytes(), b'synthetic')

    def test_foreign_and_traversal_paths_are_rejected_without_writing(self):
        for name in ['foreign/file', '/x86_rootfs/file', 'x86_rootfs/../../foreign']:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                root = Path(directory); destination = root / 'private'; destination.mkdir()
                with self.assertRaises(ValueError): lab.extract(self.archive(root, [name]), destination)
                self.assertEqual(list(destination.iterdir()), [])
