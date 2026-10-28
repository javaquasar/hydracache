import unittest
import run


class ReviewControls(unittest.TestCase):
    def test_exact_inventory(self):
        baseline = ['old::' + str(i) for i in range(106)]
        names = baseline + ['delta::cost_review::' + n for n in ('allocator_control', 'allocation_observations', 'cpu_observations')]
        raw = ('\n'.join(n + ': test' for n in names) + '\n109 tests, 0 benchmarks\n').encode()
        self.assertEqual(run.inventory(raw, baseline), sorted(names))
        for invalid in (b'0 tests, 0 benchmarks\n', raw.replace(b'old::0:', b'foreign:'), raw + b'extra: test\n'):
            with self.assertRaises(ValueError):
                run.inventory(invalid, baseline)

    def test_semantics_refuses_missing_or_changed_wire(self):
        original = [dict(mtu=100, value_len=i, accepted=False, wire='00') for i in range(239)]
        run.semantic(original, original)
        for altered in (original[:-1], [dict(r, wire='01') for r in original], list(reversed(original))):
            with self.assertRaises(ValueError):
                run.semantic(altered, original)

    def test_duplicate_workload_refused(self):
        repeated = [dict(mtu=100, value_len=1, accepted=False, wire='00')] * 239
        with self.assertRaises(ValueError):
            run.semantic(repeated, repeated)

    def test_diagnostic_count(self):
        self.assertEqual(run.payload(b'HC:{"x":1}\n', b'HC:'), {'x': 1})
        for raw in (b'', b'HC:{}\nHC:{}\n'):
            with self.assertRaises(ValueError):
                run.payload(raw, b'HC:')


if __name__ == '__main__':
    unittest.main()
