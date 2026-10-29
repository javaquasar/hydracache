"""Fake ABI differential and injected clocks only; never create a process."""
import ast
import copy
import hashlib
import inspect
import json
from pathlib import Path
import textwrap
import tomllib
import types
import unittest
from unittest.mock import Mock, patch

import capture_leaves as leaves
import git_job_cpu as accounting
import spawn_frames as subject
import watchdog_windows_job as original


class Clock:
    def __init__(self, cut=None, fatal=False):
        self.calls, self.cut, self.fatal = [], cut, fatal

    def read(self, kind):
        self.calls.append(kind)
        if len(self.calls) == self.cut:
            if self.fatal:
                raise KeyboardInterrupt('fatal spawn clock cut')
            raise OSError('ordinary spawn clock cut')
        return len(self.calls)*10

    def tape(self):
        return subject.SpawnTape(wall=lambda: self.read('wall'), cpu=lambda: self.read('cpu'),
                                 clock_domain='synthetic-control')


class Dll:
    """Model only the original ABI calls, including ctypes out parameters."""
    def __init__(self, failure=None, missing=False, std=True, close_error=False):
        self.events, self.failure, self.missing, self.std = [], failure, missing, std
        self.last_error, self.created, self.active = 0, False, False
        self.close_error = close_error
        self.next_duplicate = 100

    def event(self, name, *values):
        self.events.append((name, *values))
        if self.failure == name:
            raise original.Win32Failure('injected '+name)

    def GetCurrentProcess(self):
        self.event('current')
        return 1

    def GetStdHandle(self, number):
        self.event('std', number)
        return -number if self.std else None

    def DuplicateHandle(self, current, handle, current2, pointer, access, inherit, options):
        self.event('duplicate', current, handle, current2, access, inherit, options)
        self.next_duplicate += 4
        pointer._obj.value = self.next_duplicate
        return 1

    def GetFileType(self, handle):
        self.event('file_type', handle)
        return 1

    def InitializeProcThreadAttributeList(self, attributes, count, flags, size):
        self.event('attributes', attributes is None, count, flags)
        if attributes is None:
            size._obj.value = 256
            self.last_error = 122
            return 0
        return 1

    def UpdateProcThreadAttribute(self, attributes, flags, kind, values, size, previous, returned):
        self.event('update', flags, kind, tuple(values), size, previous, returned)
        return 1

    def CreateProcessW(self, application, line, process, thread, inherit, flags, environment, cwd, startup, info):
        self.event('create', application, line.value, process, thread, inherit, flags, environment, cwd,
                   startup._obj.info.flags, startup._obj.info.stdin, startup._obj.info.stdout, startup._obj.info.stderr)
        if self.missing:
            self.last_error = 2
            return 0
        self.created = self.active = True
        info._obj.process, info._obj.thread, info._obj.pid, info._obj.tid = 501, 502, 503, 504
        return 1

    def IsProcessInJob(self, child, job, present):
        self.event('membership', child, job)
        present._obj.value = True
        return 1

    def ResumeThread(self, thread):
        self.event('resume', thread)
        return 1

    def DeleteProcThreadAttributeList(self, attributes):
        self.event('delete_attributes')

    def CloseHandle(self, handle):
        self.events.append(('close', handle))
        if self.close_error and handle == 112:
            raise original.Win32Failure('injected duplicate close')
        return 1

    def WaitForSingleObject(self, handle, timeout):
        self.event('wait', handle, timeout)
        return 0

    def GetExitCodeProcess(self, handle, code):
        self.event('exit', handle)
        code._obj.value = 124
        return 1


class Strip(ast.NodeTransformer):
    """Only literal marks on the exact new owner field may be removed."""
    def visit_Expr(self, node):
        call = node.value
        if isinstance(call, ast.Call) and isinstance(call.func, ast.Attribute):
            receiver = call.func.value
            if isinstance(receiver, ast.Attribute) and receiver.attr == 'spawn_frames' and \
                    isinstance(receiver.value, ast.Name) and receiver.value.id in ('self', 'owner'):
                assert call.func.attr in ('begin', 'end') and not call.keywords and len(call.args) == 1
                assert isinstance(call.args[0], ast.Constant) and call.args[0].value in subject.PHASES
                return None
        return self.generic_visit(node)


