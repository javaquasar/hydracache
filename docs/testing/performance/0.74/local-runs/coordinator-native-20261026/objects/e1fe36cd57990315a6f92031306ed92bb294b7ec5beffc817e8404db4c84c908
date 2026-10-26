"""Deterministic observer-cost controls; no process or Git dispatch."""
import copy
import json
from pathlib import Path
import tempfile
import tomllib
import unittest
from unittest.mock import Mock, patch

import git_cpu_overhead as subject
from owned_process_observer import Refusal
from test_git_job_cpu import encode, transcript


def attempts():
    return [dict(block=b, slot=s, mode=m, passed=True, first_refusal=None, cleanup_errors=[],
        metrics=dict(wall_ns=100 if m == subject.REFERENCE else 120,
            whole_job_cpu_ns=80 if m == subject.REFERENCE else 90,
            whole_worker_cpu_ns=10 if m == subject.REFERENCE else 15,
            coordinator_cpu_ns=4 if m == subject.REFERENCE else 3)) for b, s, m in subject.schedule()]


class Model(unittest.TestCase):
    def test_balanced_fixed_schedule(self):
        schedule = subject.schedule()
        self.assertEqual(len(schedule), 16)
        for block in range(4):
            names = [m for b, _, m in schedule if b == block]
            self.assertEqual(names, [subject.REFERENCE, subject.PROFILE, subject.PROFILE, subject.REFERENCE]
                             if block % 2 == 0 else [subject.PROFILE, subject.REFERENCE, subject.REFERENCE, subject.PROFILE])
        self.assertEqual(sum(4 for _ in schedule), 64)

    def test_protocol_constants_and_closed_acceptance(self):
        doc = tomllib.loads(Path(subject.__file__).with_name('git-cpu-overhead-protocol.toml').read_text())
        self.assertEqual((doc['blocks'], doc['workers_per_block'], doc['calls_per_worker'], doc['git_calls']), (4, 4, 4, 64))
        self.assertEqual((doc['bound_seconds'], doc['cleanup_seconds']), (subject.BOUND, subject.CLEANUP))
        self.assertEqual((doc['free_bytes'], doc['log_bytes']), (subject.FREE_BYTES, subject.LOG_BYTES))
        for name in ('job_overhead_measured', 'pure_counter_api_cost_isolated', 'overhead_accepted',
                     'speedup_established', 'package_or_qualification', 'retry', 'git_suspended',
                     'stdin_hold', 'native_bypass', 'environment_changes', 'server_services_clocks'):
            self.assertIs(doc[name], False)

    def test_exact_block_sums_and_signed_differences_not_admission(self):
        result = subject.summarize(attempts())
        self.assertEqual((result['workers'], result['git_calls']), (16, 64))
        for block in result['blocks']:
            self.assertEqual(block['metrics']['wall_ns'], dict(reference_sum=200, profile_sum=240,
                delta=40, ratio_numerator=240, ratio_denominator=200, unavailable_reason=None))
            self.assertEqual(block['metrics']['coordinator_cpu_ns']['delta'], -2)
        for name in ('job_overhead_measured', 'pure_counter_api_cost_isolated', 'overhead_accepted',
                     'speedup_established', 'package_pass'):
            self.assertIs(result[name], False)

    def test_zero_cpu_is_not_dropped_or_an_infinite_ratio(self):
        data = attempts()
        for row in data:
            if row['mode'] == subject.REFERENCE:
                row['metrics']['coordinator_cpu_ns'] = 0
        for block in subject.summarize(data)['blocks']:
            metric = block['metrics']['coordinator_cpu_ns']
            self.assertEqual((metric['delta'], metric['ratio_numerator'], metric['ratio_denominator'],
                              metric['unavailable_reason']), (6, None, None, 'zero reference counter'))

    def test_missing_reordered_failed_or_dirty_cohort_refuses(self):
        for mode in ('missing', 'extra', 'order', 'failed', 'refusal', 'cleanup'):
            data = attempts()
            if mode == 'missing': data.pop()
            elif mode == 'extra': data.append(copy.deepcopy(data[0]))
            elif mode == 'order': data[0], data[1] = data[1], data[0]
            elif mode == 'failed': data[0]['passed'] = False
            elif mode == 'refusal': data[0]['first_refusal'] = 'first'
            else: data[0]['cleanup_errors'] = ['close']
            with self.subTest(mode=mode), self.assertRaises(Refusal):
                subject.summarize(data)

    def test_invalid_metrics_refuse_without_coercion(self):
        for value in (-1, True, 1.5):
            data = attempts()
            data[0]['metrics']['coordinator_cpu_ns'] = value
            with self.subTest(value=value), self.assertRaises(Refusal):
                subject.summarize(data)
        for mode in ('wall-zero', 'missing', 'worker-exceeds-job'):
            data = attempts()
            if mode == 'wall-zero': data[0]['metrics']['wall_ns'] = 0
            elif mode == 'missing': data[0]['metrics'].pop('wall_ns')
            else: data[0]['metrics']['whole_worker_cpu_ns'] = 81
            with self.subTest(mode=mode), self.assertRaises(Refusal):
                subject.summarize(data)

    def frame(self, folder, mode):
        api, observer, job = Mock(), Mock(), Mock()
        image = folder/'worker.exe'
        job.handle = 77
        job.worker_identity = (42, str(image))
        job.worker_start = dict(terminal=False, creation_100ns=1, exit_100ns=None,
                                user_100ns=0, kernel_100ns=0, io={})
        child = job.child
        child.pid, child.handle = 42, 123
        child.wait.return_value = 0
        def spawn(argv, cwd, log, deadline):
            log.write(encode(*transcript(mode == subject.PROFILE)))
            return child
        job.spawn.side_effect = spawn
        observer.dll.GetProcessId.return_value = 42
        observer.read.return_value = dict(job.worker_start, terminal=True, exit_100ns=20,
                                          user_100ns=10, kernel_100ns=10)
        zero = dict(user_ns=0, kernel_ns=0, total=0, active=0, terminated=0)
        terminal = dict(user_ns=10000, kernel_ns=8000, total=9, active=0, terminated=0)
        api.counters.side_effect = [zero, terminal, copy.deepcopy(terminal)]
        return api, observer, job, image

    def test_actual_frame_preserves_protocol_checks_and_healthy_close(self):
        for mode in (subject.REFERENCE, subject.PROFILE):
            with tempfile.TemporaryDirectory() as directory:
                folder = Path(directory)
                api, observer, job, image = self.frame(folder, mode)
                with patch.object(subject, 'AccountingJob', return_value=job), \
                     patch.object(subject.time, 'monotonic', return_value=1), \
                     patch.object(subject.time, 'monotonic_ns', side_effect=[100, 300]), \
                     patch.object(subject.time, 'process_time_ns', side_effect=[10, 20]), \
                     patch.object(subject, 'cleanup') as cleanup:
                    result = subject.measure(api, observer, image, 0, 0, mode, folder, 5)
                self.assertTrue(result['passed'])
                self.assertEqual(result['metrics'], dict(wall_ns=200, coordinator_cpu_ns=10,
                    whole_job_cpu_ns=18000, whole_worker_cpu_ns=2000))
                self.assertEqual(job.spawn.call_args.args[0], [str(image), '--exact', mode, '--nocapture', '--test-threads=1'])
                self.assertEqual(job.spawn.call_args.args[-1], 5)
                job.child.close.assert_called_once_with()
                job.close.assert_called_once_with()
                cleanup.assert_not_called()
                observer.open.assert_not_called()

    def test_expired_origin_cannot_create_job(self):
        with patch.object(subject.time, 'monotonic', return_value=6), patch.object(subject, 'AccountingJob') as job:
            result = subject.measure(None, None, None, 0, 0, subject.REFERENCE, Path('.'), 5)
        self.assertIn('started too late', result['first_refusal'])
        job.assert_not_called()

    def test_late_frame_after_close_keeps_first_refusal(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            api, observer, job, image = self.frame(folder, subject.REFERENCE)
            with patch.object(subject, 'AccountingJob', return_value=job), \
                 patch.object(subject.time, 'monotonic', side_effect=[1, 1, 1, 6]):
                result = subject.measure(api, observer, image, 0, 0, subject.REFERENCE, folder, 5)
            self.assertFalse(result['passed'])
            self.assertIn('late worker frame witness', result['first_refusal'])
            job.close.assert_called_once_with()

    def test_counter_pid_and_byte_faults_preserve_first_refusal_and_cleanup(self):
        for fault in ('pid', 'counter', 'bytes', 'exit'):
            with tempfile.TemporaryDirectory() as directory:
                folder = Path(directory)
                api, observer, job, image = self.frame(folder, subject.REFERENCE)
                if fault == 'pid': observer.dll.GetProcessId.return_value = 43
                elif fault == 'counter':
                    api.counters.side_effect = [dict(user_ns=0, kernel_ns=0, total=0, active=0, terminated=0),
                        dict(user_ns=1, kernel_ns=2, total=9, active=0), dict(user_ns=2, kernel_ns=2, total=9, active=0)]
                elif fault == 'bytes':
                    def bad(argv, cwd, log, deadline):
                        records, done = transcript(False)
                        records[0]['stdout_bytes'] += 1
                        log.write(encode(records, done))
                        return job.child
                    job.spawn.side_effect = bad
                else: job.child.wait.return_value = 7
                with patch.object(subject, 'AccountingJob', return_value=job), \
                     patch.object(subject.time, 'monotonic', return_value=1), \
                     patch.object(subject, 'cleanup', return_value=['cleanup-second']) as cleanup:
                    result = subject.measure(api, observer, image, 0, 0, subject.REFERENCE, folder, 5)
                self.assertFalse(result['passed'])
                self.assertNotIn('cleanup-second', result['first_refusal'])
                self.assertEqual(result['cleanup_errors'], ['cleanup-second'])
                cleanup.assert_called_once_with(job, job.child, 6)

    def test_partial_spawn_custody_is_passed_to_bounded_cleanup(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            api, observer, job, image = self.frame(folder, subject.REFERENCE)
            job.spawn.side_effect = Refusal('pre-resume refusal')
            with patch.object(subject, 'AccountingJob', return_value=job), \
                 patch.object(subject.time, 'monotonic', return_value=1), \
                 patch.object(subject, 'cleanup', return_value=[]) as cleanup:
                result = subject.measure(api, observer, image, 0, 0, subject.REFERENCE, folder, 5)
            self.assertFalse(result['passed'])
            self.assertIn('pre-resume refusal', result['first_refusal'])
            cleanup.assert_called_once_with(job, None, 6)
            self.assertIsNotNone(job.child)

    def test_normal_close_failure_cannot_be_promoted(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            api, observer, job, image = self.frame(folder, subject.REFERENCE)
            job.child.close.side_effect = Refusal('child close failure')
            with patch.object(subject, 'AccountingJob', return_value=job), \
                 patch.object(subject.time, 'monotonic', return_value=1), \
                 patch.object(subject, 'cleanup', return_value=['cleanup failure']) as cleanup:
                result = subject.measure(api, observer, image, 0, 0, subject.REFERENCE, folder, 5)
            self.assertFalse(result['passed'])
            self.assertEqual(result['first_refusal'], 'Refusal: child close failure')
            self.assertEqual(result['cleanup_errors'], ['cleanup failure'])
            cleanup.assert_called_once_with(job, job.child, 6)

    def test_missing_independent_pin_cannot_start_worker(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(subject, 'ROOT', Path(directory)), \
             patch.object(subject, 'pins', side_effect=Refusal('independent pins refused')), \
             patch.object(subject, 'measure') as measure, patch.object(subject, 'AccountingApi') as api:
            result = subject.run('build', 'first')
            self.assertEqual(result['first_refusal'], 'Refusal: independent pins refused')
            self.assertEqual(result['attempts'], [])
            measure.assert_not_called()
            api.assert_not_called()

    def orchestration(self, folder):
        image = folder/'worker.exe'
        image.write_bytes(b'fixture')
        fake_gate = Mock()
        fake_gate.environment_preflight.return_value = {}
        return image, fake_gate

    def test_cohort_first_failure_stops_without_retry_or_summary(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            image, gate = self.orchestration(folder)
            failure = dict(block=0, slot=0, mode=subject.REFERENCE, passed=False,
                           first_refusal='original failure', cleanup_errors=['second'])
            with patch.object(subject, 'ROOT', folder), patch.object(subject, 'pins', return_value={}), \
                 patch.dict('sys.modules', package_effects_review_v6=gate), \
                 patch.object(subject.shutil, 'disk_usage', return_value=Mock(free=subject.FREE_BYTES)), \
                 patch.object(subject, 'artifact', return_value=(image, dict(source={}, head='head'))), \
                 patch.object(subject, 'AccountingApi'), patch.object(subject, 'WindowsApi'), \
                 patch.object(subject, 'measure', return_value=failure) as measure:
                result = subject.run('build', 'first')
            self.assertFalse(result['passed'])
            self.assertIn('original failure', result['first_refusal'])
            self.assertEqual(result['attempts'], [failure])
            self.assertNotIn('analysis', result)
            measure.assert_called_once()
            persisted = json.loads((folder/'target/git-cpu-overhead-20261026/first/result.json').read_bytes())
            self.assertEqual(result, persisted)

    def test_cohort_does_not_renew_origin_or_promote_admission(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            image, gate = self.orchestration(folder)
            with patch.object(subject, 'ROOT', folder), patch.object(subject, 'pins', return_value={}), \
                 patch.dict('sys.modules', package_effects_review_v6=gate), \
                 patch.object(subject.shutil, 'disk_usage', return_value=Mock(free=subject.FREE_BYTES)), \
                 patch.object(subject, 'artifact', return_value=(image, dict(source={}, head='head'))), \
                 patch.object(subject, 'AccountingApi'), patch.object(subject, 'WindowsApi'), \
                 patch.object(subject.subprocess, 'check_output', return_value='head'), \
                 patch.object(subject.time, 'monotonic', return_value=1), \
                 patch.object(subject, 'measure', side_effect=attempts()) as measure:
                result = subject.run('build', 'first')
            self.assertTrue(result['passed'])
            self.assertEqual(measure.call_count, 16)
            self.assertEqual({call.args[-1] for call in measure.call_args_list}, {121})
            self.assertFalse(result['overhead_accepted'])
            self.assertFalse(result['package_pass'])

    def test_bad_labels_cannot_dispatch(self):
        with patch.object(subject, 'AccountingApi') as api, patch.object(subject, 'measure') as measure:
            for label in ('', '../escape', 'X', 'x'*65):
                with self.assertRaises(Refusal):
                    subject.run('build', label)
            api.assert_not_called()
            measure.assert_not_called()


if __name__ == '__main__':
    unittest.main()
