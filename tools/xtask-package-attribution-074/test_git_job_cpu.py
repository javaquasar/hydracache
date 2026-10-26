"""Accounting and protocol faults; no real process, Job or package dispatch."""
import copy
import json
import unittest
from unittest.mock import Mock, patch

import git_job_cpu as subject
from owned_process_observer import Refusal


def inputs():
    job = dict(total=9, active=0, terminated=0, user_ns=10000, kernel_ns=8000)
    worker = dict(terminal=True, creation_100ns=1, exit_100ns=2, user_100ns=10, kernel_100ns=10)
    launchers = [dict(pid=i+1, created_100ns=i+1, exited_100ns=i+10, user_ns=100, kernel_ns=200) for i in range(4)]
    return job, worker, launchers


def transcript(accounted):
    records = []
    for call in range(4):
        first = dict(pid=call+1, created_100ns=call+1, exited_100ns=0, user_ns=100, kernel_ns=200)
        last = dict(first, exited_100ns=call+10)
        records.append(dict(call=call, accounted=accounted, spec=subject.SPEC, stdout_bytes=23589,
            stderr_bytes=0, code=0, phases=dict(child_start=first, child_end=last) if accounted else None))
    done = dict(calls=4, accounted=accounted, git_argv=['git', 'show', subject.SPEC], stdin_hold=False,
                git_suspended=False, native_bypass=False, environment_changes=False, package_pass=False)
    return records, done


def encode(records, done):
    return ('\n'.join('HC074_GIT_CPU '+json.dumps(row) for row in records)+'\nHC074_GIT_CPU_DONE '+json.dumps(done)+
            '\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.1s\n').encode()


