import unittest

import linux_session


class LinuxRunnerControls(unittest.TestCase):
    def test_paths_do_not_escape_owned_root(self):
        for name in ('/etc/config', '../x', 'a/../b', ''):
            self.assertFalse(linux_session.safe_relative(name), name)
        self.assertTrue(linux_session.safe_relative('repo/crates/xtask/tests/control.rs'))

    def test_dependency_versions_and_checksums_cannot_expand(self):
        original = b'[[package]]\nname="a"\nversion="1"\nsource="registry"\nchecksum="old"\n'
        self.assertEqual(linux_session.lock_subset(original, original), 1)
        for changed in (original.replace(b'version="1"', b'version="2"'),
                        original.replace(b'checksum="old"', b'checksum="new"'),
                        original.replace(b'name="a"', b'name="b"')):
            with self.assertRaises(ValueError):
                linux_session.lock_subset(original, changed)
        fixture = b'[[package]]\nname="hc074-historical-blob-linux-fixture"\nversion="0.0.0"\n'
        self.assertEqual(linux_session.lock_subset(original, original + fixture), 2)
        with self.assertRaises(ValueError):
            linux_session.lock_subset(original, fixture.replace(b'0.0.0', b'1.0.0'))


if __name__ == '__main__':
    unittest.main()
