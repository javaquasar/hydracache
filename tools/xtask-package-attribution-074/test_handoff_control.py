from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import handoff_control as subject
from owned_process_observer import Refusal


class ControlTests(unittest.TestCase):
    def test_fixed_schedule_stops_on_first_unexpected_failure(self):
        with tempfile.TemporaryDirectory() as temp, patch.object(subject, "control", return_value=False) as control:
            self.assertFalse(subject.cohort(Path(temp) / "first"))
            control.assert_called_once()

    def test_two_fixed_modes_once(self):
        with tempfile.TemporaryDirectory() as temp, patch.object(subject, "control", return_value=True) as control:
            self.assertTrue(subject.cohort(Path(temp) / "first"))
            self.assertEqual([call.args[-1] for call in control.call_args_list], ["healthy", "lost-ack"])

    def test_pin_failure_before_spawn_is_retained_without_overwrite(self):
        with tempfile.TemporaryDirectory() as temp:
            out = Path(temp) / "first"
            with patch.object(subject, "pinned", side_effect=Refusal("pin drift")), \
                    patch.object(subject.subprocess, "Popen") as spawn:
                self.assertFalse(subject.control(out, "healthy"))
                spawn.assert_not_called()
            with self.assertRaises(FileExistsError):
                subject.control(out, "healthy")
