import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import census_cost as cost


def fixture():
    rows = []
    for block in range(cost.BLOCKS):
        variants = ('legacy', 'current', 'current', 'legacy') if block % 2 == 0 else (
            'current', 'legacy', 'legacy', 'current')
        for operation in cost.OPERATIONS:
            for slot, variant in enumerate(variants):
                rows.append(dict(phase='e2e', block=block, slot=slot, operation=operation,
                    variant=variant, iterations=1, wall_ns=100, parent_cpu_ns=0, result_bytes=1))
    for block in range(cost.BLOCKS):
        for component in cost.COMPONENTS:
            iterations = 1 if component in ('pipe', 'owned') else 4 if component == 'body-predicate' else 128
            rows.append(dict(phase='component', block=block, component=component,
                             iterations=iterations, wall_ns=100, parent_cpu_ns=0))
    done = dict(e2e_samples=120, component_samples=48, git_reads_expected_e2e=120,
                git_reads_expected_components=12, first_samples_retained=True, warmup=False,
                package_replay=False, production_authority=False)
    return rows, done


def encode(rows, done):
    return ('\n'.join('HC074_COST ' + json.dumps(row) for row in rows)
            + '\nHC074_COST_DONE ' + json.dumps(done) + '\n').encode()


class CostControls(unittest.TestCase):
    def test_only_literal_inherited_pager_is_allowed(self):
        self.assertEqual(cost.environment_overrides({}), [])
        self.assertEqual(cost.environment_overrides({'GIT_PAGER': 'cat'}), [])
        for name, value in (('GIT_PAGER', 'other'), ('GIT_TRACE', '1'), ('GIT_DIR', '/tmp/x'),
                            ('GIT_CONFIG_COUNT', '0'), ('HC074_CENSUS_COST_INPUT', 'x')):
            self.assertEqual(cost.environment_overrides({name: value}), [name])

    def test_complete_protocol(self):
        rows, done = fixture()
        result = cost.review(encode(rows, done))
        self.assertEqual(result['summary']['original/current']['samples'], 12)
        self.assertEqual(result['summary']['reference']['iterations_per_sample'], 128)
        self.assertFalse(result['component_phases_are_disjoint'])

    def test_exact_libtest_prefix_not_arbitrary_prefix(self):
        rows, done = fixture()
        raw = encode(rows, done)
        self.assertEqual(len(cost.review(b'test census_cost_child ... ' + raw)['samples']), 168)
        with self.assertRaises(ValueError):
            cost.review(b'other text ' + raw)

    def test_truncation_and_duplicate_refuse(self):
        rows, done = fixture()
        for changed in (rows[:-1], rows + rows[:1]):
            with self.assertRaises(ValueError):
                cost.review(encode(changed, done))

    def test_reordered_and_stale_identity_refuse(self):
        rows, done = fixture()
        changes = [copy.deepcopy(rows) for _ in range(3)]
        changes[0][0], changes[0][1] = changes[0][1], changes[0][0]
        changes[1][0]['operation'] = 'view'
        changes[2][120]['iterations'] = 129
        for changed in changes:
            with self.assertRaises(ValueError):
                cost.review(encode(changed, done))

    def test_invalid_time_refuses(self):
        for value in (-1, True, '1', float('nan')):
            rows, done = fixture()
            rows[0]['wall_ns'] = value
            with self.assertRaises(ValueError):
                cost.review(encode(rows, done))

    def test_scope_and_missing_completion_refuse(self):
        rows, done = fixture()
        done['production_authority'] = True
        with self.assertRaises(ValueError):
            cost.review(encode(rows, done))
        with self.assertRaises(ValueError):
            cost.review(b'HC074_COST {}\n')

    def test_saved_derivation_retains_first_refusal_without_processes(self):
        with tempfile.TemporaryDirectory() as directory:
            saved, outer = Path(directory) / 'capture', Path(directory) / 'outer'
            saved.mkdir()
            outer.mkdir()
            intent = dict(source={'source.rs': 'digest'}, outer_seconds=180, no_retry=True, no_warmup=True)
            preflight = dict(source=intent['source'], binary_sha256='binary')
            refusal = dict(exit=0, reason='sample count', type='ValueError')
            result = dict(exit=1, outer_refusal=None, source_unchanged=True, head_unchanged=True)
            for folder, name, value in ((saved, 'intent.json', intent), (saved, 'preflight.json', preflight),
                    (saved, 'refusal.json', refusal), (outer, 'intent.json', {}), (outer, 'result.json', result)):
                (folder / name).write_text(json.dumps(value))
            rows, done = fixture()
            (saved / 'stdout.log').write_bytes(encode(rows, done) +
                b'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 1s\n')
            (saved / 'stderr.log').write_bytes(b'')
            with patch('subprocess.Popen', side_effect=AssertionError('no repeat')):
                derived = cost.derive_saved(saved, outer)
                self.assertEqual(derived['first_refusal_preserved'], refusal)
                self.assertEqual(len(derived['inputs_sha256']), 5)
                result['source_unchanged'] = False
                (outer / 'result.json').write_text(json.dumps(result))
                with self.assertRaises(ValueError):
                    cost.derive_saved(saved, outer)


if __name__ == '__main__':
    unittest.main()
