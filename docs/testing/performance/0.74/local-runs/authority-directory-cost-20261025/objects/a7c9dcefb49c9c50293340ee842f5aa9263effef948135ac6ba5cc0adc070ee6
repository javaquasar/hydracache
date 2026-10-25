from pathlib import Path
import importlib.util
import json
import unittest

spec = importlib.util.spec_from_file_location('directory_review', Path(__file__).with_name('review.py'))
api = importlib.util.module_from_spec(spec)
spec.loader.exec_module(api)


def synthetic(phase='prototype'):
    outputs = []
    for cohort in range(8):
        rows = ['HC074-DIR-ORIGIN '+json.dumps(dict(cohort=cohort, phase=phase, samples=16,
            iterations=1024, uid=1000, warmups=32, product=False, admission=False))]
        for layout in api.LAYOUTS:
            for sample in range(16):
                for slot, variant in enumerate(api.ORDERS[(sample+cohort) % 8]):
                    arm = api.ARMS[variant]
                    cpu = 80000 if arm == 'prototype' or (phase == 'installed' and arm == 'runtime') else 100000
                    rows.append('HC074-DIR-COST '+json.dumps(dict(cohort=cohort, phase=phase,
                        layout=layout, sample=sample, slot=slot, variant=arm, calls=1024,
                        metadata_queries_per_call=4 if layout == 'fixture' else 11,
                        cpu_ns=cpu, wall_ns=110000, product=False, admission=False)))
        rows.append('test result: ok. 1 passed; 0 failed; 0 ignored;')
        outputs.append(('\n'.join(rows)+'\n').encode())
    return outputs


class Controls(unittest.TestCase):
    def test_synthetic_phases_never_admit(self):
        for phase in ['prototype', 'installed']:
            result = api.review(synthetic(phase), phase)
            self.assertTrue(result['cpu_screen_passed'])
            self.assertFalse(result['admission'])

    def test_missing_process_or_invalid_phase(self):
        for rows, phase in [(synthetic()[:-1], 'prototype'), (synthetic(), 'other')]:
            with self.assertRaises(ValueError): api.review(rows, phase)

    def test_changed_work_order_clock_and_promoted_scope_refuse(self):
        for a, b in [('"calls": 1024','"calls": 1023'), ('"slot": 0','"slot": 9'),
                     ('"cpu_ns": 80000','"cpu_ns": 0'), ('"product": false','"product": true'),
                     ('"metadata_queries_per_call": 11','"metadata_queries_per_call": 10'),
                     ('"cohort": 0','"cohort": false')]:
            rows = synthetic(); rows[0] = rows[0].replace(a.encode(), b.encode(), 1)
            with self.assertRaises(ValueError): api.review(rows, 'prototype')

    def test_duplicate_row_and_wrong_verdict_refuse(self):
        rows = synthetic(); rows[0] += rows[0].splitlines()[1]+b'\n'
        with self.assertRaises(ValueError): api.review(rows, 'prototype')
        rows = synthetic(); rows[0] = rows[0].replace(b'1 passed;', b'0 passed;')
        with self.assertRaises(ValueError): api.review(rows, 'prototype')

    def test_actual_baseline_drift_is_not_a_valid_reference(self):
        rows = [row.replace(b'"variant": "runtime", "calls": 1024, "metadata_queries_per_call": 4, "cpu_ns": 100000',
            b'"variant": "runtime", "calls": 1024, "metadata_queries_per_call": 4, "cpu_ns": 110000') for row in synthetic()]
        self.assertFalse(api.review(rows, 'prototype')['cpu_screen_passed'])

    def test_negative_result_is_retained(self):
        rows = [row.replace(b'"cpu_ns": 80000', b'"cpu_ns": 105000') for row in synthetic()]
        self.assertFalse(api.review(rows, 'prototype')['cpu_screen_passed'])


if __name__ == '__main__': unittest.main()
