import unittest
import review as r


def fixture():
    cohorts = []
    for cohort in range(8):
        lines = [f'HC074-FD-IDENTITY-ORIGIN cohort={cohort} files=7 iterations=8192 samples=12 product=false admission=false']
        for sample in range(12):
            for slot, variant in enumerate(r.ORDERS[(sample + cohort) % 6]):
                value = [100000, 100000, 70000][variant]
                lines.append(f'HC074-FD-IDENTITY cohort={cohort} sample={sample} slot={slot} variant={r.LABELS[variant]} calls=57344 cpu_ns={value} wall_ns={value}')
        cohorts.append(('\n'.join(lines) + '\n').encode())
    return cohorts


class Tests(unittest.TestCase):
    def test_installed_origin_scope_does_not_claim_complete_preparation(self):
        result = r.review_origin(origin_fixture())
        self.assertTrue(result['origin_cpu_screen_passed'])
        self.assertFalse(result['full_preparation_proven'])
        self.assertFalse(result['admission'])

    def test_origin_incomplete_work_or_wrong_result_cannot_pass(self):
        for before, after in [(b'calls=1024', b'calls=1025'), (b'slot=0', b'slot=1'),
                              (b'1 passed', b'0 passed'), (b'cpu_ns=100000', b'cpu_ns=0')]:
            cohorts = origin_fixture(); cohorts[0] = cohorts[0].replace(before, after, 1)
            with self.subTest(before=before), self.assertRaises(ValueError): r.review_origin(cohorts)

    def test_control_and_prototype_do_not_grant_runtime_or_release_admission(self):
        result = r.review(fixture())
        self.assertTrue(result['leaf_screen_passed'])
        self.assertFalse(result['runtime_improvement_proven'])
        self.assertFalse(result['admission'])

    def test_missing_or_duplicate_rows_or_processes_refuse(self):
        for kind in ('process', 'row', 'duplicate'):
            cohorts = fixture()
            if kind == 'process': cohorts.pop()
            if kind == 'row': cohorts[0] = cohorts[0].rsplit(b'\n', 2)[0] + b'\n'
            if kind == 'duplicate': cohorts[0] += cohorts[0].splitlines()[1] + b'\n'
            with self.subTest(kind=kind), self.assertRaises(ValueError): r.review(cohorts)

    def test_work_order_clock_and_origin_changes_refuse(self):
        for before, after in [(b'calls=57344', b'calls=57343'), (b'slot=0', b'slot=2'),
                              (b'cpu_ns=100000', b'cpu_ns=-1'), (b'admission=false', b'admission=true')]:
            cohorts = fixture(); cohorts[0] = cohorts[0].replace(before, after, 1)
            with self.subTest(before=before), self.assertRaises(ValueError): r.review(cohorts)

    def test_no_win_and_noisy_identical_control_stay_rejected(self):
        cohorts = [raw.replace(b'cpu_ns=70000', b'cpu_ns=99000') for raw in fixture()]
        self.assertFalse(r.review(cohorts)['leaf_screen_passed'])
        cohorts = [raw.replace(b'variant=reference-control calls=57344 cpu_ns=100000',
                               b'variant=reference-control calls=57344 cpu_ns=110000') for raw in fixture()]
        self.assertFalse(r.review(cohorts)['same_function_control_passed'])

    def test_seeded_interval_is_reproducible(self):
        self.assertEqual(r.interval([-.01, .01, 0, .002, -.002, .001, -.001, .003]),
                         r.interval([-.01, .01, 0, .002, -.002, .001, -.001, .003]))


def origin_fixture():
    result = []
    for cohort in range(8):
        lines = ['test name ... ']
        for sample in range(12):
            for slot, variant in enumerate(r.ORDERS[(sample + cohort) % 6]):
                label = ('reference', 'reference-control', 'fstat-installed')[variant]
                value = (100000, 100000, 94000)[variant]
                lines.append(f'HC074-ORIGIN-IDENTITY cohort={cohort} sample={sample} slot={slot} variant={label} calls=1024 cpu_ns={value} wall_ns={value}')
        lines.append('test result: ok. 1 passed; 0 failed; 0 ignored;')
        result.append(('\n'.join(lines)+'\n').encode())
    return result


if __name__ == '__main__': unittest.main()
