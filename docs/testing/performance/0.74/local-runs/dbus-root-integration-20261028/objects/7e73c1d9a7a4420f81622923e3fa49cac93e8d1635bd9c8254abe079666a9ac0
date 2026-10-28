"""Inert render checks: broader feature coverage with the same owner and bounds."""
from pathlib import Path
import ast
import unittest

import feature_run as f


class Features(unittest.TestCase):
    def test_render_keeps_owner_floor_deadlines_and_strict_locked_source_guard(self):
        source = f.render()
        ast.parse(source)
        self.assertIn("'metadata', '--offline', '--locked', '--all-features'", source)
        self.assertNotIn('lock_may_change=True)', source)
        self.assertIn('locked=True, only_candidate_lock_may_change=False', source)
        self.assertIn("r.BASE / 'resolved-review.json'", source)
        self.assertIn("OUT = r.BASE / 'metadata-all-features'", source)
        for original in ('time.monotonic() + 1100', 'bound=1200', 'floor=FLOOR, poll=5, retirement=5',
                         'original owned Job retirement', 'root/head postflight', 'validate_metadata(metadata, review)',
                         'actual == expected', 'source_unchanged=True'):
            self.assertIn(original, source)

    def test_no_alternate_old_resolution_or_compiler_command_is_added(self):
        source = f.render()
        tree = ast.parse(source)
        command = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'command')
        assignment = next(n for n in command.body if isinstance(n, ast.Assign) and
                          any(isinstance(v, ast.Name) and v.id == 'argv' for v in n.targets))
        literals = [n.value for n in ast.walk(assignment.value) if isinstance(n, ast.Constant) and isinstance(n.value, str)]
        self.assertEqual(literals[:6], ['metadata', '--offline', '--locked', '--all-features', '--format-version', '1'])
        self.assertFalse(any(n in literals for n in ('update', 'fetch', 'build', 'check', 'test', 'install')))


if __name__ == '__main__':
    unittest.main()
