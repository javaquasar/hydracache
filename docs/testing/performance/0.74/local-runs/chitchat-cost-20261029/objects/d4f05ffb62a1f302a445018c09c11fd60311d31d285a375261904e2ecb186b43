"""Offline checks of immutable first refusal; no clocks/processes are sampled."""
import unittest
import review_refusal as r
import common as c


class Refusal(unittest.TestCase):
    def test_first_control_remains_unqualified(self):
        result=r.review();self.assertFalse(result['clock_control_pass']);self.assertEqual(result['cohort_processes_dispatched'],0)
        self.assertGreater(result['relative_discrepancy'],result['discrepancy_limit'])
        self.assertEqual(result['elapsed_ns']['mono'],2634582);self.assertEqual(result['elapsed_ns']['raw'],2776883)
        self.assertFalse(result['CPU_qualified']);self.assertFalse(result['RSS_qualified'])

    def test_constructor_matches_original_helper(self):
        original=(c.BASE/'fixtures/Q/src/types.rs').read_text()
        helper=original.split('pub fn for_test(value: &str, version: Version) -> Self {',1)[1].split('\n    }',1)[0]
        for field in ('value: value.to_string()','version,','status: DeletionStatus::Set'):self.assertIn(field,helper)
        probe=(c.TOOLS/'probe.rs').read_text()
        for field in ('value: value.to_string()','version: *version','status: crate::DeletionStatus::Set'):self.assertIn(field,probe)


if __name__=='__main__':unittest.main()
