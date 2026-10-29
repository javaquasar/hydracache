"""Offline arithmetic/attribution controls using the sealed first trace only."""
import copy
import json
import unittest

import attribution as a


class Attribution(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import sys
        sys.path.insert(0, str(a.ROOT / 'tools/wsl-clock-readonly-074'))
        import offline_join
        files = offline_join.load(a.INPUT, a.INDEX_SHA)
        cls.analysis = json.loads(files['observation/analysis.json'])
        cls.rows = [json.loads(s) for s in files['observation/samples.jsonl'].splitlines()]

    def test_exact_counts_windows_and_rate_enclosure(self):
        result = a.summarize(self.analysis, self.rows)
        self.assertEqual((result['rows'], result['successful_calls'], result['tick_setting_requests']), (161, 45, 18))
        self.assertEqual(result['tick_setters_observed'], ['chronyd'])
        self.assertEqual([w['net_minus_offset_ns'] for w in result['step_windows']], [-6085369252, -6080632642, -6087516031])
        lo, hi = result['observed_monotonic_per_raw_enclosure']
        for value in result['tick_plus_frequency_rate_range']:
            self.assertLessEqual(lo, value)
            self.assertLessEqual(value, hi)
        self.assertIsNone(result['historical_tick_setter'])
        self.assertFalse(result['authenticated_executable_identity'])

    def test_loss_context_and_missing_window_refuse(self):
        for mutation in ('loss', 'context', 'window'):
            with self.subTest(mutation=mutation):
                analysis, rows = copy.deepcopy(self.analysis), copy.deepcopy(self.rows)
                if mutation == 'loss':
                    analysis['loss_checked'] = False
                elif mutation == 'context':
                    rows[0]['modes'] = 1
                else:
                    analysis['calls'] = [c for c in analysis['calls'] if c['entry']['event'] != 'offset']
                with self.assertRaises(ValueError):
                    a.summarize(analysis, rows)

    def test_failed_tick_request_does_not_identify_a_successful_writer(self):
        analysis = copy.deepcopy(self.analysis)
        for call in analysis['calls']:
            if call['entry']['event'] == 'adjust':
                call['successful'] = False
        result = a.summarize(analysis, self.rows)
        self.assertEqual(result['tick_setting_requests'], 0)
        self.assertEqual(result['tick_setters_observed'], [])


if __name__ == '__main__':
    unittest.main()
