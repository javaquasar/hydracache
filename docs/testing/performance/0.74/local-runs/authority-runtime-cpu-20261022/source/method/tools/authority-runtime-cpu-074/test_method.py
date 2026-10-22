"""Offline controls only. Native CPU images are never executed here."""
import json
import copy
from pathlib import Path
import tempfile
import unittest
import run as method
import review
import seal


class Controls(unittest.TestCase):
    def test_primary_needs_both_compilers_and_strict_practical_interval(self):
        def cells(ratio):
            return [dict(mode='cycle', depth=127, fault='external', metrics={
                'cycle_wall_ns': review.comparison([100] * 7, [100 * ratio] * 7)})]
        self.assertEqual(review.primary(dict(a=cells(0.90), b=cells(0.94)), 'cycle', 'cycle_wall_ns')['disposition'], 'clear-local-benefit')
        self.assertEqual(review.primary(dict(a=cells(1.10), b=cells(1.06)), 'cycle', 'cycle_wall_ns')['disposition'], 'clear-local-cost')
        for left, right in ((0.90, 1.10), (0.95, 0.90), (1.05, 1.10)):
            self.assertEqual(review.primary(dict(a=cells(left), b=cells(right)), 'cycle', 'cycle_wall_ns')['disposition'], 'no-clear-practical-effect-or-mixed')

    def test_inventory_tracks_every_byte_excluding_only_root_inventory(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); (root / 'raw').write_bytes(b'first')
            before = seal.inventory(root)
            (root / 'inventory.json').write_text('{}')
            self.assertEqual(seal.inventory(root), before)
            (root / 'raw').write_bytes(b'other')
            self.assertNotEqual(seal.inventory(root), before)
            (root / 'nested').mkdir(); (root / 'nested/inventory.json').write_text('{}')
            self.assertEqual(len(seal.inventory(root)), 2)

    def test_seven_pairs_keep_zero_and_negative_effects_without_selection(self):
        value = review.comparison([100] * 7, [90, 100, 110, 120, 130, 140, 150])
        self.assertEqual(value['median_ratio'], 1.2)
        self.assertEqual(value['median_interval'], [0.9, 1.5])
        self.assertEqual(value['nominal_coverage'], 0.984375)
        self.assertEqual(review.comparison([100] * 7, [100] * 7)['median_ratio'], 1)
        for values in ([100] * 6, [float('nan')] * 7, [float('inf')] * 7, [True] * 7):
            with self.assertRaises(ValueError): review.comparison([100] * 7, values)

    def test_rows_clocks_membership_and_corpus_guards(self):
        cost = dict(wall_ns=100, cpu_ns=80, user_ns=0, system_ns=0, voluntary=0, involuntary=0, inblock=0, oublock=0)
        header = dict(wall_resolution_ns=1, cpu_resolution_ns=1, empty=[cost] * 8)
        rows = []
        for mode, depth, fault in review.CELLS:
            row = dict(mode=mode, depth=depth, fault=fault, seed=74, iterations=32, corpus='a' * 64,
                root_name_bytes=26, outcome_checked=True)
            if mode == 'phases':
                row.update(scratch_cleared=True, original_objects_preserved=True,
                    publication=cost if depth == 127 else None, **{key: cost for key in ('open', 'warm', 'refusal', 'sticky', 'drop')})
            else: row['cycle'] = cost
            rows.append(row)
        def render(values):
            return 'test ' + method.MEASURE + ' ... HC074-RUNTIME-CPU-CLOCKS ' + json.dumps(header) + '\n' + '\n'.join(
                'HC074-RUNTIME-CPU ' + json.dumps(row) for row in values)
        _, parsed = review.parse(render(rows))
        pairs = [dict(baseline=copy.deepcopy(parsed), installed=copy.deepcopy(parsed)) for _ in range(7)]
        summary = review.summarize(pairs)
        self.assertEqual(len(summary), 12)
        self.assertEqual(review.primary(dict(a=summary, b=summary), 'cycle', 'cycle_wall_ns')['disposition'], 'no-clear-practical-effect-or-mixed')
        pairs[0]['installed'][0]['corpus'] = 'different'
        with self.assertRaises(ValueError): review.summarize(pairs)
        with self.assertRaises(ValueError): review.parse(render(rows[:-1]))
        with self.assertRaises(ValueError): review.parse('unrelated ' + render(rows))
        changed = copy.deepcopy(rows); changed[0]['cpu_ns'] = 0; changed[0]['warm'] = dict(cost, cpu_ns=0)
        with self.assertRaises(ValueError): review.parse(render(changed))

    def test_exact_overlay_no_allocator_or_original_edits(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); tests = root / method.common.TESTS; tests.parent.mkdir(parents=True)
            original = {method.common.TESTS: b'fn original_body() {}\n'}
            tests.write_bytes(original[method.common.TESTS] + method.HOOK.encode())
            for path, raw in method.generated('installed').items():
                target = root / path; target.parent.mkdir(parents=True, exist_ok=True); target.write_bytes(raw)
            rows = method.validate(root, original, 'installed')
            self.assertEqual(len(rows), 3)
            self.assertFalse(any('counter' in row['path'] for row in rows))
            tests.write_bytes(tests.read_bytes() + b'fn extra() {}')
            with self.assertRaises(ValueError): method.validate(root, original, 'installed')

    def test_child_only_affinity_and_exact_selection(self):
        self.assertEqual(method.argv(3, '/owned/image', method.MEASURE),
            ['/usr/bin/taskset', '--cpu-list', '3', '/owned/image', method.MEASURE,
             '--exact', '--nocapture', '--test-threads=1'])
        self.assertEqual(method.PAIRS, 7)
        self.assertEqual(len(method.CONTROLS), 2)

    def test_fresh_artifact_and_uninterleaved_success(self):
        row = dict(reason='compiler-artifact', executable='/owned/image', fresh=False,
            target=dict(name='hydracache_long_run_supervisor_074'), profile=dict(test=True, opt_level='3'))
        self.assertEqual(method.common.image(json.dumps(row).encode()), '/owned/image')
        row['fresh'] = True
        with self.assertRaises(ValueError): method.common.image(json.dumps(row).encode())
        raw = ('running 1 test\ntest ' + method.MEASURE + ' ... ROW\nok\n'
            'test result: ok. 1 passed; 0 failed; 0 ignored;\n').encode()
        method.common.passed(raw, method.MEASURE)
        with self.assertRaises(ValueError): method.common.passed(raw.replace(b'0 ignored', b'1 ignored'), method.MEASURE)


if __name__ == '__main__': unittest.main(verbosity=2)
