import unittest
import run


class CatalogControls(unittest.TestCase):
    @staticmethod
    def raw(names):
        return ('\n'.join(n + ': test' for n in names) + f'\n{len(names)} tests, 0 benchmarks\n').encode()

    def test_exact_parent_and_seven_additions(self):
        old = ['old::' + str(i) for i in range(14)]
        names = old + run.EXTRAS
        self.assertEqual(run.inventory(self.raw(names), old), sorted(names))
        for changed in (names[:-1], names + ['foreign'], names[:-1] + ['foreign']):
            with self.assertRaises(ValueError):
                run.inventory(self.raw(changed), old)

    def test_duplicate_refuses(self):
        old = ['old::' + str(i) for i in range(14)]
        names = old + run.EXTRAS
        with self.assertRaises(ValueError):
            run.inventory(self.raw(names[:-1] + [names[0]]), old)

    def test_empty_or_false_summary_refuses(self):
        old = ['old::' + str(i) for i in range(14)]
        for raw in (b'0 tests, 0 benchmarks\n', self.raw(old + run.EXTRAS).replace(b'21 tests', b'0 tests')):
            with self.assertRaises(ValueError):
                run.inventory(raw, old)


if __name__ == '__main__':
    unittest.main()
