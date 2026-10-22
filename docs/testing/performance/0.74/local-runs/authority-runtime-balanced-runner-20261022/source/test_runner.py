"""Synthetic-only runner controls. No native image, compiler or WSL is launched."""
import contextlib
import copy
import io
import itertools
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import balanced as m
import runner as r
import evaluate as e


def images():
    return {lane: {role: dict(path=f'/owned/target-{lane}-{role}/image', compiler=compiler,
                   sha256=m.common.sha_bytes((lane + role).encode())) for role in m.ROLE.values()}
            for lane, compiler in m.COMPILERS.items()}


def fixture():
    # Reuse immutable logical metadata only; replace every measured cost below.
    raw = (m.history.PACKET / 'measured/194-pair-0-baseline.stdout.log').read_text()
    header, rows = m.cpu_review.parse(raw)
    cost = dict(wall_ns=1250, cpu_ns=1000, user_ns=0, system_ns=0,
                voluntary=0, involuntary=0, inblock=0, oublock=0)
    header['empty'] = [dict(cost) for _ in range(8)]
    for row in rows:
        if row['mode'] == 'cycle': row['cycle'] = dict(cost)
        else:
            for phase in ('open', 'warm', 'refusal', 'sticky', 'drop', 'publication'):
                if row[phase] is not None:
                    row[phase] = {key: value * (32 if phase == 'warm' else 1) for key, value in cost.items()}
    return header, rows


def render(header, rows):
    return ('running 1 test\ntest ' + m.cpu.MEASURE + ' ... HC074-RUNTIME-CPU-CLOCKS ' + json.dumps(header) + '\n' +
            '\n'.join('HC074-RUNTIME-CPU ' + json.dumps(row) for row in rows) +
            '\nok\ntest result: ok. 1 passed; 0 failed; 0 ignored;\n').encode()


class Fake:
    mode = 'synthetic'

    def __init__(self, fail=None, at=0):
        self.fail = fail; self.at = at; self.calls = []; self.checked = 0; self.snapshots = 0
        self.seconds = 100.0; self.raw = 1000; self.timeouts = []
        self.contexts = 0
        self.header, self.rows = fixture()

    def monotonic(self): return self.seconds

    def stable(self):
        self.snapshots += 1
        if self.fail == 'context' and len(self.calls) == self.at and self.snapshots % 2 == 1:
            return dict(boot='different')
        return dict(boot='same')

    def verify_image(self, image):
        self.checked += 1
        if self.fail == 'image' and len(self.calls) == self.at and self.checked % 2 == 1: raise ValueError('Image replaced')

    def context(self):
        self.contexts += 1
        self.raw += 10
        value = r.optional_context(0, read=lambda path: 'synthetic only', clock=self.tick)
        if self.fail == 'deadline' and len(self.calls) == self.at and self.contexts % 2 == 1:
            self.seconds += m.COHORT_SECONDS
        return value

    def tick(self):
        self.raw += 1
        return self.raw

    def invoke(self, ordinal, row, image, timeout):
        self.calls.append(copy.deepcopy(row)); self.timeouts.append(timeout)
        if self.fail == 'timeout' and ordinal == self.at: raise TimeoutError('synthetic child timeout')
        if self.fail == 'interrupt' and ordinal == self.at: raise KeyboardInterrupt('synthetic interrupt')
        header = copy.deepcopy(self.header); rows = copy.deepcopy(self.rows)
        if self.fail == 'corpus' and ordinal == self.at: rows[0]['corpus'] = 'b' * 64
        if self.fail == 'semantic' and ordinal == self.at: rows[0]['scratch_cleared'] = False
        if self.fail == 'overrun' and ordinal == self.at: self.seconds += m.COHORT_SECONDS
        self.seconds += 0.5
        process = dict(pid=ordinal + 100, start_raw_ns=self.tick(), spawned_raw_ns=self.tick(), exit_raw_ns=self.tick())
        if self.fail == 'timeline' and ordinal == self.at: process['exit_raw_ns'] = 1
        return render(header, rows), process


