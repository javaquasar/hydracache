"""Deterministic CI-watchdog faults; no child launch or host mutation."""
import importlib.util
import itertools
import math
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from watchdog_windows_job import WindowsJob, Win32Failure

SPEC = importlib.util.spec_from_file_location('watchdog_074', Path(__file__).with_name('run-with-heartbeat.py'))
watch = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(watch)


class Child:
    pid = 1234

    def __init__(self, code=0, fail=None):
        self.code = code
        self.fail = fail
        self.events = []

    def poll(self):
        return self.code

    def wait(self, timeout=None):
        self.events.append(('wait', timeout))
        if self.fail == 'wait':
            raise RuntimeError('injected child wait failure')
        return self.code

    def close(self):
        self.events.append(('close',))


class Job:
    def __init__(self, child, fail=None):
        self.child = child
        self.fail = fail
        self.events = []
        self.active = True

    def spawn(self, command, cwd, log_handle, deadline):
        self.events.append(('spawn', command, cwd, log_handle, deadline))
        if self.fail == 'spawn':
            raise RuntimeError('injected spawn failure')
        return self.child

    def is_empty(self):
        self.events.append(('query',))
        if self.fail == 'query':
            raise RuntimeError('injected query failure')
        return not self.active

    def terminate(self):
        self.events.append(('terminate',))
        if self.fail == 'terminate':
            raise RuntimeError('injected terminate failure')
        if self.fail != 'busy':
            self.active = False

    def close(self):
        self.events.append(('close',))
        if self.fail == 'close':
            raise RuntimeError('injected job close failure')


@unittest.skipUnless(watch.os.name == 'nt', 'Windows cleanup state machine')
class StateMachine(unittest.TestCase):
    def run_case(self, code=0, fail=None):
        child = Child(code, fail)
        job = Job(child, fail)
        if fail in ['empty', 'late-empty', 'late-close']:
            job.active = False
        args = SimpleNamespace(command=['fixture'], timeout_seconds=0.35, heartbeat_seconds=0.05,
            status_json=Path('unused-receipt'), log_file=None, attempt_id='unit', cwd=None)
        receipts = []
        ticks = itertools.count(0.0, 6 if fail == 'late-empty' else 0.05)

        def clock():
            if fail == 'late-close' and job.events and job.events[-1] == ('close',):
                return 20
            return next(ticks)
        with patch.object(watch, 'parse_args', return_value=args), \
             patch.object(watch, 'WindowsJob', return_value=job, create=True), \
             patch.object(watch.subprocess, 'Popen', return_value=child), \
             patch.object(watch.time, 'monotonic', side_effect=clock), \
             patch.object(watch.time, 'sleep'), \
             patch.object(watch, 'write_receipt', side_effect=lambda _, r: receipts.append(r)), \
             patch('builtins.print'):
            result = watch.main()
        self.assertEqual(len(receipts), 1)
        return result, receipts[0], job, child

    def test_successful_root_with_live_descendant_reaches_original_deadline(self):
        result, receipt, job, child = self.run_case()
        self.assertEqual(result, 124)
        self.assertEqual(receipt['classification'], 'timeout')
        self.assertIn(('terminate',), job.events)
        self.assertLess(job.events.index(('terminate',)), job.events.index(('close',)))
        self.assertEqual(job.events[0][-1], 0.35)
        self.assertIn(('close',), child.events)

    def test_failed_root_still_terminates_descendants(self):
        result, receipt, job, _ = self.run_case(code=23)
        self.assertEqual(result, 23)
        self.assertEqual(receipt['classification'], 'product-failure')
        self.assertIn(('terminate',), job.events)
        self.assertIn(('close',), job.events)

    def test_termination_failure_is_runner_loss_not_timeout(self):
        result, receipt, job, _ = self.run_case(fail='terminate')
        self.assertEqual(result, 125)
        self.assertEqual(receipt['classification'], 'runner-loss')
        self.assertIn('terminate failure', receipt['detail'])
        self.assertEqual(job.events[-1], ('close',))

    def test_query_failure_is_runner_loss_and_closes_job(self):
        result, receipt, job, _ = self.run_case(fail='query')
        self.assertEqual(result, 125)
        self.assertEqual(receipt['classification'], 'runner-loss')
        self.assertIn('query failure', receipt['detail'])
        self.assertEqual(job.events[-1], ('close',))

    def test_spawn_failure_closes_empty_owner(self):
        result, receipt, job, _ = self.run_case(fail='spawn')
        self.assertEqual(result, 125)
        self.assertIn('spawn failure', receipt['detail'])
        self.assertEqual(job.events[-1], ('close',))

    def test_empty_job_and_successful_root_do_not_trigger_termination(self):
        result, receipt, job, _ = self.run_case(fail='empty')
        self.assertEqual(result, 0)
        self.assertEqual(receipt['classification'], 'success')
        self.assertNotIn(('terminate',), job.events)

    def test_original_child_wait_failure_is_runner_loss(self):
        result, receipt, _, _ = self.run_case(fail='wait')
        self.assertEqual(result, 125)
        self.assertIn('child wait failure', receipt['detail'])

    def test_job_close_failure_cannot_be_a_successful_receipt(self):
        result, receipt, _, _ = self.run_case(fail='close')
        self.assertEqual(result, 125)
        self.assertIn('job close failure', receipt['detail'])

    def test_busy_job_exhausts_fixed_cleanup_budget_and_refuses(self):
        result, receipt, _, _ = self.run_case(fail='busy')
        self.assertEqual(result, 125)
        self.assertIn('exceeded five seconds', receipt['detail'])

    def test_empty_witness_after_cleanup_budget_is_not_success(self):
        result, receipt, _, _ = self.run_case(fail='late-empty')
        self.assertEqual(result, 125)
        self.assertIn('exceeded five seconds', receipt['detail'])

    def test_close_after_cleanup_budget_is_not_success(self):
        result, receipt, _, _ = self.run_case(fail='late-close')
        self.assertEqual(result, 125)
        self.assertIn('exceeded five seconds', receipt['detail'])


