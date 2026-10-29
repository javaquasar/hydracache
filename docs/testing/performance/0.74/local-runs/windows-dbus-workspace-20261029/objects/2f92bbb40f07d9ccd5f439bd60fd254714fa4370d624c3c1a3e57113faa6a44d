"""Offline tests and falsifying controls against immutable refused trace inputs."""

import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


spec = importlib.util.spec_from_file_location("attribute_partial", Path(__file__).with_name("attribute_partial.py"))
subject = importlib.util.module_from_spec(spec)
spec.loader.exec_module(subject)
ROOT = Path(__file__).resolve().parents[2]
PACKET = ROOT / "docs/testing/performance/0.74/local-runs" / subject.PACKET


class AttributionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.metadata = subject.load(PACKET / "metadata.json")
        cls.data = {}
        for lane in ("194", "188"):
            path = PACKET / f"{lane}-Profile-0.log"
            cls.data[lane] = (subject.rows(path, "HC074-HELD-PHASE "),
                              subject.rows(path, "HC074-HELD-LIVE-ORIGINAL ")[0],
                              subject.load(PACKET / f"{lane}-Profile-0.exit.json"))

    def lane(self, lane="194", mutate=lambda phases, original, receipt: None):
        phases, original, receipt = copy.deepcopy(self.data[lane])
        mutate(phases, original, receipt)
        return subject.lane_facts(phases, original, receipt, lane, self.metadata)

    def test_both_partial_prefixes_keep_original_refusal(self):
        for lane, count, phase, age in (("194",6,"owned-root-preparation",10175057064),
                                        ("188",7,"enrollment",10006497560)):
            result = self.lane(lane)
            self.assertEqual(len(result["phases"]), count)
            self.assertEqual(result["phases"][-1]["phase"], phase)
            self.assertEqual(result["original_refusal"]["age_ns"], age)
            self.assertEqual(result["dominant_phase"], "outputs")
            self.assertEqual(result["output_dominant_step"], "ReservedLive")
            self.assertFalse(result["complete_profile_cohort"])
            self.assertFalse(result["healthy_gate"])

    def test_graph_and_round_remainders_reconcile(self):
        output = self.lane()["phases"][2]
        self.assertEqual(output["wall_ns"], output["round_step_wall_ns"] + output["wall_outside_round_steps_ns"])
        for step in output["steps"]:
            self.assertEqual(step["wall_ns"], step["kernel_outside_children_total_ns"] +
                             step["wall_outside_instrumented_kernel_ns"])
        self.assertEqual(output["steps"][1]["authentication_calls"], 736)
        self.assertEqual(output["authentication_calls"], 782)

    def test_missing_reordered_duplicate_phase_refused(self):
        mutations = (lambda p,o,r:p.pop(1), lambda p,o,r:p.reverse(), lambda p,o,r:p.append(p[0]))
        for mutate in mutations:
            with self.subTest(mutate=mutate), self.assertRaises(ValueError):
                self.lane(mutate=mutate)

    def test_returned_failure_or_original_exit_erased_refused(self):
        for mutate in (lambda p,o,r:p[-1].update(returned=True),
                       lambda p,o,r:o.update(returned=True), lambda p,o,r:r.update(exit=0)):
            with self.subTest(mutate=mutate), self.assertRaises(ValueError):
                self.lane(mutate=mutate)

    def test_missing_or_reordered_step_refused(self):
        for mutate in (lambda p,o,r:p[2]["round_steps"].pop(),
                       lambda p,o,r:p[2]["round_steps"].reverse()):
            with self.subTest(mutate=mutate), self.assertRaises(ValueError):
                self.lane(mutate=mutate)

    def test_kernel_node_edge_root_mutations_refused(self):
        for mutate in (lambda p,o,r:p[2]["round_steps"][1]["kernel"][0].update(calls=1),
                       lambda p,o,r:p[2]["round_steps"][1]["edges"][0].update(calls=1),
                       lambda p,o,r:p[2]["round_steps"][1]["roots"].__setitem__(0,1),
                       lambda p,o,r:p[2]["round_steps"][1]["edges"][0].update(inclusive_ns=1)):
            with self.subTest(mutate=mutate), self.assertRaises(ValueError):
                self.lane(mutate=mutate)

    def test_duplicate_or_out_of_range_edge_refused(self):
        for mutate in (lambda p,o,r:p[2]["round_steps"][1]["edges"].append(p[2]["round_steps"][1]["edges"][0]),
                       lambda p,o,r:p[2]["round_steps"][1]["edges"][0].update(child=10)):
            with self.subTest(mutate=mutate), self.assertRaises(ValueError):
                self.lane(mutate=mutate)

    def test_round_count_completion_and_continuation_mutations_refused(self):
        for mutate in (lambda p,o,r:p[2]["round_steps"][1].update(calls=45),
                       lambda p,o,r:p[-1]["round_steps"][3].update(completed=1),
                       lambda p,o,r:p[-1]["round_steps"][4].update(calls=1)):
            with self.subTest(mutate=mutate), self.assertRaises(ValueError):
                self.lane(mutate=mutate)

    def test_parent_count_auth_and_output_round_signature_mutations_refused(self):
        for mutate in (lambda p,o,r:p[2]["kernel"][0].update(calls=1),
                       lambda p,o,r:p[2].update(authentication_calls=781),
                       lambda p,o,r:p[2]["kernel"][0].update(calls=8465)):
            with self.subTest(mutate=mutate), self.assertRaises(ValueError):
                self.lane(mutate=mutate)

    def test_negative_fractional_bool_and_impossible_wall_refused(self):
        for value in (-1, 1.0, True):
            with self.subTest(value=value), self.assertRaises(ValueError):
                self.lane(mutate=lambda p,o,r:p[2].update(wall_ns=value))
        with self.assertRaisesRegex(ValueError, "Step wall"):
            self.lane(mutate=lambda p,o,r:p[2].update(wall_ns=1))

    def test_scope_flags_cannot_be_relaxed(self):
        for key in ("deadline_changed","guard_clock_added","server","product","includes_child_cpu","healthy_gate"):
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.lane(mutate=lambda p,o,r:p[2].update({key:True}))

    def test_refusal_boot_age_and_site_mutations_refused(self):
        for change in (dict(now_boot="other"),dict(diagnostic_reason="healthy"),dict(age_ns=1),dict(site="other")):
            with self.subTest(change=change), self.assertRaises(ValueError):
                self.lane(mutate=lambda p,o,r:o["causal"]["first_refusal"].update(change))

    def test_optional_cpu_is_not_inferred_or_clamped_to_wall(self):
        result = self.lane(mutate=lambda p,o,r:p[0].update(process_cpu_ns=None))
        self.assertIsNone(result["observed_parent_cpu_total_ns"])
        result = self.lane(mutate=lambda p,o,r:p[0].update(process_cpu_ns=10**12))
        self.assertGreater(result["phases"][0]["parent_process_cpu_ns"],result["phases"][0]["wall_ns"])

    def test_full_analysis_keeps_interpretation_and_gates_false(self):
        result = subject.analyze(PACKET)
        self.assertEqual(result["new_profile_invocations"], 0)
        self.assertEqual(result["input_members_verified"], 55)
        self.assertFalse(result["native_gate_open"])
        self.assertFalse(result["complete_profile_cohort"])
        self.assertFalse(result["interpretation"]["phase_sum_is_issued_age"])
        self.assertFalse(result["interpretation"]["kernel_outside_is_cpu"])
        self.assertFalse(result["interpretation"]["compiler_comparison"])

    def test_inventory_pin_and_member_bytes_are_checked(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(ValueError,"inventory pin"):
                (Path(directory)/"inventory.json").write_text("{}")
                subject.verify_inventory(Path(directory))
        original = subject.sha
        with patch.object(subject,"sha",side_effect=lambda p:"changed" if Path(p).name=="metadata.json" else original(p)):
            with self.assertRaisesRegex(ValueError,"member changed"):
                subject.verify_inventory(PACKET)

    def test_derived_output_is_exclusive_and_reconstructed_on_verify(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)/"facts.json"
            subject.publish(path, dict(observation=74))
            with self.assertRaises(FileExistsError):
                subject.publish(path, dict(observation=75))
            subject.publish(path, dict(observation=74), verify=True)
            with self.assertRaisesRegex(ValueError,"Derived facts changed"):
                subject.publish(path, dict(observation=75), verify=True)

    def test_observer_names_follow_current_unchanged_source(self):
        sources = (("src/diagnostic_validation_frame.rs","enum ValidationFrameStep",subject.STEPS),
                   ("src/diagnostic_kernel_walk_trace.rs","enum Kind",subject.KERNEL),
                   ("src/diagnostic_live_walk_trace.rs","enum Kind",subject.LIVE))
        for path, marker, expected in sources:
            raw = (ROOT/"tools/long-run-supervisor-074"/path).read_text()
            body = raw.split(marker,1)[1].split("{",1)[1].split("}",1)[0]
            actual = tuple(word.strip() for word in body.split(",") if word.strip())
            self.assertEqual(actual, expected)

    def test_retained_recipe_cli_reads_only_explicit_pinned_input(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)/"facts.json"
            args = ["attribute_partial.py","--input",str(PACKET),"--output",str(path)]
            with patch("sys.argv",args):
                subject.main()
            with patch("sys.argv",args+["--verify"]):
                subject.main()
            self.assertFalse(subject.load(path)["native_gate_open"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
