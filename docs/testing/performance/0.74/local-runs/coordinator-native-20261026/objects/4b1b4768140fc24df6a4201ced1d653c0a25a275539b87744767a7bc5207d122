import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import dispatch_request as subject
from owned_process_observer import Refusal


class RequestTests(unittest.TestCase):
    def test_prepare_does_not_admit_dispatch_even_if_preflight_passes(self):
        with tempfile.TemporaryDirectory() as temp, patch.object(subject, "workspace", return_value=Path(temp)), \
                patch.object(subject.launch_preflight, "evaluate", return_value=dict(preflight_pass=True, dispatch_authorized=False)):
            result = subject.request("prepare", "first")
            self.assertTrue(result["prepared"])
            self.assertEqual(result["real_target_spawns"], 0)
            result = subject.request("dispatch", "second")
            self.assertIn("forbids real-target dispatch", result["first_refusal"])
            self.assertEqual(result["real_target_spawns"], 0)
            with self.assertRaises(FileExistsError):
                subject.request("prepare", "first")
        self.assertNotIn("subprocess", subject.__dict__)

    def test_preflight_refusal_retained_and_invalid_labels_before_reads(self):
        with tempfile.TemporaryDirectory() as temp, patch.object(subject, "workspace", return_value=Path(temp)), \
                patch.object(subject.launch_preflight, "evaluate", side_effect=Refusal("first pin drift")) as evaluate:
            for label in ("../escape", "bad/name", "A", "a" * 65):
                with self.assertRaises(Refusal):
                    subject.request("dispatch", label)
            evaluate.assert_not_called()
            result = subject.request("dispatch", "first")
            self.assertIn("first pin drift", result["first_refusal"])
            self.assertTrue((Path(temp) / "target/observer-dispatch-074/first/intent.json").is_file())

    def test_unsuccessful_or_authorized_response_does_not_become_prepare(self):
        for response in (dict(preflight_pass=False, dispatch_authorized=False),
                         dict(preflight_pass=True, dispatch_authorized=True), {}):
            with tempfile.TemporaryDirectory() as temp, patch.object(subject, "workspace", return_value=Path(temp)), \
                    patch.object(subject.launch_preflight, "evaluate", return_value=response):
                self.assertFalse(subject.request("prepare", "first")["prepared"])


if __name__ == "__main__":
    unittest.main()
