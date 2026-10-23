"""Offline outcome controls for the fresh configuration-only lane."""
import json
from pathlib import Path
import tempfile
import unittest

import registration


class Outcome(unittest.TestCase):
    def fixture(self, folder, complete=True):
        files = {'postflight.json': {'root': {}, 'root_controls_equal': True, 'owner_cleanup': True,
                                    'guardian_cleanup': True, 'boot_unchanged': True,
                                    'pid1_start_unchanged': True, 'time_namespace_unchanged': True},
                 'guardian.json': {'reason': 'normal_completion'},
                 'session-intent.json': {'definitions': ['p:owned/one original']},
                 'partial-state.json': {'registered_probes': 'p:owned/one original\n'}}
        if complete:
            files['configuration-complete.json'] = {'complete': True, 'activated': False}
        for name, value in files.items():
            (folder / name).write_text(json.dumps(value))
        (folder / 'operations.jsonl').write_text(json.dumps({'operation': 'write',
            'path': '/owned/tracing_on', 'value': '0', 'phase': 'after'}) + '\n')
        return files

    def test_complete_configuration_and_refusal_are_distinct(self):
        for complete in (True, False):
            with self.subTest(complete=complete), tempfile.TemporaryDirectory() as name:
                folder = Path(name)
                self.fixture(folder, complete)
                result = registration.outcome(folder)
                self.assertIs(result['configuration_complete'], complete)
                self.assertTrue(result['cleanup_context_confirmed'])
                self.assertFalse(result['trace_activation'])
                self.assertEqual(result['clock_queries'], 0)

    def test_changed_definition_context_deadline_or_activation_refuse(self):
        for mutation in ('definition', 'context', 'deadline', 'activation', 'contradictory_failure'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as name:
                folder = Path(name)
                files = self.fixture(folder)
                if mutation == 'definition':
                    files['partial-state.json']['registered_probes'] = 'p:owned/one changed\n'
                elif mutation == 'context':
                    files['postflight.json']['boot_unchanged'] = False
                elif mutation == 'deadline':
                    files['guardian.json']['reason'] = 'hard_deadline'
                else:
                    item = {'operation': 'write', 'path': '/owned/tracing_on',
                            'value': '1' if mutation == 'activation' else '0',
                            'phase': 'after' if mutation == 'activation' else 'failed'}
                    (folder / 'operations.jsonl').write_text(json.dumps(item) + '\n')
                for filename, value in files.items():
                    (folder / filename).write_text(json.dumps(value))
                with self.assertRaises(ValueError):
                    registration.outcome(folder)


if __name__ == '__main__':
    unittest.main()
