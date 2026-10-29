import unittest
import run


class CatalogControls(unittest.TestCase):
    @staticmethod
    def raw(names):
        return ('\n'.join(n + ': test' for n in names) + f'\n{len(names)} tests, 0 benchmarks\n').encode()

    def test_original_thirty_identity_contract(self):
        old = ['old::' + str(i) for i in range(21)]
        names = old + run.EXTRAS
        self.assertEqual(run.inventory(self.raw(names), old), sorted(names))
        for changed in (names[:-1], names + ['foreign'], names[:-1] + ['foreign'], names[:-1] + [names[0]]):
            with self.assertRaises(ValueError):
                run.inventory(self.raw(changed), old)

    def test_original_empty_or_false_summary_refuses(self):
        old = ['old::' + str(i) for i in range(21)]
        for raw in (b'0 tests, 0 benchmarks\n', self.raw(old + run.EXTRAS).replace(b'30 tests', b'0 tests')):
            with self.assertRaises(ValueError):
                run.inventory(raw, old)

    def test_v3_exact_original_plus_peer_control(self):
        old = ['old::' + str(i) for i in range(21)]
        names = old + run.EXTRAS + [run.PEER_CONTROL]
        self.assertEqual(run.inventory_v3(self.raw(names), old), sorted(names))

    def test_v3_missing_foreign_duplicate_or_old_catalog_refuses(self):
        old = ['old::' + str(i) for i in range(21)]
        names = old + run.EXTRAS + [run.PEER_CONTROL]
        for changed in (names[:-1], names[:-1] + ['foreign'], names + ['foreign'], names[:-1] + [names[0]]):
            with self.assertRaises(ValueError):
                run.inventory_v3(self.raw(changed), old)

    def test_v3_duplicate_baseline_refuses(self):
        old = ['old::' + str(i) for i in range(21)]
        names = old + run.EXTRAS + [run.PEER_CONTROL]
        with self.assertRaises(ValueError):
            run.inventory_v3(self.raw(names), old[:-1] + [old[0]])

    def test_v3_empty_false_summary_or_benchmark_refuses(self):
        old = ['old::' + str(i) for i in range(21)]
        raw = self.raw(old + run.EXTRAS + [run.PEER_CONTROL])
        for changed in (b'0 tests, 0 benchmarks\n', raw.replace(b'31 tests', b'30 tests'), raw.replace(b'0 benchmarks', b'1 benchmarks')):
            with self.assertRaises(ValueError):
                run.inventory_v3(changed, old)


if __name__ == '__main__':
    unittest.main()
