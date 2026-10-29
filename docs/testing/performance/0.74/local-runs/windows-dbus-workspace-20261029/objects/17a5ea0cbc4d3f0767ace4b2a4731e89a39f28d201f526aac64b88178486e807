import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("summarize-memory-diagnostic.py")
SPEC = importlib.util.spec_from_file_location("summarize_memory_diagnostic", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class SummarizeMemoryDiagnosticTest(unittest.TestCase):
    def test_builds_compact_non_promotable_complete_summary(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "reproduction-command.txt").write_text(
                "ship_evidence_eligible=false\nsource_commit=abc\nsource_tree_clean=true\n"
                "diagnostic_environment=github-hosted\nkernel=Linux test\nonline_cpus=4\n"
                "redis_image=redis@sha256:test\ntargets=hydra redis\n",
                encoding="utf-8",
            )
            (root / "leak-status.tsv").write_text(
                "experiment\ttarget\tpattern\tstatus\n01\thydra\tfixed\tcomplete\n",
                encoding="utf-8",
            )
            (root / "leak-index.json").write_text(
                json.dumps([[{"experiment": "01", "target": "hydra"}, {"samples": 3}, {"vmrss_bytes": {"last": 42}}]]),
                encoding="utf-8",
            )
            summary = MODULE.build_summary(root)
            self.assertTrue(summary["complete"])
            self.assertTrue(summary["non_promotable"])
            self.assertEqual(summary["cases"][0]["metrics"]["vmrss_bytes"]["last"], 42)

    def test_rejects_promotable_input(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "reproduction-command.txt").write_text("ship_evidence_eligible=true\n", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "ship_evidence_eligible=false"):
                MODULE.build_summary(root)


if __name__ == "__main__":
    unittest.main()
