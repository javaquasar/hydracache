import json
import ast
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import Mock

from bound_target_observation import BoundTargetObservation
from owned_process_observer import Refusal
from test_target_result_join import fixture


def composed(failed=False, partial=False, lost_ack=False):
    intent, result, stdout, stderr, expected, catalog = fixture(failed=failed, partial=partial)
    owner = object()
    observation = dict(observations_complete=not lost_ack, first_refusal="lost ACK" if lost_ack else None)
    custody = SimpleNamespace(owner=owner, observation=observation, exit_proof=dict(owner_returncode=int(failed or partial)),
                              release_attempted=False, released=False, custody_refusal=None)
    def finalize():
        custody.release_attempted = custody.released = True
    custody.finalize_observation = Mock(side_effect=finalize)
    custody.result = Mock(return_value=observation)
    outputs = SimpleNamespace(attempt_id="fixed-attempt", reservation=Path("reserved"), owner=owner, custody=custody,
                              attach_attempted=True, first_refusal=None, expected=expected)
    outputs.seal = Mock(return_value=({"intent.json": json.dumps(intent).encode(), "result.json": json.dumps(result).encode(),
                                      "stdout.log": stdout.encode(), "stderr.log": stderr.encode()}, {"bound": True}))
    dispatch = SimpleNamespace(attempt_id=outputs.attempt_id, directory=outputs.reservation, owner=owner, custody=custody,
                               acknowledged=not lost_ack, first_refusal="lost ACK" if lost_ack else None,
                               result=Mock(return_value=dict(acknowledged=not lost_ack)))
    return BoundTargetObservation(dispatch, outputs, catalog)


class CompositionTests(unittest.TestCase):
    def test_model_complete_join_is_not_package_execution_or_real_cargo_proof(self):
        subject = composed()
        result = subject.finish()
        self.assertTrue(result["diagnostic_pass"])
        self.assertTrue(result["target"]["selected_target_pass"])
        self.assertFalse(result["target"]["live_attempt_binding_required"])
        for field in ("full_package_pass", "actual_cargo_shape_proven", "all_writers_proven_stopped",
                      "execution_completion_authorized", "admission"):
            self.assertFalse(result[field])
        subject.outputs.seal.assert_called_once_with(subject.dispatch.custody)
        subject.dispatch.custody.finalize_observation.assert_called_once()
        with self.assertRaises(Refusal):
            subject.finish()

    def test_failed_partial_and_lost_ack_never_become_diagnostic_pass(self):
        for flags in (dict(failed=True), dict(partial=True), dict(lost_ack=True)):
            result = composed(**flags).finish()
            self.assertFalse(result["diagnostic_pass"])
        result = composed(lost_ack=True).finish()
        self.assertTrue(result["target"]["selected_target_pass"])
        self.assertEqual(result["dispatch"]["acknowledged"], False)

    def test_foreign_slot_owner_custody_or_unbound_outputs_refuse(self):
        for name, value in (("attempt_id", "other"), ("reservation", Path("other")), ("owner", object()),
                            ("custody", object()), ("attach_attempted", False), ("first_refusal", "first")):
            subject = composed()
            setattr(subject.outputs, name, value)
            with self.assertRaises(Refusal):
                BoundTargetObservation(subject.dispatch, subject.outputs, subject.catalog)

    def test_unfrozen_or_live_does_not_read_files(self):
        for field in ("observation", "exit_proof"):
            subject = composed()
            setattr(subject.dispatch.custody, field, None)
            with self.assertRaises(Refusal):
                subject.finish()
            subject.outputs.seal.assert_not_called()

    def test_exit_or_source_conflict_remains_first_refusal_without_retry(self):
        for fault in ("exit", "source", "duplicate-json"):
            subject = composed()
            if fault == "exit":
                subject.dispatch.custody.exit_proof["owner_returncode"] = 1
            else:
                raw, _ = subject.outputs.seal.return_value
                if fault == "source":
                    intent = json.loads(raw["intent.json"])
                    intent["head"] = "d" * 40
                    raw["intent.json"] = json.dumps(intent).encode()
                else:
                    raw["result.json"] = b'{"exit":0,"exit":1}'
            with self.assertRaises(Refusal):
                subject.finish()
            first = subject.first_refusal
            with self.assertRaises(Refusal):
                subject.finish()
            self.assertEqual(subject.first_refusal, first)

    def test_output_or_cleanup_failure_never_passes(self):
        for fault in ("output", "cleanup"):
            subject = composed()
            if fault == "output":
                subject.outputs.seal.side_effect = Refusal("original output failure")
            else:
                subject.dispatch.custody.finalize_observation.side_effect = Refusal("cleanup failure")
            with self.assertRaises(Refusal):
                subject.finish()

    def test_cleanup_attempt_without_release_refuses(self):
        subject = composed()
        subject.dispatch.custody.release_attempted = True
        with self.assertRaisesRegex(Refusal, "cleanup not proven"):
            subject.finish()
        subject.dispatch.custody.finalize_observation.assert_not_called()

    def test_composition_has_no_process_launch_or_external_log_reader(self):
        import bound_target_observation
        import attempt_output_binding
        for module in (bound_target_observation, attempt_output_binding):
            tree = ast.parse(Path(module.__file__).read_text())
            imports = {alias.name for node in ast.walk(tree) if isinstance(node, ast.Import) for alias in node.names}
            self.assertNotIn("subprocess", imports)
        source = Path(bound_target_observation.__file__).read_text()
        for forbidden in ("read_bytes(", "read_text(", "open(", "kill(", "terminate(", "Popen("):
            self.assertNotIn(forbidden, source)


if __name__ == "__main__":
    unittest.main()
