import importlib.util
import os
from pathlib import Path
import stat
import tempfile
import unittest


MODULE_PATH = Path(__file__).with_name("performance_long_run_host_capability_074.py")
SPEC = importlib.util.spec_from_file_location("performance_long_run_host_capability_074", MODULE_PATH)
assert SPEC and SPEC.loader
capability = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(capability)


class HostCapabilityTests(unittest.TestCase):
    def test_path_metadata_distinguishes_missing_file_directory_and_socket(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            missing = capability.path_metadata(str(root / "missing"))
            self.assertEqual(missing["status"], "missing")
            self.assertEqual(capability.path_metadata(str(root))["kind"], "directory")
            regular = root / "file"
            regular.write_text("x", encoding="utf-8")
            self.assertEqual(capability.path_metadata(str(regular))["kind"], "regular")

    @unittest.skipUnless(hasattr(os, "mkfifo"), "Unix file kinds are unavailable")
    def test_path_metadata_does_not_treat_non_regular_file_as_ready(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fifo = Path(directory) / "fifo"
            os.mkfifo(fifo)
            metadata = capability.path_metadata(str(fifo))
            self.assertEqual(metadata["status"], "present")
            self.assertEqual(metadata["kind"], "other")
            self.assertTrue(stat.S_ISFIFO(os.lstat(fifo).st_mode))

    def test_missing_command_is_explicit(self) -> None:
        result = capability.command("hydracache-command-that-does-not-exist-074")
        self.assertFalse(result["available"])
        self.assertIsNone(result["exit_code"])


if __name__ == "__main__":
    unittest.main()