def reader_fixture(root):
    """Entirely fabricated native-shaped receipts for reader unit controls only."""
    native_images = images()
    host = dict(head='a' * 40, dirty='', boot='synthetic-only', affinity=list(range(8)), qualification=m.common.QUAL,
                kernel='6.6.synthetic-microsoft-standard-WSL2', uid=1000, gid=1000,
                taskset_sha256='a' * 64, cpuinfo_identity_sha256='b' * 64,
                configs={path: None for path in ('/etc/wsl.conf', '/etc/systemd/timesyncd.conf', '/mnt/c/Users/ArturBuzov/.wslconfig')},
                namespaces={name: 'synthetic-only' for name in r.NAMESPACES})
    class RecordedFake(Fake):
        def stable(self): return host
        def invoke(self, ordinal, selection, image, timeout):
            raw, process = super().invoke(ordinal, selection, image, timeout)
            name = f'{ordinal:03d}-profile'
            save_command(root / 'cohort', name, m.cpu.argv(0, image['path'], m.cpu.MEASURE),
                         '/owned/' + m.ROLE[selection['role']], raw, process)
            return raw, process
    backend = RecordedFake()
    terminal = r.run_cohort(backend, root / 'cohort', native_images, m.signature(backend.rows), 0, host)
    if terminal['failure']: raise AssertionError(terminal['failure'])
    hashes = m.inputs()
    for name in hashes:
        target = root / 'compiled-inputs' / name; target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes((m.ROOT / name).read_bytes())
    outer = dict(failure=None, plan=m.plan(), runtime_sources=m.plan()['runtime_sources'],
        allocation_wrapper=False, server=False, product=False, live=False, clocks_changed=False, services_changed=False,
        measure_before=host, measure_after=host, build_before=host, method=host['head'], cpu=0,
        tool_hashes=hashes, images=native_images, owned_root='/owned')
    m.common.publish(root / 'result.json', outer)
    for lane in m.COMPILERS:
        for name, (argv, cwd) in e.build_commands(lane, '/owned').items():
            if name.endswith('-build'):
                role = 'baseline' if '-baseline-' in name else 'installed'
                raw = json.dumps(dict(reason='compiler-artifact', executable=native_images[lane][role]['path'], fresh=False,
                    target=dict(name='hydracache_long_run_supervisor_074'), profile=dict(test=True, opt_level='3'))).encode()
            else: raw = b'synthetic command control only'
            save_command(root, name, argv, cwd or '/synthetic-source', raw)
        for role in m.ROLE.values():
            for index, test in enumerate(m.cpu.CONTROLS):
                raw = ('running 1 test\ntest ' + test + ' ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored;\n').encode()
                save_command(root, lane + '-' + role + '-control-' + str(index),
                             m.cpu.argv(0, native_images[lane][role]['path'], test), '/owned/' + role, raw)


def save_command(folder, name, argv, cwd, raw, process=None):
    (folder / (name + '.stdout.log')).write_bytes(raw)
    (folder / (name + '.stderr.log')).write_bytes(b'')
    receipt = dict(name=name, argv=argv, cwd=cwd, exit=0, timeout=False,
                   stdout_sha256=m.common.sha_bytes(raw), stderr_sha256=m.common.sha_bytes(b''))
    if process: receipt.update(error=None, **process)
    m.common.publish(folder / (name + '.json'), receipt)


