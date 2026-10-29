"""Synthetic saved-log controls only; no native/live process."""
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location('ordinary_review', Path(__file__).with_name('review.py'))
review = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(review)


class ReviewControls(unittest.TestCase):
    def event(self, clock, site='clock-order'):
        return f'HC074-KERNEL-FIRST-REFUSAL Event {{ phase: "callback", site: "{site}", file: "source.rs", line: 259, clock: [{clock}] }}'

    def test_backward_realtime_is_not_boot_or_boottime_failure(self):
        row = review.events(self.event('100, 200, 1000, 900, 1, 60000000000'))[0]
        self.assertEqual(row['failed_predicates'], ['realtime-backwards'])
        self.assertEqual(row['realtime_backward_ns'], 100)
        self.assertEqual(row['boottime_delta_ns'], 100)

    def test_simultaneous_failures_remain_distinct(self):
        row = review.events(self.event('200, 100, 1000, 900, 0, 60000000000'))[0]
        self.assertEqual(row['failed_predicates'], ['boot-id-changed', 'boottime-backwards', 'realtime-backwards'])

    def test_equal_readings_do_not_explain_clock_order_refusal(self):
        for clock in ('100, 100, 1000, 1000, 1, 60000000000',
                      '100, 200, 1000, 900, 1, 70000000000',
                      '100, 200, 1000, 900, 2, 60000000000'):
            with self.subTest(clock=clock), self.assertRaises(ValueError):
                review.events(self.event(clock))

    def test_unknown_and_interleaved_lines_are_retained_not_discarded(self):
        raw = 'prefix ' + self.event('100, 200, 1000, 900, 1, 60000000000')
        self.assertEqual(review.events(raw), [dict(stderr_line=1, raw=raw, parsed=False)])
        row = review.events(self.event('0, 0, 0, 0, 0, 0', 'sender'))[0]
        self.assertNotIn('failed_predicates', row)

    def test_complete_parent_membership_and_failures(self):
        text = 'running 3 tests\ntest a ... ok\ntest b ... FAILED\n'
        text += 'test result: FAILED. 1 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out; finished in 1.00s\n'
        result = review.ordinary(text)
        self.assertEqual(result['selected'], 3)
        self.assertEqual(result['failed_names'], ['b'])
        for invalid in (text.replace('b ... FAILED', 'b ... ok'), text + text,
                        text.replace('0 filtered', '1 filtered'), text.replace('running 3', 'running 2')):
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                review.ordinary(invalid)


if __name__ == '__main__':
    unittest.main()
