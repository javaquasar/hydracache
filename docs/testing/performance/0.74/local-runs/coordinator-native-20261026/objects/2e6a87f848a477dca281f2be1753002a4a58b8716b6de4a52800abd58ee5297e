import copy
import unittest

from launch_preflight import ARGV
from owned_process_observer import Refusal
from target_result_join import ExpectedSource, join_target, source_digest


NAMES = frozenset(f"module::case_{i}" for i in range(321))


def fixture(failed=False, partial=False):
    source = {"crates/fixed.rs": "a" * 64}
    expected = ExpectedSource("b" * 40, source_digest(source))
    intent = dict(argv=ARGV.copy(), bound_seconds=780, retry=False, source=source, head=expected.head)
    result = dict(exit=101 if failed else 0, outer_refusal=None, source_unchanged=True,
                  head_unchanged=True, source_after=source.copy())
    names = sorted(NAMES)
    lines = ["running 321 tests"]
    lines += [f"test {name} ... {'FAILED' if failed and i == 0 else 'ok'}" for i, name in enumerate(names)]
    lines += [f"test result: {'FAILED' if failed else 'ok'}. {320 if failed else 321} passed; {1 if failed else 0} failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s"]
    if partial:
        lines = lines[:12]
        result.update(exit=1, outer_refusal="outer-timeout")
    stderr = "    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.01s\n     Running tests\\performance_contract_074.rs (target\\debug\\fixed.exe)\n"
    return intent, result, "\n".join(lines), stderr, expected, NAMES


class ResultJoinTests(unittest.TestCase):
    def test_complete_target_not_package_or_telemetry(self):
        result = join_target(*fixture())
        self.assertTrue(result["selected_target_pass"])
        self.assertFalse(result["full_package_pass"])
        self.assertTrue(result["live_attempt_binding_required"])

    def test_failed_and_partial_target_preserve_own_verdict(self):
        for kwargs in (dict(failed=True), dict(partial=True)):
            with self.subTest(kwargs=kwargs):
                result = join_target(*fixture(**kwargs))
                self.assertFalse(result["selected_target_pass"])
        self.assertFalse(join_target(*fixture(partial=True))["complete"])

    def test_reserved_identity_cannot_be_taken_from_changed_response(self):
        values = list(fixture())
        values[0]["source"]["crates/fixed.rs"] = "c" * 64
        values[1]["source_after"] = values[0]["source"].copy()
        with self.assertRaisesRegex(Refusal, "caller-reserved"):
            join_target(*values)

    def test_argv_window_retry_and_source_flags_refuse(self):
        for mutation in (dict(argv=ARGV + ["filter"]), dict(bound_seconds=781), dict(retry=True), dict(head="d" * 40)):
            values = list(fixture())
            values[0].update(mutation)
            with self.subTest(mutation=mutation), self.assertRaises(Refusal):
                join_target(*values)
        for mutation in (dict(source_unchanged=False), dict(head_unchanged=False), dict(source_after={})):
            values = list(fixture())
            values[1].update(mutation)
            with self.assertRaises(Refusal):
                join_target(*values)

    def test_unknown_duplicate_missing_case_and_filtered_scope_refuse(self):
        for edit in (
            lambda s: s.replace("module::case_0", "foreign::case_0", 1),
            lambda s: s.replace("module::case_1 ...", "module::case_0 ...", 1),
            lambda s: s.replace("test module::case_0 ... ok\n", ""),
            lambda s: s.replace("0 filtered out", "1 filtered out"),
            lambda s: s.replace("running 321 tests", "running 320 tests"),
            lambda s: s.replace("case_0 ... ok", "case_0 ... ignored").replace("321 passed; 0 failed; 0 ignored", "320 passed; 0 failed; 1 ignored")):
            values = list(fixture())
            values[2] = edit(values[2])
            with self.assertRaises((Refusal, ValueError)):
                join_target(*values)

    def test_different_catalog_and_invalid_identity_refuse(self):
        values = list(fixture())
        values[-1] = NAMES - {"module::case_0"}
        with self.assertRaises(Refusal):
            join_target(*values)
        for head, digest in (("x", "a" * 64), ("a" * 40, "bad")):
            with self.assertRaises(Refusal):
                ExpectedSource(head, digest)


if __name__ == "__main__":
    unittest.main()
