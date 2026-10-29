"""Fresh-capture orchestration tests; all WSL calls and decoding are mocked."""
from contextlib import ExitStack
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import observe


class Orchestration(unittest.TestCase):
    def exercise(self, folder, failure=False):
        with ExitStack() as stack:
            stack.enter_context(patch.object(observe, 'OUT', folder))
            stack.enter_context(patch.object(observe, 'configure_archive'))
            stack.enter_context(patch.object(observe.base, 'sources', return_value={}))
            stack.enter_context(patch.object(observe.subprocess, 'check_output', side_effect=[b'', 'pinned-source\n']))
            stack.enter_context(patch.object(observe.base, 'linux', side_effect=lambda p: str(p)))
            command = stack.enter_context(patch.object(observe.base, 'command'))
            decode = stack.enter_context(patch.object(observe.analyze, 'analyze',
                side_effect=ValueError('first decoder refusal') if failure else None,
                return_value={'admission': False}))
            if failure:
                with self.assertRaisesRegex(ValueError, 'first decoder refusal'):
                    observe.collect()
            else:
                observe.collect()
            self.assertEqual([c.args[0] for c in command.call_args_list], ['controls', 'session'])
            self.assertEqual(command.call_args_list[-1].args[2], 180)
            decode.assert_called_once_with(folder)

    def test_success_uses_one_dispatch_and_fixed_limits(self):
        with tempfile.TemporaryDirectory() as name:
            folder = Path(name) / 'fresh'
            self.exercise(folder)
            intent = json.loads((folder / 'intent.json').read_text())
            self.assertEqual((intent['capture_seconds_raw'], intent['guardian_seconds_raw'], intent['maximum_queries']), (90, 120, 200))
            self.assertEqual(json.loads((folder / 'analysis.json').read_text()), {'admission': False})
            self.assertFalse((folder / 'first-analysis-refusal.json').exists())

    def test_first_decoder_refusal_preserved_without_live_retry(self):
        with tempfile.TemporaryDirectory() as name:
            folder = Path(name) / 'fresh'
            self.exercise(folder, failure=True)
            refusal = json.loads((folder / 'first-analysis-refusal.json').read_text())
            self.assertIn('first decoder refusal', refusal['error'])
            self.assertFalse(refusal['new_live_dispatch_allowed'])
            self.assertFalse((folder / 'analysis.json').exists())


if __name__ == '__main__':
    unittest.main()
