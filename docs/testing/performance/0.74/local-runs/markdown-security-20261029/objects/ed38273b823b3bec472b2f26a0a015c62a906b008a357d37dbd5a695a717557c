"""Fast negative controls for the Linux-only pending custody fault driver."""
import fcntl
import os
from pathlib import Path
import tempfile
import unittest

import original_custody_owner_loss as driver


class CustodyLossDriverTests(unittest.TestCase):
    def test_explicit_refusal(self):
        driver.require(True, "healthy")
        with self.assertRaisesRegex(RuntimeError, "refused"):
            driver.require(False, "refused")

    def test_pidfd_is_live_and_owned(self):
        fd = os.pidfd_open(os.getpid())
        try:
            self.assertEqual(driver.pidfd_pid(fd), os.getpid())
            self.assertFalse(driver.terminal(fd))
            self.assertEqual(driver.parent_pid(os.getpid()), os.getppid())
        finally:
            os.close(fd)

    def test_independent_lock_observer_never_keeps_the_original_lock(self):
        with tempfile.TemporaryDirectory() as name:
            path = Path(name) / "lock"
            path.touch(mode=0o600)
            driver.locked(path, False)
            with path.open("rb") as original:
                fcntl.flock(original, fcntl.LOCK_EX | fcntl.LOCK_NB)
                driver.locked(path, True)
                with self.assertRaisesRegex(RuntimeError, "availability"):
                    driver.locked(path, False)
            driver.locked(path, False)

    def test_snapshot_detects_bytes_metadata_and_refuses_symlinks(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            data = root / "data"
            data.write_bytes(b"one")
            before = driver.snapshot(root)
            self.assertEqual(driver.snapshot(root), before)
            data.write_bytes(b"two")
            self.assertNotEqual(driver.snapshot(root), before)
            (root / "link").symlink_to(data)
            with self.assertRaisesRegex(RuntimeError, "shape"):
                driver.snapshot(root)

    def test_snapshot_refuses_over_budget_data(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            (root / "large").write_bytes(bytes(65537))
            with self.assertRaisesRegex(RuntimeError, "size"):
                driver.snapshot(root)

    def test_command_selects_only_one_ignored_fixture(self):
        command = driver.command(Path("/tmp/test-lib"))
        self.assertEqual(command[:2], ["/tmp/test-lib", "--exact"])
        self.assertEqual(command[2], driver.PREFIX + "original_custody_owner_victim")
        self.assertEqual(command[3:], ["--ignored", "--nocapture", "--test-threads=1"])


if __name__ == "__main__":
    unittest.main()
