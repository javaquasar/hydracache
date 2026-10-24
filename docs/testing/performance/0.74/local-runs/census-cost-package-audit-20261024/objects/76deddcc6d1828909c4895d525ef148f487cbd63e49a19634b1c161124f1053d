from pathlib import Path
import unittest
from unittest.mock import patch

import package_replay_audit


class PackageAuditControls(unittest.TestCase):
    def test_current_inventory_and_old_outcomes_require_no_execution(self):
        root = Path(__file__).resolve().parents[2]
        with patch('subprocess.Popen', side_effect=AssertionError('read-only audit')):
            result = package_replay_audit.audit(root)
        self.assertEqual(len(result['targets']), len(set(result['targets'])))
        self.assertIn('tests/census_cost_074.rs', result['targets'])
        self.assertEqual(result['outer_bound_seconds'], 780)
        self.assertFalse(result['execution_effects_cleared'])
        self.assertFalse(result['package_replayed'])
        self.assertTrue(result['lexical_sightings'])
        self.assertIn('NOT expanded Rust AST', result['scanner_scope'])


if __name__ == '__main__':
    unittest.main()
