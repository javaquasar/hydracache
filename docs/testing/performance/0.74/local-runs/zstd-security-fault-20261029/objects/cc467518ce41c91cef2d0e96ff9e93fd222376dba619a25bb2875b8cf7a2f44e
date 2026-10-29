import copy
import json
import unittest

import profile_review as p


def fixture():
    phases = []
    for name in p.PHASES[:6]:
        phases.append(dict(phase=name, instrumented=True, guard_clock_added=False, deadline_changed=False,
            healthy_gate=False, includes_child_cpu=False, overlapping_spans=True, wall_ns=10,
            process_cpu_ns=11, returned=name != p.PHASES[5], authentication_calls=17, signature_calls=17,
            kernel=[dict(calls=1, inclusive_ns=8, outside_children_ns=3) for _ in p.KERNEL],
            live=[dict(calls=1, inclusive_ns=9, outside_children_ns=9) for _ in p.LIVE]))
    original = dict(case='healthy', returned=False, guard_clock_added=False, deadline_changed=False,
        causal=dict(first_refusal=dict(age_ns=10_000_000_001, issued_ns=1, now_ns=10_000_000_002,
                                      diagnostic_reason='expired')))
    return phases, original


def encode(phases, original):
    # libtest prefixes the first println with the test name on the same line.
    return ('test name ... ' + '\n'.join('HC074-HELD-PHASE ' + json.dumps(row) for row in phases)
            + '\nHC074-HELD-LIVE-ORIGINAL ' + json.dumps(original) + '\n').encode()


class Tests(unittest.TestCase):
    def test_partial_first_result_is_retained_not_promoted(self):
        result = p.review(encode(*fixture()), 101, 101, 0)
        self.assertEqual(len(result['phases']), 6)
        self.assertEqual(result['first_failed_phase'], 'owned-root-preparation')
        self.assertEqual(result['not_observed_phases'], list(p.PHASES[6:]))
        self.assertFalse(result['diagnostic_complete'])
        self.assertFalse(result['admission'])
        self.assertTrue(result['postflight_passed'])

    def test_observer_clocks_missing_cpu_and_failed_postflight_remain_separate(self):
        phases, original = fixture()
        phases[0]['process_cpu_ns'] = None
        result = p.review(encode(phases, original), 101, 101, 91)
        self.assertFalse(result['postflight_passed'])
        self.assertFalse(result['wall_residual_is_io'])

    def test_changed_schedule_clocks_spans_or_refusal_fail_closed(self):
        for case in ('zero', 'phase', 'duplicate', 'guard', 'span', 'cpu', 'continued', 'age'):
            phases, original = copy.deepcopy(fixture())
            code = 101
            if case == 'zero': code = 0
            if case == 'phase': phases[0]['phase'] = 'outputs'
            if case == 'duplicate': phases.insert(1, phases[0])
            if case == 'guard': phases[0]['guard_clock_added'] = True
            if case == 'span': phases[0]['kernel'][0]['outside_children_ns'] = 9
            if case == 'cpu': phases[0]['process_cpu_ns'] = -1
            if case == 'continued': phases[0]['returned'] = False
            if case == 'age': original['causal']['first_refusal']['now_ns'] += 1
            with self.subTest(case=case), self.assertRaises(ValueError):
                p.review(encode(phases, original), code, 101, 0)


if __name__ == '__main__':
    unittest.main()