class Model(unittest.TestCase):
    def test_protocol_closed_scope_and_explicit_owner_gate(self):
        doc = tomllib.loads(Path(subject.__file__).with_name('spawn-frames-protocol.toml').read_text())
        contract = subject.contract()
        self.assertEqual(doc['schema'], contract['schema'])
        self.assertEqual(doc['phases'], list(subject.PHASES))
        self.assertEqual((doc['max_additional_probes_per_worker'], doc['max_additional_clock_reads_per_worker'],
                          doc['max_recorder_clock_reads_with_outer_and_leaves']), (6, 18, 102))
        self.assertEqual((doc['bound_seconds'], doc['cleanup_seconds'], doc['free_bytes'], doc['log_bytes']),
                         (120, 5, 4*1024**3, 16*1024**2))
        for key in ('caller_connected', 'native_control_executed', 'cleanup_instrumented', 'pure_create_cost_isolated',
                    'existing_predicates_removed_or_reordered', 'window_renewed', 'overhead_accepted', 'package_pass'):
            self.assertIs(contract[key], False)
            self.assertIs(doc[key], False)
        api = Mock()
        with self.assertRaisesRegex(accounting.Refusal, 'explicit spawn frame owner'):
            subject.SpawnAccountingJob(api, Mock(), Path('.'), Mock(), Mock())
        api.create_job.assert_not_called()
        self.assertNotIn('__main__', Path(subject.__file__).read_text())

    def scenario(self, *, traced=True, clock=None, failure=None, missing=False, late=False, std=True, close_error=False):
        dll = Dll(failure, missing, std, close_error)
        clock = clock or Clock()
        trace = clock.tape()
        api = object.__new__(subject.SpawnAccountingApi if traced else accounting.AccountingApi)
        api.dll = dll
        def terminate(handle):
            dll.event('terminate_job', handle)
            dll.active = False
        api.terminate, api.empty = terminate, lambda handle: not dll.active
        job = object.__new__(subject.SpawnAccountingJob if traced else leaves.LeafAccountingJob)
        job.api, job.handle, job.attempted = api, 700, False
        job.spawn_frames, job._child = trace, None
        job.worker_identity = job.worker_start = None
        job.image = Path('target/model-worker.exe').resolve()
        job.leaves = Mock()
        job.observer = Mock()
        def identity(handle):
            dll.event('identity', handle)
            return 503, str(job.image)
        def read(handle):
            dll.event('initial', handle)
            return dict(creation_100ns=1, exit_100ns=None, terminal=False, user_100ns=0, kernel_100ns=0, io={})
        job.observer.identity.side_effect, job.observer.read.side_effect = identity, read
        result = dict(operation_passed=False, first_refusal=None, cleanup_errors=[])
        fake_msvcrt = types.SimpleNamespace(get_osfhandle=lambda descriptor: 88)
        with patch.dict('sys.modules', msvcrt=fake_msvcrt), \
             patch.object(original.c, 'get_last_error', side_effect=lambda: (dll.events.append(('last_error', dll.last_error)), dll.last_error)[1]), \
             patch.object(original.c, 'WinError', side_effect=lambda code: FileNotFoundError(code, 'original missing program')), \
             patch.object(original.time, 'monotonic', return_value=6 if late else 1) as deadline:
            try:
                # These arguments, not a replacement subprocess mechanism, are identical.
                job.spawn([str(job.image), 'space value', 'quote"value', '\u03a9', 'tail\\'], Path('target'),
                          Mock(fileno=lambda: 9) if std else None, 5)
                result['operation_passed'] = True
            except BaseException as error:
                result['first_refusal'] = f'{type(error).__name__}: {error}'
            result['spawn_events'] = list(dll.events)
            result['deadline_reads_before_cleanup'] = deadline.call_count
            if job.child is not None:
                # Exercise the existing cleanup, including the caller's child=None case.
                result['cleanup_errors'] = accounting.cleanup(job, None, 6 if late else 5)
        result['trace'] = trace.finish(result['operation_passed'])
        result['all_events'] = list(dll.events)
        return result, job, clock

    def test_restricted_ast_preserves_original_api_and_retained_setter(self):
        for a, b in ((original.Api.spawn, subject.SpawnAccountingApi.spawn),
                     (leaves.LeafAccountingJob.child.fset, subject.SpawnAccountingJob.child.fset)):
            old, new = (ast.parse(textwrap.dedent(inspect.getsource(f))).body[0] for f in (a, b))
            new.decorator_list = old.decorator_list
            self.assertEqual(ast.dump(old), ast.dump(Strip().visit(new)))

    def test_original_caller_ci_and_worker_bytes_match_retained_native_input(self):
        root = Path(__file__).resolve().parents[2]
        raw = (root/'docs/testing/performance/0.74/local-runs/capture-leaves-native-20261026/index.json').read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(),
                         '926f30f0eca6ae708b3706976f256a9ca52a3a5d7e9a9c447a8f45cec59a0855')
        index = json.loads(raw)
        for path in ('scripts/ci/watchdog_windows_job.py', 'tools/xtask-package-attribution-074/git_cpu_overhead.py',
                     'tools/xtask-package-attribution-074/capture_leaves.py',
                     'crates/xtask/tests/git_descendant_cpu_074.rs'):
            self.assertEqual(hashlib.sha256((root/path).read_bytes()).hexdigest(), index['members']['source/'+path]['sha256'])

    def test_differential_healthy_argv_handles_deadline_custody_and_cleanup(self):
        old, new = self.scenario(traced=False), self.scenario()
        for key in ('operation_passed', 'first_refusal', 'cleanup_errors', 'spawn_events', 'all_events', 'deadline_reads_before_cleanup'):
            self.assertEqual(old[0][key], new[0][key])
        self.assertTrue(new[0]['trace']['complete'])
        self.assertEqual(len(new[2].calls), 18)
        self.assertEqual(new[0]['deadline_reads_before_cleanup'], 1)
        self.assertIsNotNone(new[1].child)
        self.assertIsNone(new[1].child.handle)

    def test_all_18_ordinary_clock_faults_keep_original_effects_and_cleanup(self):
        expected = self.scenario(traced=False)[0]
        for cut in range(1, 19):
            with self.subTest(cut=cut):
                result, job, clock = self.scenario(clock=Clock(cut))
                self.assertTrue(result['operation_passed'])
                self.assertIsNone(result['first_refusal'])
                self.assertEqual(result['all_events'], expected['all_events'])
                self.assertEqual(result['cleanup_errors'], [])
                self.assertFalse(result['trace']['capture_valid'])
                self.assertIn('ordinary spawn clock cut', result['trace']['first_refusal'])
                self.assertEqual(len(clock.calls), cut)
                self.assertIsNotNone(job.child)

    def test_all_18_fatal_cuts_never_lose_a_created_child(self):
        for cut in range(1, 19):
            with self.subTest(cut=cut):
                result, job, clock = self.scenario(clock=Clock(cut, fatal=True))
                self.assertFalse(result['operation_passed'])
                self.assertIn('KeyboardInterrupt: fatal spawn clock cut', result['first_refusal'])
                self.assertFalse(result['trace']['complete'])
                self.assertEqual(len(clock.calls), cut)
                self.assertEqual(result['cleanup_errors'], [])
                created = any(e[0] == 'create' for e in result['spawn_events'])
                self.assertEqual(created, cut >= 10)
                self.assertEqual(job.child is not None, created)
                if created:
                    self.assertIn(('terminate_job', 700), result['all_events'])
                    self.assertIsNone(job.child.handle)
                    self.assertIsNone(job.child.thread)
                self.assertEqual(any(e[0] == 'resume' for e in result['spawn_events']), cut >= 16)

    def test_missing_program_last_error_is_consumed_before_any_new_observation(self):
        for traced in (False, True):
            result, job, clock = self.scenario(traced=traced, missing=True)
            self.assertIn('FileNotFoundError', result['first_refusal'])
            events = result['spawn_events']
            create = next(i for i, e in enumerate(events) if e[0] == 'create')
            self.assertEqual(events[create+1], ('last_error', 2))
            self.assertIsNone(job.child)
            self.assertFalse(result['trace']['complete'])
            self.assertFalse(any(e[0] in ('membership', 'identity', 'resume') for e in events))
            if traced:
                self.assertEqual(len(clock.calls), 9)

    def test_operational_faults_still_refuse_at_exact_original_step(self):
        for failure in ('current', 'duplicate', 'attributes', 'update', 'create', 'identity', 'initial', 'membership', 'resume'):
            with self.subTest(failure=failure):
                old = self.scenario(traced=False, failure=failure)[0]
                new = self.scenario(failure=failure)[0]
                for key in ('operation_passed', 'first_refusal', 'spawn_events', 'all_events', 'cleanup_errors', 'deadline_reads_before_cleanup'):
                    self.assertEqual(old[key], new[key])
                self.assertFalse(new['operation_passed'])
                self.assertIn('injected '+failure, new['first_refusal'])
                self.assertFalse(new['trace']['complete'])

    def test_operational_refusal_and_clock_refusal_remain_separate(self):
        result, _, _ = self.scenario(clock=Clock(1), failure='membership')
        self.assertIn('injected membership', result['first_refusal'])
        self.assertNotIn('spawn clock cut', result['first_refusal'])
        self.assertIn('ordinary spawn clock cut', result['trace']['first_refusal'])

    def test_partial_stdio_cleanup_and_original_error_composition_stay_identical(self):
        old = self.scenario(traced=False, failure='membership', close_error=True)[0]
        new = self.scenario(failure='membership', close_error=True)[0]
        for key in ('first_refusal', 'spawn_events', 'all_events', 'cleanup_errors'):
            self.assertEqual(old[key], new[key])
        self.assertIn('injected membership; release stdio duplicates: injected duplicate close', new['first_refusal'])

    def test_expired_original_deadline_never_resumes_even_with_complete_tape(self):
        old, new = (self.scenario(traced=mode, late=True)[0] for mode in (False, True))
        self.assertEqual(old['spawn_events'], new['spawn_events'])
        self.assertEqual(new['deadline_reads_before_cleanup'], 1)
        self.assertFalse(any(e[0] == 'resume' for e in new['spawn_events']))
        self.assertTrue(new['trace']['complete'])
        self.assertTrue(new['trace']['contract']['complete_frame_does_not_prove_resume'])
        self.assertFalse(new['trace']['contract']['native_control_executed'])

    def test_no_standard_handles_path_is_unchanged(self):
        old, new = (self.scenario(traced=mode, std=False)[0] for mode in (False, True))
        self.assertEqual(old['all_events'], new['all_events'])
        self.assertFalse(any(e[0] == 'duplicate' for e in new['spawn_events']))
        self.assertEqual(next(e for e in new['spawn_events'] if e[0] == 'create')[5], False)

    def test_none_publication_and_first_fatal_retention_probe(self):
        clock = Clock(10, fatal=True)
        api, observer = Mock(), Mock()
        job = subject.SpawnAccountingJob(api, observer, Path('.').resolve(), Mock(), clock.tape())
        self.assertEqual(clock.calls, [])
        self.assertIsNone(job.child)
        trace = job.spawn_frames
        trace.begin('preparation')
        trace.end('preparation')
        trace.begin('create_to_retention')
        child = Mock()
        with self.assertRaises(KeyboardInterrupt):
            job.child = child
        self.assertIs(job.child, child)
        observer.identity.assert_not_called()
        with self.assertRaisesRegex(accounting.Refusal, 'cannot be replaced'):
            job.child = Mock()

    def test_first_fault_freezes_reads_and_partial_export_has_no_invented_end(self):
        clock = Clock(1)
        trace = clock.tape()
        trace.begin('preparation')
        trace.end('preparation')
        trace.begin('create_to_retention')
        exported = trace.finish(False)
        self.assertEqual(len(clock.calls), 1)
        self.assertIsNone(exported['spans'][0]['end_probe'])
        self.assertIsNone(exported['spans'][0]['elapsed'])
        original_export = copy.deepcopy(exported)
        trace.begin('membership_release')
        self.assertEqual(exported, original_export)

    def test_fixed_schedule_clock_domains_types_and_regressions_refuse(self):
        for kwargs in (dict(wall=lambda: 0), dict(clock_domain='foreign')):
            with self.assertRaises(accounting.Refusal):
                subject.SpawnTape(**kwargs)
        for value in (True, -1, 1.5):
            for kind in ('wall', 'cpu'):
                trace = subject.SpawnTape(wall=lambda: value if kind == 'wall' else 0,
                    cpu=lambda: value if kind == 'cpu' else 0, clock_domain='synthetic-control')
                trace.begin('preparation')
                self.assertFalse(trace.finish(False)['capture_valid'])
        trace = Clock().tape()
        trace.begin('membership_release')
        self.assertIn('unexpected leaf begin', trace.finish(False)['first_refusal'])
        for kind in ('wall', 'cpu'):
            clock = Clock()
            def read(k):
                result = clock.read(k)
                return 0 if len(clock.calls) == (4 if kind == 'wall' else 5) else result
            trace = subject.SpawnTape(wall=lambda: read('wall'), cpu=lambda: read('cpu'), clock_domain='synthetic-control')
            trace.begin('preparation')
            trace.end('preparation')
            self.assertIn('regressed', trace.finish(False)['first_refusal'])


if __name__ == '__main__':
    unittest.main()