class Arguments(unittest.TestCase):
    def test_nonfinite_time_arguments_cannot_disable_deadline(self):
        for name in ['--timeout-seconds', '--heartbeat-seconds']:
            for value in ['nan', 'inf', '-inf']:
                argv = ['watchdog', '--timeout-seconds', '1', '--heartbeat-seconds', '1',
                        name + '=' + value, '--status-json', 'unused', '--', 'fixture']
                with self.subTest(name=name, value=value), patch.object(watch.sys, 'argv', argv), \
                     patch.object(watch.sys, 'stderr'), self.assertRaises(SystemExit):
                    watch.parse_args()

    def test_fractional_deadline_is_not_rounded(self):
        with patch.object(watch.sys, 'argv', ['watchdog', '--timeout-seconds', '0.35',
                    '--status-json', 'unused', '--', 'fixture']):
            self.assertEqual(watch.parse_args().timeout_seconds, 0.35)
        self.assertTrue(math.isfinite(0.35))


class FakeApi:
    def __init__(self, fail=None):
        self.fail, self.events = fail, []

    def call(self, name):
        self.events.append(name)
        if name == self.fail:
            raise Win32Failure('injected ' + name)

    def create_job(self):
        self.call('create')
        return 123

    def configure(self, handle):
        self.call('configure')

    def close(self, handle):
        self.call('close')

    def spawn(self, owner, *args):
        self.call('spawn')
        owner.child = Child()
        return owner.child

    def empty(self, handle):
        self.call('empty')
        return True

    def terminate(self, handle):
        self.call('terminate')


class Ownership(unittest.TestCase):
    def test_configuration_failure_releases_created_job(self):
        api = FakeApi('configure')
        with self.assertRaisesRegex(Win32Failure, 'configure'):
            WindowsJob(api)
        self.assertEqual(api.events, ['create', 'configure', 'close'])

    def test_creation_failure_does_not_close_an_unowned_handle(self):
        api = FakeApi('create')
        with self.assertRaisesRegex(Win32Failure, 'create'):
            WindowsJob(api)
        self.assertEqual(api.events, ['create'])

    def test_one_spawn_attempt_even_after_failure(self):
        api = FakeApi('spawn')
        job = WindowsJob(api)
        with self.assertRaisesRegex(Win32Failure, 'spawn'):
            job.spawn(['fixture'], None, None, 0.35)
        with self.assertRaisesRegex(Win32Failure, 'already-used'):
            job.spawn(['fixture'], None, None, 0.35)
        job.close()
        self.assertEqual(api.events.count('spawn'), 1)

    def test_closed_owner_cannot_spawn_query_or_terminate(self):
        api = FakeApi()
        job = WindowsJob(api)
        job.close()
        for operation in [lambda: job.spawn(['fixture'], None, None, 0.35), job.is_empty, job.terminate]:
            with self.assertRaises(Win32Failure):
                operation()
        job.close()
        self.assertEqual(api.events, ['create', 'configure', 'close'])

    def test_close_failure_is_not_silently_a_closed_owner(self):
        api = FakeApi('close')
        job = WindowsJob(api)
        with self.assertRaisesRegex(Win32Failure, 'close'):
            job.close()
        self.assertEqual(job.handle, 123)

    def test_successful_owner_preserves_child_and_passes_queries(self):
        api = FakeApi()
        job = WindowsJob(api)
        child = job.spawn(['fixture'], None, None, 0.35)
        self.assertIs(child, job.child)
        self.assertTrue(job.is_empty())
        job.terminate()
        job.close()
        self.assertEqual(api.events, ['create', 'configure', 'spawn', 'empty', 'terminate', 'close'])


if __name__ == '__main__':
    unittest.main()
