import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import launch_preflight as subject
from owned_process_observer import Refusal


def fixture():
    source = {"Cargo.toml": "pinned"}
    receipt = dict(build_source_head="fixed", build_source_count=1,
                   build_source_fingerprint=subject.sha(json.dumps(source, sort_keys=True, separators=(",", ":")).encode()),
                   cwd=r"C:\fixture", images=dict(test=dict(path=r"C:\fixture\test.exe")))
    before = dict(argv=subject.BUILD_ARGV, head="fixed", source=source)
    after = dict(exit=0, outer_refusal=None, source_unchanged=True, head_unchanged=True, source_after=source.copy())
    artifact = dict(reason="compiler-artifact", manifest_path=r"C:\fixture\crates\xtask\Cargo.toml",
                    target=dict(name="performance_contract_074", kind=["test"], src_path=r"C:\fixture\crates\xtask\tests\performance_contract_074.rs"),
                    features=[], profile=dict(opt_level="0", debuginfo=2, debug_assertions=True, overflow_checks=True, test=True),
                    executable=r"C:\fixture\test.exe")
    return receipt, before, after, [artifact, dict(reason="build-finished", success=True)]


def check(values):
    receipt, before, after, rows = values
    return subject.check_build(receipt, before, after, "\n".join(json.dumps(row) for row in rows).encode())


class PreflightTests(unittest.TestCase):
    def test_valid_no_run_receipt_and_no_dispatch(self):
        self.assertEqual(check(fixture()), {"Cargo.toml": "pinned"})
        self.assertNotIn("subprocess", subject.__dict__)

    def test_build_argv_head_and_first_result_not_replaced(self):
        for mutation in (dict(argv=["cargo", "test"]), dict(head="different")):
            values = fixture()
            values[1].update(mutation)
            with self.assertRaises(Refusal):
                check(values)
        for mutation in (dict(exit=1), dict(outer_refusal="timeout"), dict(source_unchanged=False), dict(head_unchanged=False), dict(source_after={})):
            values = fixture()
            values[2].update(mutation)
            with self.subTest(mutation=mutation), self.assertRaises(Refusal):
                check(values)

    def test_build_inventory_not_current_response_derived(self):
        for key, value in (("build_source_count", 2), ("build_source_fingerprint", "wrong")):
            values = fixture()
            values[0][key] = value
            with self.assertRaisesRegex(Refusal, "inventory drift"):
                check(values)

    def test_feature_profile_kind_source_and_image_drifts(self):
        for mutate in (
            lambda a: a.update(features=["extra"]),
            lambda a: a["profile"].update(opt_level="3"),
            lambda a: a["target"].update(kind=["bin"]),
            lambda a: a["target"].update(src_path=r"C:\foreign\test.rs"),
            lambda a: a.update(manifest_path=r"C:\foreign\Cargo.toml"),
            lambda a: a.update(executable=r"C:\foreign\test.exe")):
            values = fixture()
            mutate(values[3][0])
            with self.assertRaises(Refusal):
                check(values)

    def test_missing_duplicate_or_unfinished_build_refuses(self):
        for rows in ([], [dict(reason="build-finished", success=False)],
                     [fixture()[3][0], fixture()[3][0], fixture()[3][-1]],
                     list(reversed(fixture()[3])), [fixture()[3][0]]):
            values = fixture()
            values = (*values[:3], rows)
            with self.subTest(rows=rows), self.assertRaises(Refusal):
                check(values)

    def test_duplicate_json_keys_refuse(self):
        with self.assertRaisesRegex(Refusal, "duplicate JSON"):
            subject.decode(b'{"dispatch":false,"dispatch":true}')

    def test_exact_file_pin_and_size_fail_closed(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "file"
            path.write_bytes(b"first")
            digest = subject.sha(b"first")
            self.assertEqual(subject.pinned(path, digest), b"first")
            path.write_bytes(b"other")
            with self.assertRaisesRegex(Refusal, "bytes drift"):
                subject.pinned(path, digest)
            with self.assertRaisesRegex(Refusal, "oversized"):
                subject.pinned(path, digest, 1)

    def test_source_selection_and_safe_paths(self):
        source = {name: "fixed" for name in ("Cargo.toml", "crates/a/data", ".cargo/config.toml", "tools/a/b.rs", "tools/a/b.toml", "tools/new.py", "docs/a.md")}
        self.assertEqual(len(subject.source_inputs(source)), 5)
        for name in ("../escape", "/absolute", r"C:\escape", r"a\escape"):
            with self.subTest(name=name), self.assertRaises(Refusal):
                subject.source_path(Path("fixture"), name)

    def test_preflight_first_refusal_before_dispatch_and_no_overwrite(self):
        with tempfile.TemporaryDirectory() as temp:
            out = Path(temp) / "first"
            with patch.object(subject, "evaluate", side_effect=Refusal("receipt drift")):
                self.assertFalse(subject.main(out))
            result = json.loads((out / "result.json").read_text())
            self.assertIn("receipt drift", result["first_refusal"])
            self.assertFalse(result["dispatch_authorized"])
            with self.assertRaises(FileExistsError):
                subject.main(out)


if __name__ == "__main__":
    unittest.main()
