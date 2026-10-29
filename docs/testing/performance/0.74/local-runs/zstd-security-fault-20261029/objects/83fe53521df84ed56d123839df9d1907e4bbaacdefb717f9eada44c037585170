import copy
import importlib.util
import pathlib
import tomllib
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
SCRIPT = pathlib.Path(__file__).with_name("performance_qualification_dry_run_074.py")
MANIFEST = ROOT / "docs/testing/performance/0.74/qualification-manifest.toml"
SPEC = importlib.util.spec_from_file_location("performance_qualification_dry_run_074", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
DRY_RUN = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DRY_RUN)


class QualificationDryRun074Tests(unittest.TestCase):
    def setUp(self) -> None:
        self.manifest = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))

    def test_checked_in_manifest_is_valid_and_blocked(self) -> None:
        self.assertEqual(DRY_RUN.check_manifest(ROOT, self.manifest), [])
        receipt = DRY_RUN.dry_run_receipt(MANIFEST, self.manifest)
        self.assertFalse(receipt["ready_to_execute"])
        self.assertEqual(receipt["executed_commands"], [])
        self.assertIn("authorization", receipt["unresolved"])
        self.assertNotIn("predecessor_annotated_tag", receipt["unresolved"])
        self.assertNotIn("predecessor_confirmation", receipt["unresolved"])

    def test_digest_drift_and_expensive_admission_are_rejected(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["contract_inputs"][0]["sha256"] = "0" * 64
        changed["phases"][2]["expected_before_authorization"] = "run"
        problems = DRY_RUN.check_manifest(ROOT, changed)
        self.assertTrue(any("digest mismatch" in problem for problem in problems))
        self.assertTrue(any("must remain not-run" in problem for problem in problems))

    def test_phase_reordering_and_hidden_blocker_are_rejected(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["phases"].reverse()
        changed["unresolved"]["admitted_host"] = False
        problems = DRY_RUN.check_manifest(ROOT, changed)
        self.assertTrue(any("missing or reordered" in problem for problem in problems))
        self.assertTrue(any("blockers must remain explicit" in problem for problem in problems))


if __name__ == "__main__":
    unittest.main()