class Controls(unittest.TestCase):
    @classmethod
    def setUpClass(cls): cls.expected = m.reference()

    def run_fake(self, backend):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'cohort'
            result = r.run_cohort(backend, output, images(), self.expected, 0, dict(boot='same'))
            steps = [json.loads(p.read_text()) for p in sorted(output.glob('*-step.json'))]
            self.assertEqual(json.loads((output / 'result.json').read_text()), result)
            return result, steps

    def test_exact_schedule_digest_positions_and_counts(self):
        rows = m.validate_schedule(m.schedule())
        self.assertEqual(len(rows), 64); self.assertEqual(m.schedule_digest(rows), m.SCHEDULE_SHA)
        self.assertEqual([r['compiler'] for r in rows[:9]], ['194'] * 4 + ['188'] * 5)
        for mutate in (lambda v: v.pop(), lambda v: v.reverse(), lambda v: v.append(v[0]),
                       lambda v: v[0].update(position=True), lambda v: v[0].update(role='I')):
            changed = copy.deepcopy(rows); mutate(changed)
            with self.assertRaises(ValueError): m.validate_schedule(changed)

    def test_plan_cli_has_no_native_or_build_route(self):
        with patch.object(r, 'execute') as execute, patch.object(subprocess, 'Popen', side_effect=AssertionError('no subprocess')):
            with contextlib.redirect_stdout(io.StringIO()) as stdout: r.main(['--plan'])
            self.assertEqual(json.loads(stdout.getvalue()), m.plan()); execute.assert_not_called()
            for args in (['--plan', '--attempt', 'x'], ['--execute']):
                with self.assertRaises(ValueError): r.main(args)
            execute.assert_not_called()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit): r.main([])

    def test_environment_and_unsafe_attempt_refuse_before_preparation(self):
        with patch.object(r.sys, 'platform', 'win32'), self.assertRaises(ValueError): r.environment_guard()
        with patch.object(r, 'environment_guard') as guard, self.assertRaises(ValueError): r.execute('../escape')
        guard.assert_not_called()

    def test_all_64_synthetic_positions_complete_serially(self):
        backend = Fake()
        with patch.object(subprocess, 'Popen', side_effect=AssertionError('no executable')):
            result, steps = self.run_fake(backend)
        self.assertIsNone(result['failure']); self.assertEqual(result['mode'], 'synthetic')
        self.assertEqual(result['completed'], 64); self.assertEqual(result['attempted'], 64)
        self.assertEqual(backend.calls, m.schedule()); self.assertEqual(len(steps), 64)
        self.assertEqual(backend.checked, 128); self.assertEqual(backend.snapshots, 128)
        self.assertEqual(backend.timeouts, [300] * 64)

    def test_first_refusals_keep_prefix_without_retry(self):
        for fault, completed, attempted_calls in (('timeout', 5, 6), ('interrupt', 5, 6), ('corpus', 5, 6),
                ('semantic', 5, 6), ('timeline', 5, 6), ('image', 5, 5), ('context', 5, 5),
                ('deadline', 5, 5), ('overrun', 5, 6)):
            with self.subTest(fault=fault):
                backend = Fake(fail=fault, at=5); result, steps = self.run_fake(backend)
                self.assertEqual(result['completed'], completed)
                self.assertEqual(result['attempted'], 6); self.assertEqual(len(backend.calls), attempted_calls)
                self.assertEqual(len(steps), 6); self.assertIsNotNone(result['failure'])
                self.assertIsNotNone(steps[-1]['failure']); self.assertTrue(all(s['failure'] is None for s in steps[:-1]))

    def test_existing_partial_cohort_never_resumes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); backend = Fake()
            with self.assertRaises(ValueError): r.run_cohort(backend, root, images(), self.expected, 0, {})
            self.assertEqual(backend.calls, [])

    def test_remaining_budget_caps_child_and_stops_without_extension(self):
        class NearDeadline(Fake):
            def context(self):
                value = super().context()
                if not self.calls: self.seconds = 100 + m.COHORT_SECONDS - 2
                return value
        backend = NearDeadline(); result, _ = self.run_fake(backend)
        self.assertEqual(backend.timeouts[0], 2)
        self.assertIsNotNone(result['failure'])
        self.assertLess(len(backend.calls), 64)

    def test_optional_context_missing_is_explicit_not_a_filter(self):
        counter = iter([10, 12])
        def unreadable(path): raise PermissionError('synthetic')
        value = r.optional_context(0, read=unreadable, clock=lambda: next(counter))
        r.validate_context(value, 0)
        self.assertTrue(all(v == dict(status='unavailable', reason='PermissionError') for v in value['values'].values()))
        for changed in (dict(value, end_raw_ns=1), dict(value, values={})):
            with self.assertRaises(ValueError): r.validate_context(changed, 0)

    def test_image_role_compiler_and_hash_reuse_refuse(self):
        m.validate_images(images())
        for field in ('path', 'sha256'):
            changed = images(); changed['188']['installed'][field] = changed['194']['baseline'][field]
            with self.assertRaises(ValueError): m.validate_images(changed)
        changed = images(); changed['188']['installed']['compiler'] = '1.94.0'
        with self.assertRaises(ValueError): m.validate_images(changed)

    def test_geometric_block_estimator_and_zero_counters(self):
        self.assertAlmostEqual(m.block_comparison([1, 100], [2, 200])['ratio'], 2)
        self.assertIsNone(m.block_comparison([0, 1], [0, 2])['ratio'])
        self.assertIsNone(m.interval([None] * 8)['median'])
        self.assertEqual(m.interval([1] * 8)['nominal_coverage'], 0.9921875)
        for values in ([1], [float('nan'), 1], [True, 1], [-1, 1]):
            with self.assertRaises(ValueError): m.block_comparison(values, [1, 1])
        with self.assertRaises(ValueError): m.interval([1] * 7)
        for boundary in (95, 105):
            block = m.block_comparison([100, 100], [boundary, boundary])
            self.assertFalse(m.beyond_boundary(block, 'benefit'))
            self.assertFalse(m.beyond_boundary(block, 'cost'))
        self.assertTrue(m.beyond_boundary(m.block_comparison([100, 100], [94, 94]), 'benefit'))
        self.assertTrue(m.beyond_boundary(m.block_comparison([100, 100], [106, 106]), 'cost'))

    def test_summarizer_requires_every_block_and_canonical_workload(self):
        _, rows = fixture()
        observations = [dict(selection=s, rows=copy.deepcopy(rows)) for s in m.schedule()]
        value = e.summarize(observations, self.expected)
        for primary in value['primary'].values():
            self.assertEqual(primary['disposition'], 'no-clear-practical-effect-or-mixed')
            self.assertFalse(primary['release_admission'])
            for lane in primary['lanes'].values():
                self.assertEqual(len(lane['blocks']), 8)
                self.assertEqual(lane['interval'], [1, 1])
        with self.assertRaises(ValueError): e.summarize(observations[:-1], self.expected)
        observations[0]['rows'][0]['seed'] = 75
        with self.assertRaises(ValueError): e.summarize(observations, self.expected)

    def test_frozen_probe_overlay_and_fresh_build_guards_stay_original(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); tests = root / m.common.TESTS; tests.parent.mkdir(parents=True)
            original = {m.common.TESTS: b'fn unchanged() {}\n'}
            tests.write_bytes(original[m.common.TESTS] + m.cpu.HOOK.encode())
            for name, raw in m.cpu.generated('installed').items():
                path = root / name; path.parent.mkdir(parents=True, exist_ok=True); path.write_bytes(raw)
            self.assertEqual(len(m.cpu.validate(root, original, 'installed')), 3)
            tests.write_bytes(tests.read_bytes() + b'fn extra() {}')
            with self.assertRaises(ValueError): m.cpu.validate(root, original, 'installed')
        row = dict(reason='compiler-artifact', executable='/owned/image', fresh=True,
                   target=dict(name='hydracache_long_run_supervisor_074'), profile=dict(test=True, opt_level='3'))
        with self.assertRaises(ValueError): m.common.image(json.dumps(row).encode())

    def test_freeze_timeout_and_wrong_digest_never_launch(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); clock = iter([0, 600])
            with self.assertRaisesRegex(ValueError, 'freeze absent'):
                r.wait_method(root, m.inputs(), clock=lambda: next(clock), sleep=lambda _: None)
            m.common.publish(root / 'method.json', dict(source='a' * 40, schedule_sha256='wrong'))
            with self.assertRaises(ValueError): r.wait_method(root, m.inputs(), clock=lambda: 0)

    def test_freeze_exact_source_and_original_deadline(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            m.common.publish(root / 'method.json', dict(source='a' * 40, schedule_sha256=m.SCHEDULE_SHA))
            with patch.object(m.common, 'source', return_value=b'changed'), self.assertRaises(ValueError):
                r.wait_method(root, m.inputs(), clock=lambda: 0)
            with patch.object(m.common, 'source', side_effect=lambda commit, name: (m.ROOT / name).read_bytes()):
                self.assertEqual(r.wait_method(root, m.inputs(), clock=lambda: 0), 'a' * 40)
            with self.assertRaises(ValueError): r.wait_method(root, {}, clock=lambda: 0)

    def test_native_adapter_success_is_mocked_and_retains_parent_receipt(self):
        class Process:
            pid = 4321
            returncode = 0
            def communicate(self, timeout=None): return b'synthetic bytes', b''
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); backend = r.LinuxBackend(root, dict(baseline=Path('/owned/baseline')), {}, 0)
            with patch.object(r.subprocess, 'Popen', return_value=Process()) as spawn, patch.object(r, 'raw_clock', side_effect=itertools.count(10)), patch.object(r.os, 'killpg', create=True) as kill:
                raw, stamps = backend.invoke(0, m.schedule()[0], images()['194']['baseline'], 300)
            self.assertEqual(raw, b'synthetic bytes'); self.assertEqual(stamps['pid'], 4321)
            self.assertTrue(spawn.call_args.kwargs['start_new_session']); kill.assert_not_called()
            self.assertEqual(spawn.call_args.args[0], m.cpu.argv(0, images()['194']['baseline']['path'], m.cpu.MEASURE))
            receipt = json.loads((root / '000-profile.json').read_text())
            self.assertFalse(receipt['timeout']); self.assertEqual(receipt['exit'], 0)
            self.assertEqual(receipt['stdout_sha256'], m.common.sha_bytes(raw))

    def test_native_timeout_and_interrupt_kill_only_owned_group_in_mock(self):
        for kind in ('timeout', 'interrupt'):
            class Process:
                pid = 4321
                returncode = None
                calls = 0
                def communicate(self, timeout=None):
                    self.calls += 1
                    if self.calls == 1:
                        if kind == 'timeout': raise subprocess.TimeoutExpired('synthetic', timeout)
                        raise KeyboardInterrupt('synthetic')
                    self.returncode = -9
                    return b'synthetic partial output', b'first refusal'
                def poll(self): return self.returncode
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as directory:
                root = Path(directory); backend = r.LinuxBackend(root, dict(baseline=Path('/owned/baseline')), {}, 0)
                with patch.object(r.subprocess, 'Popen', return_value=Process()), patch.object(r, 'raw_clock', side_effect=itertools.count(10)), patch.object(r.os, 'killpg', create=True) as kill, patch.object(r.signal, 'SIGKILL', 9, create=True):
                    with self.assertRaises(ValueError): backend.invoke(0, m.schedule()[0], images()['194']['baseline'], 2)
                kill.assert_called_once_with(4321, 9)
                receipt = json.loads((root / '000-profile.json').read_text())
                self.assertEqual(receipt['timeout'], kind == 'timeout')
                self.assertEqual((root / '000-profile.stdout.log').read_bytes(), b'synthetic partial output')

    def test_native_spawn_failure_keeps_first_error_without_kill_or_retry(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); backend = r.LinuxBackend(root, dict(baseline=Path('/owned/baseline')), {}, 0)
            with patch.object(r.subprocess, 'Popen', side_effect=OSError('synthetic spawn refusal')) as spawn, patch.object(r, 'raw_clock', side_effect=itertools.count(10)), patch.object(r.os, 'killpg', create=True) as kill:
                with self.assertRaises(ValueError): backend.invoke(0, m.schedule()[0], images()['194']['baseline'], 300)
            spawn.assert_called_once(); kill.assert_not_called()
            receipt = json.loads((root / '000-profile.json').read_text())
            self.assertIn('OSError', receipt['error']); self.assertIsNone(receipt['exit'])

    def test_complete_reader_and_tamper_refusals_use_fabricated_transcripts_only(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with patch.object(subprocess, 'Popen', side_effect=AssertionError('no native process')):
                reader_fixture(root)
                with self.assertRaisesRegex(ValueError, 'Synthetic'): e.verify(root)
                path = root / 'cohort/result.json'; terminal = json.loads(path.read_text())
                terminal['mode'] = 'native'
                # Explicitly fabricated native-shaped fixture, never retained as measured evidence.
                path.write_text(json.dumps(terminal))
                facts = e.verify(root)
                self.assertEqual(facts['processes'], 64); self.assertEqual(facts['rows'], 768)
                for name, mutation in (
                        ('cohort/005-step.json', lambda v: v.update(timeout_seconds=301)),
                        ('cohort/005-step.json', lambda v: v['selection'].update(position=3)),
                        ('194-baseline-check.json', lambda v: v.update(argv=['unrelated'])),
                        ('result.json', lambda v: v['tool_hashes'].pop(m.INPUTS[0])),
                        ('cohort/result.json', lambda v: v.update(completed=63))):
                    with self.subTest(name=name):
                        target = root / name; before = target.read_bytes(); changed = json.loads(before)
                        mutation(changed); target.write_text(json.dumps(changed))
                        with self.assertRaises((ValueError, KeyError)): e.verify(root)
                        target.write_bytes(before)
                raw = root / 'cohort/000-profile.stdout.log'; before = raw.read_bytes(); raw.write_bytes(before + b' ')
                with self.assertRaises(ValueError): e.verify(root)
                raw.write_bytes(before)
                (root / 'cohort/extra').write_bytes(b'unregistered')
                with self.assertRaises(ValueError): e.verify(root)

    def test_strict_native_build_check_command_receipts(self):
        own = Path('/synthetic')
        with patch.object(m.cpu, 'build', return_value=('194', images()['194'])) as builder, patch.object(m.common, 'invoke', return_value=b'') as invoke:
            selected = r.build('194', dict(baseline=own, installed=own), own, own, {}, 0)
        self.assertEqual(selected, images()['194']); builder.assert_called_once()
        self.assertEqual(invoke.call_count, 2)
        for call in invoke.call_args_list:
            self.assertIn('--offline', call.args[0]); self.assertIn('--locked', call.args[0]); self.assertIn('--tests', call.args[0])

    def test_full_execute_orchestration_is_mocked_without_builds_or_profiles(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); own = root / 'cache'
            (root / 'target').mkdir()
            host = dict(head='a' * 40, dirty='', boot='synthetic', affinity=[0], qualification=m.common.QUAL)
            class Backend:
                @staticmethod
                def stable(): return host
            def prepare(owned, output):
                folders = {role: owned / role for role in m.ROLE.values()}
                return folders, dict.fromkeys(m.ROLE.values(), {}), 'a' * 64
            with patch.object(m, 'ROOT', root), patch.object(r, 'OWN_ROOT', own), patch.object(r, 'environment_guard'), \
                    patch.object(r, 'validate_host'), patch.object(m, 'reference', return_value=self.expected), \
                    patch.object(m, 'inputs', return_value={}), patch.object(r, 'prepare', side_effect=prepare), \
                    patch.object(r, 'build', side_effect=lambda lane, *args: images()[lane]) as build, \
                    patch.object(r, 'wait_method', return_value='a' * 40), patch.object(m.cpu, 'validate'), \
                    patch.object(r, 'LinuxBackend', return_value=object()) as backend, \
                    patch.object(r, 'run_cohort', return_value=dict(failure=None)) as cohort, \
                    patch.object(subprocess, 'Popen', side_effect=AssertionError('no native process')):
                backend.stable.side_effect = Backend.stable
                r.execute('synthetic-control')
            self.assertEqual(build.call_count, 2); cohort.assert_called_once()
            result = json.loads((root / 'target/authority-runtime-balanced-synthetic-control/result.json').read_text())
            self.assertIsNone(result['failure']); self.assertFalse(result['server']); self.assertFalse(result['product'])
            self.assertEqual(result['measure_before'], result['measure_after'])


if __name__ == '__main__': unittest.main(verbosity=2)
