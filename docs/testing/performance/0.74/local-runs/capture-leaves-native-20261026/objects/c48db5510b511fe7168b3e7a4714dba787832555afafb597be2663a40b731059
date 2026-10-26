import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import historical_target_replay as subject


class HistoricalReplayTests(unittest.TestCase):
    def test_first_reader_refusal_retained_without_running_producer(self):
        with tempfile.TemporaryDirectory() as temp:
            out = Path(temp) / "first"
            with patch.object(subject, "analyze", side_effect=ValueError("archived bytes drift")):
                self.assertFalse(subject.main(out))
            result = json.loads((out / "result.json").read_text())
            self.assertIn("archived bytes drift", result["first_refusal"])
            self.assertFalse(result["current_release_gate_pass"])
            with self.assertRaises(FileExistsError):
                subject.main(out)
