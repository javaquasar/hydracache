"""Offline synthetic rootfs controls: no downloads, WSL or service commands."""
import io
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import prepare as p


def image(path, *, version=b'13.1\n', systemd='257.8-1', extra=(), missing=None):
    packages = ('systemd', 'systemd-sysv', 'dbus', 'python3-minimal')
    status = '\n\n'.join('Package: ' + name + '\nStatus: install ok installed\nVersion: ' + systemd
        for name in packages if name != missing).encode()
    files = [('etc/debian_version', version), ('var/lib/dpkg/status', status),
        ('etc/wsl.conf', b'vendor'), ('etc/wsl-distribution.conf', b'OOBE'),
        ('usr/lib/proof', b'unchanged\x00bytes'), *extra]
    with tarfile.open(path, 'w:gz') as archive:
        for name, data in files:
            item = tarfile.TarInfo('./' + name)
            item.mode = 0o751; item.uid = 74; item.gid = 75; item.mtime = 123456
            item.size = len(data)
            archive.addfile(item, io.BytesIO(data))


class Controls(unittest.TestCase):
    def test_masks_exist_before_boot_and_unrelated_contents_and_metadata_survive(self):
        with tempfile.TemporaryDirectory() as directory:
            original = Path(directory) / 'original.gz'; prepared = Path(directory) / 'prepared.tar'
            image(original, extra=[(p.MASKS[0], b'vendor unit')])
            before = p.sha(original)
            result = p.derive(original, prepared)
            self.assertEqual(p.sha(original), before)
            self.assertEqual(result['debian_version'], '13.1')
            with tarfile.open(prepared) as archive:
                self.assertNotIn('etc/wsl-distribution.conf', archive.getnames())
                self.assertEqual(archive.extractfile('etc/wsl.conf').read(), p.WSL_CONF)
                self.assertIn(b'systemd=true', p.WSL_CONF)
                for name in p.MASKS:
                    item = archive.getmember(name)
                    self.assertTrue(item.issym()); self.assertEqual(item.linkname, '/dev/null')
                item = archive.getmember('usr/lib/proof')
                self.assertEqual((item.mode, item.uid, item.gid, item.mtime), (0o751, 74, 75, 123456))
                self.assertEqual(archive.extractfile(item).read(), b'unchanged\x00bytes')

    def test_unsafe_or_duplicate_names_refuse_without_launch(self):
        for name in ('../escape', '/absolute', 'etc/../escape', 'etc//escape', 'etc\\escape'):
            with self.subTest(name=name), self.assertRaises(ValueError): p.member(name)
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / 'source.gz'; dest = Path(directory) / 'out.tar'
            image(source, extra=[('etc/debian_version', b'13.2')])
            with self.assertRaisesRegex(ValueError, 'Duplicate'): p.derive(source, dest)

    def test_wrong_distribution_version_packages_or_manager_never_pass(self):
        for kwargs in (dict(version=b'12.1'), dict(systemd='255.4-1'), dict(systemd='unknown'),
                dict(missing='dbus')):
            with self.subTest(kwargs=kwargs), tempfile.TemporaryDirectory() as directory:
                source = Path(directory) / 'source.gz'; dest = Path(directory) / 'out.tar'
                image(source, **kwargs)
                with self.assertRaises(ValueError): p.derive(source, dest)

    def test_python_is_reported_optional_for_environment_not_assumed_for_harness(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / 'source.gz'; dest = Path(directory) / 'out.tar'
            image(source, missing='python3-minimal')
            self.assertNotIn('python3-minimal', p.derive(source, dest)['packages'])

    def test_first_outputs_cannot_be_overwritten(self):
        with tempfile.TemporaryDirectory() as directory:
            dest = Path(directory) / 'out.tar'; dest.write_bytes(b'first refusal')
            with self.assertRaisesRegex(ValueError, 'Never overwrite'): p.derive(Path('absent'), dest)
            self.assertEqual(dest.read_bytes(), b'first refusal')
            receipt = Path(directory) / 'receipt.json'; p.publish(receipt, dict(first=True))
            with self.assertRaises(FileExistsError): p.publish(receipt, {})
            with patch.object(p, 'OUTPUT', Path(directory)), patch.object(p.urllib.request, 'urlopen') as net:
                with self.assertRaisesRegex(ValueError, 'Never overwrite'): p.fetch()
                net.assert_not_called()

    def test_download_requires_space_before_network_or_directory_creation(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'not-created'
            with patch.object(p, 'OUTPUT', output), patch.object(p.shutil, 'disk_usage') as usage, \
                    patch.object(p.urllib.request, 'urlopen') as net:
                usage.return_value.free = 0
                with self.assertRaisesRegex(ValueError, 'twenty GiB'): p.fetch()
                self.assertFalse(output.exists()); net.assert_not_called()


if __name__ == '__main__': unittest.main(verbosity=2)
