import unittest
import run


class CatalogControls(unittest.TestCase):
    @staticmethod
    def raw(names):
        return ('\n'.join(n + ': test' for n in names) + f'\n{len(names)} tests, 0 benchmarks\n').encode()

    def test_fixed_thirteen(self):
        baseline = ['old::' + str(i) for i in range(5)]
        names = baseline + run.EXTRAS
        self.assertEqual(run.inventory(self.raw(names), baseline), sorted(names))
        for bad in (names[:-1], names + ['foreign'], names[:-1] + [names[0]]):
            with self.assertRaises(ValueError):
                run.inventory(self.raw(bad), baseline)

    def test_only_declared_constructor_addition(self):
        baseline = ['old::' + str(i) for i in range(5)]
        names = baseline + run.EXTRAS + ['constructor_review::nested_constructor_panics_before_bad_arguments']
        self.assertEqual(run.inventory(self.raw(names), baseline, True), sorted(names))
        for bad in (names[:-1], names[:-1] + ['constructor_review::foreign']):
            with self.assertRaises(ValueError):
                run.inventory(self.raw(bad), baseline, True)

    def test_zero_and_wrong_summary_refuse(self):
        baseline = ['old::' + str(i) for i in range(5)]
        raw = self.raw(baseline + run.EXTRAS)
        for bad in (b'0 tests, 0 benchmarks\n', raw.replace(b'13 tests', b'0 tests')):
            with self.assertRaises(ValueError):
                run.inventory(bad, baseline)


if __name__ == '__main__':
    unittest.main()
