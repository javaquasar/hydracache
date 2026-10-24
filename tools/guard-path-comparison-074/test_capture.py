import copy
import json
from pathlib import Path
import unittest
import capture


def fixture():
    rows = []
    for block in range(6):
        order = (False, True, True, False) if block % 2 == 0 else (True, False, False, True)
        for slot, candidate in enumerate(order):
            rows.append(dict(block=block, slot=slot, candidate=candidate, loops=256, paths=13,
                comparisons=13312, matched=1024, live=0, timing=False,
                allocations=0 if candidate else 13312, reallocations=0,
                requested=0 if candidate else 13312*80, peak=0 if candidate else 80))
    return rows


def encode(rows):
    return ('\n'.join('HC074_GUARD_PATH ' + json.dumps(row) for row in rows) +
        '\nHC074_GUARD_PATH_DONE {"samples":24,"warmup":false,"product":false,"package_replay":false}\n').encode()


class Controls(unittest.TestCase):
    def test_complete_allocation_only_and_not_retained_rss_claim(self):
        result = capture.review(encode(fixture()))
        self.assertEqual(result['per_predicate']['baseline']['allocations'], 1)
        self.assertEqual(result['per_predicate']['candidate']['requested'], 0)
        self.assertFalse(result['timing'])
        self.assertFalse(result['full_normalizer'])
        self.assertFalse(result['rss_or_allocator_retention'])

    def test_missing_duplicate_reordered_scope_and_leaked_counts_refuse(self):
        rows = fixture()
        bad = [rows[:-1], rows + rows[:1], [rows[1], rows[0], *rows[2:]]]
        for key, value in [('loops', 257), ('candidate', 1), ('block', False), ('comparisons', 13313),
                           ('matched', 1023), ('live', 1), ('allocations', 1), ('timing', True), ('timing', 0)]:
            changed = copy.deepcopy(rows)
            changed[1][key] = value
            bad.append(changed)
        for changed in bad:
            with self.assertRaises(ValueError):
                capture.review(encode(changed))

    def test_fixed_proposal_and_fixture_are_bound_to_baseline(self):
        folder = Path(__file__).resolve().parent
        self.assertEqual(capture.sha((folder / 'fixed-slice-before.txt').read_bytes()), capture.BASELINE)
        self.assertEqual(capture.sha((capture.ROOT / capture.SOURCE).read_bytes()), capture.BASELINE)


if __name__ == '__main__':
    unittest.main()
