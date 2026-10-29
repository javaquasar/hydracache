import copy
import unittest

import build
import run


class Tests(unittest.TestCase):
    def test_exact_extraction_and_ambiguity_refusal(self):
        raw = (build.SOURCE / 'diagnostic_kernel_admission.rs').read_bytes().replace(b'\r\n', b'\n')
        code = build.extract(raw)
        self.assertTrue(code.startswith(b'#[derive(Debug, PartialEq, Eq)]\nstruct Projection {'))
        self.assertIn(b'fn projection(', code)
        self.assertNotIn(b'struct Sender', code)
        for bad in (b'', raw + raw):
            with self.assertRaises(ValueError):
                build.extract(bad)

    def test_estimator_is_fixed_and_workloads_must_match(self):
        def rows(cpu):
            return [dict(shape=shape, iterations=20_000, seed=0x07405ca1, workload_witness='frozen', cpu_ns=cpu)
                    for shape in (512, 2048, 8192)]
        pairs = [dict(baseline=rows(100), candidate=rows(70)) for _ in range(5)]
        self.assertTrue(all(row['passed'] for row in run.estimate(pairs)))
        self.assertTrue(all(not row['passed'] for row in run.estimate(
            [dict(baseline=rows(100), candidate=rows(90)) for _ in range(5)])))
        with self.assertRaises(ValueError):
            run.estimate(pairs[:4])
        for key in ('shape', 'iterations', 'seed', 'workload_witness'):
            changed = copy.deepcopy(pairs)
            changed[0]['candidate'][0][key] = 'changed'
            with self.subTest(key=key), self.assertRaises(ValueError):
                run.estimate(changed)


if __name__ == '__main__':
    unittest.main()
