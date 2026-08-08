#!/usr/bin/env python3

import importlib.util
from pathlib import Path
import unittest


SCRIPT = Path(__file__).with_name("reference-memory-only-window.py")
SPEC = importlib.util.spec_from_file_location("memory_only_window", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
guard = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(guard)


class MemoryOnlyWindowTests(unittest.TestCase):
    def test_cpu_list_is_strict(self) -> None:
        self.assertEqual(guard.parse_cpu_list("1-4"), {1, 2, 3, 4})
        with self.assertRaises(guard.GuardError):
            guard.parse_cpu_list("4-1")

    def test_counter_delta_detects_activity_and_mapping_drift(self) -> None:
        self.assertEqual(guard.nonzero_delta({"nvme0": [1, 2]}, {"nvme0": [1, 2]}), [])
        self.assertEqual(
            guard.nonzero_delta({"nvme0": [1, 2]}, {"nvme0": [1, 3]}),
            ["nvme0[1]=+1"],
        )
        self.assertEqual(
            guard.nonzero_delta({"nvme0": [1]}, {"nvme1": [1]}),
            ["nvme0=mapping-changed", "nvme1=mapping-changed"],
        )

    def test_diskstats_selects_only_nvme_namespaces(self) -> None:
        import tempfile

        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "diskstats"
            path.write_text(
                "259 0 nvme0n1 1 0 2 0 3 0 4 0 0 0 0 0\n"
                "259 1 nvme0n1p1 8 0 9 0 10 0 11 0 0 0 0 0\n"
                "8 0 sda 1 0 2 0 3 0 4 0 0 0 0 0\n",
                encoding="utf-8",
            )
            self.assertEqual(set(guard.read_diskstats(path)), {"nvme0n1"})


if __name__ == "__main__":
    unittest.main()