class Accounting(unittest.TestCase):
    def test_whole_terminal_lifetimes_not_late_deltas(self):
        result = subject.calculate(*inputs())
        self.assertEqual(result['non_launcher_descendant_cpu'], dict(user_ns=8600, kernel_ns=6200))
        self.assertTrue(result['cpu_includes_native'])
        for key in ('exact_native_image_attested', 'per_call_native_cpu', 'initial_sample_subtracted',
                    'overhead_accepted', 'speedup_established', 'package_pass'):
            self.assertFalse(result[key])

    def test_active_wrong_process_count_and_duplicate_lifetime_refuse(self):
        for key, value in (('active', 1), ('active', False), ('total', 8), ('total', 10), ('total', True)):
            job, worker, roots = inputs()
            job[key] = value
            with self.assertRaises(Refusal):
                subject.calculate(job, worker, roots)
        job, worker, roots = inputs()
        roots[1] = copy.deepcopy(roots[0])
        with self.assertRaisesRegex(Refusal, 'duplicate launcher'):
            subject.calculate(job, worker, roots)

    def test_underflow_is_refusal_not_zero_or_saturated_subtraction(self):
        for field in ('user_ns', 'kernel_ns'):
            job, worker, roots = inputs()
            job[field] = 1
            with self.assertRaisesRegex(Refusal, 'underflow'):
                subject.calculate(job, worker, roots)

    def test_missing_terminal_missing_root_invalid_counter_and_scope_refuse(self):
        for mode in ('worker-live', 'root-live', 'missing', 'bool', 'negative', 'calls'):
            job, worker, roots = inputs()
            if mode == 'worker-live': worker['terminal'] = False
            elif mode == 'root-live': roots[0]['exited_100ns'] = 0
            elif mode == 'missing': roots.pop()
            elif mode == 'bool': roots[0]['user_ns'] = True
            elif mode == 'negative': job['kernel_ns'] = -1
            with self.subTest(mode=mode), self.assertRaises(Refusal):
                subject.calculate(job, worker, roots, 3 if mode == 'calls' else 4)

    def test_default_job_does_not_install_limits_or_lifetime_policy(self):
        api = object.__new__(subject.AccountingApi)
        api.empty = Mock(return_value=True)
        with patch.object(subject.Api, 'configure') as restricted:
            api.configure(77)
            api.empty.assert_called_once_with(77)
            restricted.assert_not_called()
        api.empty.return_value = False
        with self.assertRaisesRegex(Refusal, 'not empty'):
            api.configure(77)

    def test_complete_reference_and_accounted_protocol(self):
        for accounted in (False, True):
            records, done = transcript(accounted)
            self.assertEqual(subject.rows(encode(records, done), accounted), records)

    def test_missing_duplicate_reordered_output_or_changed_argv_refuse(self):
        for mode in ('missing', 'duplicate', 'reorder', 'bytes', 'stderr', 'code', 'argv', 'hold', 'suspend', 'native', 'env'):
            records, done = transcript(True)
            if mode == 'missing': records.pop()
            elif mode == 'duplicate': records.append(records[0])
            elif mode == 'reorder': records.reverse()
            elif mode in ('bytes', 'stderr', 'code'):
                records[0][{'bytes':'stdout_bytes', 'stderr':'stderr_bytes', 'code':'code'}[mode]] += 1
            elif mode == 'argv': done['git_argv'] = ['git', '--no-replace-objects', 'show', subject.SPEC]
            else: done[{'hold':'stdin_hold', 'suspend':'git_suspended', 'native':'native_bypass', 'env':'environment_changes'}[mode]] = True
            with self.subTest(mode=mode), self.assertRaises(Refusal):
                subject.rows(encode(records, done), True)

    def test_retained_launcher_birth_pid_and_cpu_cannot_change(self):
        for key, value in (('pid', 99), ('created_100ns', 99), ('exited_100ns', 0), ('user_ns', 0), ('kernel_ns', 0)):
            records, done = transcript(True)
            records[0]['phases']['child_end'][key] = value
            with self.subTest(key=key), self.assertRaises(Refusal):
                subject.rows(encode(records, done), True)

    def test_reference_cannot_use_profile_or_incomplete_verdict(self):
        records, done = transcript(False)
        records[0]['phases'] = {}
        with self.assertRaises(Refusal):
            subject.rows(encode(records, done), False)
        records, done = transcript(True)
        with self.assertRaises(Refusal):
            subject.rows(encode(records, done).split(b'test result:')[0], True)

    def test_bad_labels_refuse_without_job_or_process(self):
        with patch.object(subject, 'WindowsJob') as job, patch.object(subject, 'AccountingApi') as api:
            for label in ('', '../outside', 'UPPER', 'x'*65):
                with self.assertRaises(Refusal):
                    subject.run(label, 'fixture')
            api.assert_not_called()
            job.assert_not_called()

    def test_both_modes_require_stable_complete_terminal_job(self):
        job, _, _ = inputs()
        subject.terminal_job(job, copy.deepcopy(job))
        for field, value in (('user_ns', 10001), ('kernel_ns', 8001), ('terminated', 1)):
            with self.subTest(field=field), self.assertRaisesRegex(Refusal, 'stable'):
                subject.terminal_job(job, dict(job, **{field: value}))
        for field, value in (('active', 1), ('active', False), ('total', 8), ('total', True)):
            bad = dict(job, **{field: value})
            with self.subTest(field=field), self.assertRaises(Refusal):
                subject.terminal_job(bad, bad)

    def test_cleanup_healthy_does_not_terminate_or_reopen(self):
        job, child = Mock(), Mock()
        job.is_empty.return_value = True
        with patch.object(subject.time, 'monotonic', return_value=1):
            self.assertEqual(subject.cleanup(job, child, 5), [])
        job.terminate.assert_not_called()
        child.wait.assert_called_once_with(timeout=4)
        child.close.assert_called_once_with()
        job.close.assert_called_once_with()

    def test_cleanup_uses_retained_child_after_partial_spawn(self):
        job = Mock()
        job.is_empty.side_effect = [False, True]
        with patch.object(subject.time, 'monotonic', return_value=1):
            self.assertEqual(subject.cleanup(job, None, 5), [])
        job.terminate.assert_called_once_with()
        job.child.wait.assert_called_once_with(timeout=4)
        job.child.close.assert_called_once_with()

    def test_cleanup_errors_cannot_skip_either_close(self):
        for failure in ('terminate', 'wait', 'child-close'):
            job, child = Mock(), Mock()
            job.is_empty.side_effect = [False, True]
            {'terminate':job.terminate, 'wait':child.wait, 'child-close':child.close}[failure].side_effect = Refusal(failure)
            job.close.side_effect = Refusal('job-close')
            with patch.object(subject.time, 'monotonic', return_value=1):
                errors = subject.cleanup(job, child, 5)
            self.assertEqual(errors, [failure, 'job-close'])
            child.close.assert_called_once_with()
            job.close.assert_called_once_with()

    def test_cleanup_late_close_refuses_without_extending_deadline(self):
        job, child = Mock(), Mock()
        job.is_empty.return_value = True
        with patch.object(subject.time, 'monotonic', side_effect=[1, 6]):
            self.assertEqual(subject.cleanup(job, child, 5), ['late cleanup witness'])
        child.wait.assert_called_once_with(timeout=4)

    def test_cleanup_live_job_is_bounded_and_still_closes_handles(self):
        job, child = Mock(), Mock()
        job.is_empty.return_value = False
        with patch.object(subject.time, 'monotonic', return_value=6):
            self.assertEqual(subject.cleanup(job, child, 5), ['cleanup deadline', 'late cleanup witness'])
        child.wait.assert_not_called()
        child.close.assert_called_once_with()
        job.close.assert_called_once_with()


if __name__ == '__main__':
    unittest.main()
