import io
from pathlib import Path
import stat
import sys
import unittest
import warnings
import zipfile
from unittest.mock import patch

import review as r


def fixture(extra=()):
    stream = io.BytesIO()
    with zipfile.ZipFile(stream, 'w') as archive:
        for name in sorted(r.REQUIRED):
            archive.writestr(r.PREFIX+'/dist/'+name, b'fixture')
        for name, mode, body in extra:
            info = zipfile.ZipInfo(name)
            info.create_system = 3
            info.external_attr = mode << 16
            with warnings.catch_warnings():
                warnings.simplefilter('ignore', UserWarning)
                archive.writestr(info, body)
    return stream.getvalue()


class Assets(unittest.TestCase):
    def test_regular_archive_is_inspected_without_extraction(self):
        with patch.object(zipfile.ZipFile, 'extract', side_effect=AssertionError('extract')), \
                patch.object(zipfile.ZipFile, 'extractall', side_effect=AssertionError('extractall')):
            self.assertEqual(len(r.inventory(fixture())), len(r.REQUIRED))

    def test_symlinks_and_special_files_refuse(self):
        for kind in (stat.S_IFLNK, stat.S_IFIFO, stat.S_IFSOCK, stat.S_IFBLK, stat.S_IFCHR):
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                r.inventory(fixture([(r.PREFIX+'/dist/link', kind | 0o755, b'/outside')]))

    def test_privileged_bits_refuse(self):
        for bits in (0o4000, 0o2000, 0o1000):
            with self.subTest(bits=bits), self.assertRaises(ValueError):
                r.inventory(fixture([(r.PREFIX+'/dist/privileged', stat.S_IFREG | 0o755 | bits, b'x')]))

    def test_noncanonical_cross_platform_paths_refuse(self):
        for name in ('/absolute', '../escape', r.PREFIX+'/../escape',
                     r.PREFIX+'/dist/./a', r.PREFIX+'/dist//a', r.PREFIX+'/dist/a.',
                     r.PREFIX+'/dist/a ', r.PREFIX+'/dist/C:stream', r.PREFIX+'/dist/..\\escape'):
            with self.subTest(name=name), self.assertRaises(ValueError):
                r.inventory(fixture([(name, stat.S_IFREG | 0o644, b'x')]))

    def test_duplicate_and_case_alias_refuse(self):
        for name in (r.PREFIX+'/dist/index.html', r.PREFIX+'/dist/INDEX.HTML'):
            with self.subTest(name=name), self.assertRaises(ValueError):
                r.inventory(fixture([(name, stat.S_IFREG | 0o644, b'x')]))

    def test_file_parent_refuses(self):
        with self.assertRaises(ValueError):
            r.inventory(fixture([(r.PREFIX+'/dist/file', stat.S_IFREG | 0o644, b'x'),
                                 (r.PREFIX+'/dist/file/child', stat.S_IFREG | 0o644, b'y')]))

    def test_missing_required_assets_and_size_limits_refuse(self):
        raw = fixture()
        for field, value in (('MAX_ARCHIVE', 1), ('MAX_ENTRIES', 1), ('MAX_TOTAL', 1), ('MAX_FILE', 1)):
            with self.subTest(field=field), patch.object(r, field, value), self.assertRaises(ValueError):
                r.inventory(raw)
        with patch.object(r, 'REQUIRED', {'not-present'}), self.assertRaises(ValueError):
            r.inventory(raw)

    def test_corrupt_payload_crc_refuses(self):
        raw = fixture().replace(b'fixture', b'changed', 1)
        with self.assertRaises(zipfile.BadZipFile):
            r.inventory(raw)

    def test_unknown_crate_refuses_before_archive_parsing(self):
        with patch.object(r.tarfile, 'open', side_effect=AssertionError('unverified tar')), self.assertRaises(ValueError):
            r.review(b'unknown')


if __name__ == '__main__':
    unittest.main()
