from pathlib import Path
import tempfile
import unittest
from unittest import mock

import oracle


class FixtureScope(unittest.TestCase):
    def test_fault_profiles_leave_the_full_oracle_unchanged(self):
        full = [('disabled', False), ('default', False), ('default', True)]
        for replaced in (False, True):
            self.assertEqual(oracle.profiles(False, replaced), full)
        self.assertEqual(oracle.profiles(True, False), [('disabled', False)])
        self.assertEqual(oracle.profiles(True, True), [('default', False)])

    def test_invalid_mode_refuses_before_effects(self):
        with mock.patch.object(oracle.os.environ, 'clear') as clear, \
             mock.patch.object(oracle.subprocess, 'run') as run:
            with self.assertRaises(ValueError):
                oracle.prepare('unused', fault_controls='full')
            clear.assert_not_called()
            run.assert_not_called()

    def test_nonempty_or_file_root_refuses_before_environment_or_git_effects(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'keep'
            path.write_bytes(b'not a fixture')
            for root in (Path(directory), path):
                with mock.patch.object(oracle.subprocess, 'run') as run, \
                     mock.patch.object(oracle.os.environ, 'clear') as clear:
                    for fault_controls in (False, True):
                        with self.assertRaises(ValueError):
                            oracle.prepare(root, fault_controls=fault_controls)
                    run.assert_not_called()
                    clear.assert_not_called()
            self.assertEqual(path.read_bytes(), b'not a fixture')

    def test_missing_root_refuses_without_creating_it(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'absent'
            with mock.patch.object(oracle.subprocess, 'run') as run:
                with self.assertRaises(FileNotFoundError):
                    oracle.prepare(root)
                run.assert_not_called()
            self.assertFalse(root.exists())


if __name__ == '__main__':
    unittest.main()
