import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('stamp_review', Path(__file__).with_name('review.py'))
api = importlib.util.module_from_spec(spec)
spec.loader.exec_module(api)


def samples(installed=False):
    result = []
    labels = ['reference', 'reference-control', 'fresh-stamp']
    for cohort in range(8):
        rows = [] if installed else [f'HC074-STAMP-ORIGIN cohort={cohort} files=7 iterations=8192 samples=12 product=false admission=false']
        for sample in range(12):
            for slot, variant in enumerate(api.common.ORDERS[(sample+cohort)%6]):
                cpu = 80000 if variant == 2 else 100000
                marker = 'HC074-STAMP-INSTALLED' if installed else 'HC074-STAMP'
                calls = 1024 if installed else 57344
                rows.append(f'{marker} cohort={cohort} sample={sample} slot={slot} variant={labels[variant]} calls={calls} cpu_ns={cpu} wall_ns=110000')
        if installed:
            rows.append('test result: ok. 1 passed; 0 failed; 0 ignored;')
        result.append(('\n'.join(rows)+'\n').encode('ascii'))
    return result


class Controls(unittest.TestCase):
    def test_synthetic_leaf_and_installed(self):
        for installed in [False, True]:
            r = api.review(samples(installed), installed)
            self.assertTrue(r['cpu_screen_passed'])
            self.assertFalse(r['admission'])

    def test_missing_process(self):
        with self.assertRaises(ValueError): api.review(samples()[:-1])

    def test_wrong_calls_order_clock_and_variant(self):
        for a, b in [('calls=57344','calls=57343'), ('slot=0','slot=9'),
                     ('cpu_ns=80000','cpu_ns=0'), ('variant=fresh-stamp','variant=other'),
                     ('variant=fresh-stamp','variant=fstat-prototype')]:
            rows = samples(); rows[0] = rows[0].replace(a.encode(), b.encode(), 1)
            with self.assertRaises(ValueError): api.review(rows)

    def test_installed_wrong_work_or_verdict(self):
        for a, b in [('calls=1024', 'calls=1023'), ('1 passed;', '0 passed;')]:
            rows = samples(True); rows[0] = rows[0].replace(a.encode(), b.encode(), 1)
            with self.assertRaises(ValueError): api.review(rows, True)

    def test_negative_is_retained_not_admitted(self):
        rows = [r.replace(b'cpu_ns=80000', b'cpu_ns=105000') for r in samples()]
        self.assertFalse(api.review(rows)['cpu_screen_passed'])

    def test_identical_control_drift_refuses(self):
        rows = [r.replace(b'variant=reference-control calls=57344 cpu_ns=100000', b'variant=reference-control calls=57344 cpu_ns=110000') for r in samples()]
        self.assertFalse(api.review(rows)['cpu_screen_passed'])


if __name__ == '__main__': unittest.main()
