from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import output_binding_control as subject
from owned_process_observer import Refusal


class FixtureControls(unittest.TestCase):
    def test_finite_schedule_stops_on_first_failure(self):
        with tempfile.TemporaryDirectory() as temp, patch.object(subject, "control", return_value=False) as control:
            self.assertFalse(subject.cohort(Path(temp) / "out"))
            control.assert_called_once()

    def test_exact_two_cases_no_retry(self):
        with tempfile.TemporaryDirectory() as temp, patch.object(subject, "control", return_value=True) as control:
            self.assertTrue(subject.cohort(Path(temp) / "out"))
            self.assertEqual([call.args[-1] for call in control.call_args_list], ["healthy", "writer-held"])

    def test_preflight_failure_retained_before_spawn_and_not_overwritten(self):
        with tempfile.TemporaryDirectory() as temp, patch.object(subject, "pinned", side_effect=Refusal("pin drift")), \
                patch.object(subject.subprocess, "Popen") as spawn:
            out = Path(temp) / "out"
            self.assertFalse(subject.control(out, "healthy"))
            spawn.assert_not_called()
            with self.assertRaises(FileExistsError):
                subject.control(out, "healthy")
