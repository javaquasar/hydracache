import json
import unittest

import review as r


def report(case, returned=True):
    age = 10_015_008_749
    return {'case': case, 'returned': returned, 'deadline_changed': False, 'guard_clock_added': False,
            'causal': {'first_refusal': None if returned else {
                'diagnostic_reason': 'expired', 'age_ns': age, 'issued_ns': 100,
                'now_ns': 100 + age}}, 'kernel': None, 'age': {'sites': []}}


def encoded(reports, successes=(), summaries=0):
    lines = ['HC074-HELD-LIVE-ORIGINAL ' + json.dumps(x) for x in reports]
    lines += ['HC074-HELD-LIVE case=' + x + ' seed=0x074c051e actual_fixed=true actual_consuming=true' for x in successes]
    lines += ['test result: ok. 1 passed; 0 failed; 0 ignored;'] * summaries
    return ('\n'.join(lines) + '\n').encode()


class ReviewTests(unittest.TestCase):
    def test_first_healthy_expiry_stays_negative_despite_successful_postflight(self):
        result = r.holder(encoded([report('healthy', False)]), 101, 101, 0)
        self.assertFalse(result['complete_matrix_passed'])
        self.assertEqual(result['not_executed_cases'], r.CASES[1:])
        self.assertEqual(result['first_original_refusal']['causal']['age_ns'], 10_015_008_749)
        self.assertFalse(result['admission'])

    def test_full_synthetic_shape_is_not_original_delivery_or_production_retirement(self):
        raw = encoded([report(x) for x in r.CASES], r.CASES, 6)
        result = r.holder(raw, 0, 0, 0)
        self.assertTrue(result['complete_matrix_passed'])
        self.assertFalse(result['original_delivery_proven'])
        self.assertFalse(result['all_production_writers_retired_proven'])

    def test_zero_or_bad_postflight_cannot_promote_partial_shape(self):
        for exits in ((0, 0, 0), (0, 0, 1)):
            with self.assertRaises(ValueError):
                r.holder(encoded([report('healthy')]), *exits)

    def test_schedule_override_duplicate_or_cost_refuses(self):
        for raw in (encoded([report('trust-after')]), encoded([report('healthy'), report('healthy')]),
                    b'HC074-HELD-LIVE-COST unexpected\n'):
            with self.assertRaises(ValueError):
                r.holder(raw, 101, 101, 0)

    def test_changed_window_inconsistent_age_or_clock_guard_refuses(self):
        for field in ('age', 'guard', 'early'):
            item = report('healthy', False)
            if field == 'age':
                item['causal']['first_refusal']['now_ns'] += 1
            elif field == 'guard':
                item['guard_clock_added'] = True
            else:
                refusal = item['causal']['first_refusal']
                refusal.update(age_ns=9, now_ns=109)
            with self.assertRaises(ValueError):
                r.holder(encoded([item]), 101, 101, 0)


if __name__ == '__main__':
    unittest.main()
