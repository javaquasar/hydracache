import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock, patch

import lineage_control as subject
from owned_process_observer import Refusal


class ControlTests(unittest.TestCase):
    def test_preflight_refusal_retained_no_child_and_no_overwrite(self):
        with tempfile.TemporaryDirectory() as temp:
            out = Path(temp) / "first"
            with patch("process_lineage.SnapshotApi", side_effect=Refusal("platform")), \
                    patch.object(subject.subprocess, "Popen") as spawn:
                self.assertFalse(subject.control(out, 2))
            spawn.assert_not_called()
            self.assertIn("platform", json.loads((out / "result.json").read_text())["refusal"])
            self.assertTrue((out / "intent.json").exists())
            with self.assertRaises(FileExistsError):
                subject.control(out, 2)

    def test_finite_schedule_stops_on_first_failure(self):
        with tempfile.TemporaryDirectory() as temp:
            out = Path(temp) / "first"
            with patch.object(subject, "control", return_value=False) as control:
                self.assertFalse(subject.cohort(out))
            control.assert_called_once_with(out.resolve() / "two-level", 2, False)
            self.assertEqual(len(json.loads((out / "result.json").read_text())["results"]), 1)

    def test_finite_positive_and_negative_schedule_once(self):
        with tempfile.TemporaryDirectory() as temp:
            out = Path(temp) / "first"
            with patch.object(subject, "control", return_value=True) as control:
                self.assertTrue(subject.cohort(out))
            self.assertEqual([call.args[1:] for call in control.call_args_list],
                             [(2, False), (3, False), (2, True)])

    def test_node_depth_rejected_before_effect(self):
        with patch.object(subject.subprocess, "Popen") as spawn:
            with self.assertRaisesRegex(RuntimeError, "depth"):
                subject.node(4, "unused")
        spawn.assert_not_called()

    def test_leaf_and_parent_terminate_from_release(self):
        with tempfile.TemporaryDirectory() as temp:
            out = Path(temp)
            (out / "release").write_bytes(b"release")
            subject.node(0, out)
            child = Mock()
            child.wait.return_value = 0
            child.poll.return_value = 0
            with patch.object(subject.subprocess, "Popen", return_value=child):
                subject.node(1, out)
            child.kill.assert_not_called()

    def test_node_timeout_cleans_only_direct_owned_child(self):
        with tempfile.TemporaryDirectory() as temp:
            child = Mock()
            child.poll.return_value = None
            with patch.object(subject.subprocess, "Popen", return_value=child), \
                    patch.object(subject.time, "monotonic", side_effect=[0, 9]):
                with self.assertRaisesRegex(RuntimeError, "deadline"):
                    subject.node(1, temp)
            child.kill.assert_called_once_with()
            child.wait.assert_called_once_with(timeout=3)


if __name__ == "__main__":
    unittest.main()
