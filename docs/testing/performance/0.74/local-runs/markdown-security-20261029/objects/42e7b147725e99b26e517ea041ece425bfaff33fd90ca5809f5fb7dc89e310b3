"""Offline source/trace joins and explicit countermodels; no Git fixture run."""
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

import preservation_reuse_audit as audit


def frame(case, args):
    return dict(schema='hc074-helper-trace-v1', valid=True, nested_intervals=True,
        product=False, package_replay=False, case=case, thread='thread-'+case, started=3,
        records=[
            dict(id=2, parent=1, start_ns=1, end_ns=2, elapsed_ns=1, operation='output',
                 returned=True, unwinding=False, output_bytes=1, stderr_bytes=0, arguments=[]),
            dict(id=3, parent=1, start_ns=2, end_ns=3, elapsed_ns=1, operation='decode',
                 returned=True, unwinding=False, output_bytes=1, stderr_bytes=0, arguments=[]),
            dict(id=1, parent=0, start_ns=0, end_ns=4, elapsed_ns=4, operation='git',
                 returned=True, unwinding=False, output_bytes=1, stderr_bytes=0, arguments=args)])


class ReuseAudit(unittest.TestCase):
    def test_full_revision_is_not_replacement_disable_or_authority(self):
        self.assertEqual(audit.classify(['show', 'a'*40+':file.rs']), 'full40_default_replacements')
        value = audit.analyze([frame('a', ['show', 'a'*40+':file.rs'])])
        self.assertFalse(value['literal_reuse_is_admission'])
        self.assertFalse(value['reduced_startup_candidate_admitted'])

    def test_short_spelling_and_path_are_not_canonicalized(self):
        value = audit.analyze([frame('a', ['show', 'abcd:file.rs']),
                               frame('b', ['show', 'abcd'+'0'*36+':file.rs']),
                               frame('c', ['show', 'abcd:File.rs'])])
        self.assertEqual(value['distinct_literal_show_queries'], 3)
        self.assertEqual(value['literal_occurrences_beyond_first'], 0)

    def test_literal_repeats_do_not_become_observed_hits(self):
        value = audit.analyze([frame('a', ['show', 'abcd:file.rs']), frame('b', ['show', 'abcd:file.rs'])])
        self.assertEqual(value['literal_occurrences_beyond_first'], 1)
        self.assertEqual(value['repeated_queries'][0]['occurrences'], 2)
        self.assertFalse(value['observed_cache_hits'])

    def test_fresh_inventory_bytes_and_attributes_remain_separate(self):
        self.assertEqual(audit.classify(['ls-files', '--', audit.SCOPE]), 'fresh_worktree_inventory')
        self.assertEqual(audit.classify(['hash-object', '--no-filters', audit.FIXTURE]), 'fresh_fixture_bytes')
        self.assertEqual(audit.classify(['check-attr', 'text', '--', audit.FIXTURE]), 'fresh_fixture_attributes')
        self.assertEqual(audit.classify(['ls-tree', '-r', '--name-only', 'abcd', '--', audit.SCOPE]),
                         'historical_inventory_default_replacements')

    def test_unreviewed_shapes_refuse_without_resolving(self):
        for args in ([], ['show', 'HEAD:file'], ['show', 'abcd:../file'], ['show', 'abcd:/file'],
                     ['show', 'abcd:file\n'], ['--no-replace-objects', 'show', 'a'*40+':file'],
                     ['show', 'abcd:file', 'abcd:other'], ['ls-files'], ['check-attr', '-a', 'file']):
            with self.assertRaises(ValueError): audit.classify(args)

    def test_trace_loss_duplicate_case_and_bad_bytes_refuse(self):
        with self.assertRaises(ValueError): audit.analyze([])
        with self.assertRaises(ValueError): audit.analyze([frame('a', ['ls-files', '--', audit.SCOPE])])
        with self.assertRaises(ValueError): audit.analyze([frame('a', ['show', 'abcd:file'])]*2)
        bad = frame('a', ['show', 'abcd:file']); bad['records'][2]['output_bytes'] = 2
        with self.assertRaises(ValueError): audit.analyze([bad])

    def test_replacement_aba_countermodel_hides_a_required_refusal(self):
        # Model only, not a new real-Git result. Pins are independent initial A.
        before, during, after, independent_pin, current = b'A', b'B', b'A', b'A', b'A'
        self.assertEqual(before, after)  # endpoint comparison misses ABA
        self.assertEqual(before, independent_pin)  # cached output passes its pin
        self.assertEqual(current, before)  # cached comparison would admit
        self.assertNotEqual(current, during)  # fresh comparison must refuse

    def test_conditional_prefetch_countermodel_changes_failure_order(self):
        reached = []
        def original(helper_valid):
            reached.append('helper')
            if not helper_valid: return 'helper-refusal'
            reached.extend(('base', 'reviewed'))
            return 'continue'
        self.assertEqual(original(False), 'helper-refusal')
        self.assertEqual(reached, ['helper'])
        self.assertNotEqual(reached, ['base', 'reviewed', 'helper'])

    def test_saved_full_scope_is_offline_and_installed_helper_unchanged(self):
        root = Path(__file__).resolve().parents[2]
        with patch.object(subprocess, 'Popen', side_effect=AssertionError('no new child')):
            value = audit.audit(root)
        self.assertEqual(value['frames'], 19)
        self.assertEqual(value['git_calls'], 3157)
        self.assertEqual(value['show_calls'], 3141)
        self.assertEqual(value['distinct_literal_show_queries'], 3124)
        self.assertEqual(value['literal_occurrences_beyond_first'], 17)
        self.assertEqual(value['command_shapes']['short_default_replacements'], 1795)
        self.assertEqual(value['command_shapes']['full40_default_replacements'], 1346)
        self.assertEqual(value['fresh_read_calls'], 3140)
        self.assertFalse(value['current_helper_erases_to_original'])
        self.assertTrue(value['current_helper_equals_original'])
        self.assertTrue(value['retained_measured_helper_erases_to_original'])

    def test_installed_body_drift_refuses_even_with_valid_saved_trace(self):
        root = Path(__file__).resolve().parents[2]
        original = Path.read_text
        def read(path, *args, **kwargs):
            raw = original(path, *args, **kwargs)
            return raw+'\nfn unreviewed() {}\n' if path == root/audit.sweep.SOURCE else raw
        with patch.object(Path, 'read_text', read):
            with self.assertRaisesRegex(ValueError, 'installed helper differs'):
                audit.audit(root)

    def test_changed_packet_pin_refuses_before_trace_review(self):
        with patch.object(audit.review, 'sha', return_value='changed'):
            with self.assertRaisesRegex(ValueError, 'sealed helper packet drift'):
                audit.audit(Path(__file__).resolve().parents[2])
