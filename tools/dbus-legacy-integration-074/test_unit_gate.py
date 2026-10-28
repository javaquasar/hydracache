import copy
import importlib.util
import json
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
spec = importlib.util.spec_from_file_location('legacy_exact_unit_gate', HERE / 'unit_native.py')
u = importlib.util.module_from_spec(spec)
spec.loader.exec_module(u)


def fixture():
    before = ROOT / 'target/dbus-original-delivery-20261028/unit-api-owned/runtime/fd-194-list/result.json'
    catalog = json.loads(before.read_bytes())['inventory']
    source = ROOT / 'target/dbus-legacy-integration-20261028/tests/input/fd/tools/long-run-supervisor-074/src'
    selected = u.select(catalog, source)
    return catalog, source, selected


class UnitGate(unittest.TestCase):
    def test_exact_source_selection_preserves_caller_and_all_prior_wire_cases(self):
        catalog, source, selected = fixture()
        self.assertEqual(len(selected), 64)
        self.assertEqual(sum(n.startswith('diagnostic_manager::tests::') for n in selected), 14)
        self.assertEqual(sum(n.startswith('diagnostic_loaded::tests::') for n in selected), 7)
        self.assertEqual(sum(n.startswith('systemd_unit::tests::') for n in selected), 2)
        self.assertFalse(any('reads_a_real_loaded_service_over_dbus' in n for n in selected))
        self.assertFalse(any('privileged_' in n for n in selected))
        self.assertEqual(len(catalog), 1593)

    def test_missing_or_duplicate_actual_catalog_member_refuses(self):
        catalog, source, selected = fixture()
        for changed in ([n for n in catalog if n != selected[0]], catalog + [selected[0]]):
            with self.assertRaises(ValueError):
                u.select(changed, source)

    def test_actual_rows_summary_exit_filtered_must_all_agree(self):
        catalog, _, selected = fixture()
        rows = [(n, 'ok') for n in selected]
        summaries = [('ok', '64', '0', '0', '0', '1529')]
        u.validate_accounting(rows, summaries, catalog, selected, 0)
        for case in ('zero', 'missing', 'duplicate', 'foreign', 'ignored', 'filtered', 'exit'):
            r, s, c, code = copy.deepcopy(rows), copy.deepcopy(summaries), list(catalog), 0
            if case == 'zero': r = []
            if case == 'missing': r.pop()
            if case == 'duplicate': r[-1] = r[0]
            if case == 'foreign': r[-1] = ('foreign::case', 'ok')
            if case == 'ignored': r[-1] = (r[-1][0], 'ignored')
            if case == 'filtered': s[0] = ('ok', '64', '0', '0', '0', '0')
            if case == 'exit': code = 101
            with self.subTest(case=case), self.assertRaises(ValueError):
                u.validate_accounting(r, s, c, selected, code)


if __name__ == '__main__':
    unittest.main()
