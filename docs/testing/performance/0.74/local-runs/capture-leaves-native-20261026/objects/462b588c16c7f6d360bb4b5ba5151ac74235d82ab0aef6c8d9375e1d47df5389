"""Real-Git topology control models; no native command or package execution."""
import copy
import json
from pathlib import Path
import tempfile
import tomllib
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import git_process_boundary as subject
from owned_process_observer import Refusal


def sample(pid, terminal=False):
    return dict(terminal=terminal, creation_100ns=pid * 100, exit_100ns=9999 if terminal else None,
                kernel_100ns=2, user_100ns=3, io={'read_ops': 0}, memory=None)


class Child:
    pid, _handle = 11, 11

    def __init__(self):
        self.terminal = False
        self.waits = []
        self.stdin = SimpleNamespace(closed=False)
        self.stdin.close = lambda: setattr(self.stdin, 'closed', True)

    def poll(self):
        return 0 if self.terminal else None

    def wait(self, timeout):
        self.waits.append(timeout)
        assert self.stdin.closed
        self.terminal = True
        return 0


class Api:
    def __init__(self, child):
        self.child = child
        self.opened, self.closed = [], []
        self.rows = [(11, 0, 'git.exe'), (22, 11, 'git.exe')]
        self.images = {11: subject.LAUNCHER, 22: subject.NATIVE}
        self.snapshots = 0
        self.regress = False

    def snapshot(self):
        self.snapshots += 1
        return copy.deepcopy(self.rows)

    def identity(self, handle):
        return handle, self.images[handle]

    def read(self, handle):
        row = sample(handle, self.child.terminal)
        if self.regress and handle == 22 and self.child.terminal:
            row['user_100ns'] = 0
        return row

    def open(self, pid):
        self.opened.append(pid)
        return pid

    def close(self, handle):
        self.closed.append(handle)


def recorded():
    return dict(scope='git-launcher-native-boundary-only', argv=subject.ARGV,
                refusal=None, cleanup_errors=[], exit_code=0, retry=False, object_requests=0,
                run_seconds=10, discovery_seconds=5, cleanup_seconds=5, discovery_snapshots=1,
                image_pins_before=subject.IMAGES, image_pins_after=subject.IMAGES,
                package_pass=False, performance_cohort=False, product_admission=False,
                mapped_image_attested=False, all_descendants_accounted=False, original_helper_executed=False,
                identities=[dict(pid=pid, image=image, creation_100ns=pid*100)
                            for pid, image in ((11, subject.LAUNCHER), (22, subject.NATIVE))],
                root_samples=[sample(11), sample(11, True)], native_samples=[sample(22), sample(22, True)])


class Boundary(unittest.TestCase):
    def test_borrows_original_root_holds_one_native_and_releases_with_eof(self):
        child = Child()
        api = Api(child)
        result = subject.observe(child, api, 0, clock=lambda: 0, sleep=lambda _: None)
        self.assertEqual((api.opened, api.closed), ([22], [22]))
        self.assertEqual(api.snapshots, 3)
        self.assertEqual([row['pid'] for row in result['identities']], [11, 22])
        self.assertTrue(child.stdin.closed)
        self.assertEqual(child.waits, [10])
        self.assertTrue(result['root_samples'][-1]['terminal'])
        self.assertTrue(result['native_samples'][-1]['terminal'])

    def test_early_root_exit_does_not_open_another_process(self):
        child = Child()
        child.terminal = True
        api = Api(child)
        with self.assertRaisesRegex(Refusal, 'root exited'):
            subject.observe(child, api, 0, clock=lambda: 0)
        self.assertEqual(api.opened, [])

    def test_missing_native_exhausts_discovery_without_attachment(self):
        child = Child()
        api = Api(child)
        api.rows.pop()
        with self.assertRaisesRegex(Refusal, 'discovery exhausted'):
            subject.observe(child, api, 0, clock=lambda: 0, sleep=lambda _: None)
        self.assertEqual(api.snapshots, 100)
        self.assertEqual(api.opened, [])

    def test_wrong_image_is_one_failed_attachment_without_fallback(self):
        child = Child()
        api = Api(child)
        api.images[22] = r'C:\wrong\git.exe'
        with self.assertRaisesRegex(Refusal, 'held lineage image/PID mismatch'):
            subject.observe(child, api, 0, clock=lambda: 0)
        self.assertEqual((api.opened, api.closed, api.snapshots), ([22], [22], 2))

    def test_late_attachment_and_exit_are_not_accepted(self):
        for times, reason in (([0, 0, 5], 'late attachment'), ([0, 0, 0, 0, 10], 'late root exit')):
            child = Child()
            api = Api(child)
            with patch.object(subject.time, 'monotonic_ns', return_value=1), self.assertRaisesRegex(Refusal, reason):
                subject.observe(child, api, 0, clock=unittest.mock.Mock(side_effect=times))
            self.assertEqual((api.opened, api.closed), ([22], [22]))

    def test_late_snapshot_cannot_begin_attachment(self):
        child = Child()
        api = Api(child)
        with self.assertRaisesRegex(Refusal, 'late discovery witness'):
            subject.observe(child, api, 0, clock=unittest.mock.Mock(side_effect=[0, 5]))
        self.assertEqual(api.opened, [])

    def test_preregistered_protocol_matches_fixed_limits_command_and_closed_scope(self):
        protocol = tomllib.loads(Path(subject.__file__).with_name('git-boundary-protocol.toml').read_text())
        self.assertEqual(protocol['argv'], subject.ARGV)
        self.assertEqual((protocol['run_seconds'], protocol['discovery_seconds'],
                          protocol['maximum_discovery_snapshots'], protocol['cleanup_seconds']), (10, 5, 100, 5))
        self.assertEqual(protocol['attachment_attempts'], 1)
        self.assertEqual(protocol['object_requests'], 0)
        for key in ('replacement_root_or_leaf', 'environment_sanitization', 'performance_comparison',
                    'old_cohort_topology_attested', 'mapped_image_attested', 'all_descendants_accounted',
                    'package_or_release_admission', 'retry', 'server_services_clocks_product'):
            self.assertIs(protocol[key], False)

    def test_counter_regression_remains_a_refusal_with_owned_query_cleanup(self):
        child = Child()
        api = Api(child)
        api.regress = True
        with self.assertRaisesRegex(Refusal, 'counter regressed'):
            subject.observe(child, api, 0, clock=lambda: 0)
        self.assertEqual(api.closed, [22])

    def test_positive_reader_excludes_speedup_and_old_cohort_claims(self):
        result = subject.review(recorded())
        self.assertTrue(result['directly_created_counter_excludes_native'])
        self.assertFalse(result['numeric_speedup_established'])
        self.assertFalse(result['old_cohort_topology_attested'])
        self.assertFalse(result['current_full_helper_cost_attributed'])

    def test_scope_pin_budget_identity_and_missing_terminal_mutations_refuse(self):
        variants = []
        for key, value in (('argv', ['git', 'show']), ('refusal', 'first failure'),
                           ('cleanup_errors', ['close refused']), ('package_pass', True),
                           ('object_requests', True), ('object_requests', 1), ('run_seconds', 11),
                           ('discovery_snapshots', 101), ('image_pins_after', {})):
            value = {**recorded(), key: value}
            variants.append(value)
        for mode in ('same-pid', 'stale-birth', 'missing-terminal', 'reversed-exit'):
            value = recorded()
            if mode == 'same-pid':
                value['identities'][1]['pid'] = 11
            elif mode == 'stale-birth':
                value['native_samples'][1]['creation_100ns'] += 1
            elif mode == 'missing-terminal':
                value['native_samples'].pop()
            else:
                value['root_samples'][1]['exit_100ns'] = 9998
            variants.append(value)
        for value in variants:
            with self.subTest(value=value), self.assertRaises(Refusal):
                subject.review(value)

    def test_first_preflight_refusal_has_intent_and_does_not_spawn_or_retry(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(subject, 'ROOT', Path(directory)), \
                patch.object(subject, 'pins', side_effect=Refusal('original pin failure')), \
                patch.object(subject.subprocess, 'Popen') as spawn:
            result = subject.run('fixture')
            self.assertFalse(result['passed'])
            self.assertEqual(result['refusal'], 'Refusal: original pin failure')
            out = Path(directory) / 'target/git-process-boundary-20261026/fixture'
            self.assertFalse(json.loads((out / 'intent.json').read_bytes())['retry'])
            self.assertEqual(json.loads((out / 'result.json').read_bytes())['refusal'], result['refusal'])
            with self.assertRaises(FileExistsError):
                subject.run('fixture')
            spawn.assert_not_called()

    def test_bad_labels_refuse_before_directory_creation(self):
        for label in ('', '../outside', 'UPPER', 'x'*65):
            with self.assertRaisesRegex(Refusal, 'bounded local label'):
                subject.run(label)

    def test_image_checks_refuse_path_or_bytes_without_adopting_discovery(self):
        class Digest:
            def __init__(self, raw):
                self.raw = raw
            def hexdigest(self):
                return subject.IMAGES.get(self.raw.decode(), 'changed')
        with patch.object(subject.shutil, 'which', return_value=subject.LAUNCHER), \
                patch.object(subject.Path, 'read_bytes', lambda path: str(path).encode()), \
                patch.object(subject.hashlib, 'sha256', Digest):
            self.assertEqual(subject.pins(), subject.IMAGES)
            with patch.object(subject.shutil, 'which', return_value=r'C:\wrong\git.exe'):
                with self.assertRaisesRegex(Refusal, 'launcher path drift'):
                    subject.pins()
            with patch.object(subject.Path, 'read_bytes', return_value=b'changed'):
                with self.assertRaisesRegex(Refusal, 'image bytes drift'):
                    subject.pins()

    def test_close_failure_overrides_an_otherwise_healthy_control_and_is_retained(self):
        child = Child()
        child._handle = SimpleNamespace(Close=unittest.mock.Mock(side_effect=Refusal('original close failure')))
        environment = SimpleNamespace(environment_preflight=lambda root: {})
        with tempfile.TemporaryDirectory() as directory, patch.object(subject, 'ROOT', Path(directory)), \
                patch.object(subject, 'pins', return_value=subject.IMAGES), \
                patch.dict(subject.sys.modules, {'package_effects_review_v6': environment}), \
                patch.object(subject.subprocess, 'Popen', return_value=child), \
                patch.object(subject, 'SnapshotApi'), patch.object(subject, 'observe', return_value=recorded()):
            result = subject.run('fixture')
            self.assertFalse(result['passed'])
            self.assertEqual(result['cleanup_errors'], ['original close failure'])
            self.assertTrue(child.stdin.closed)
            with self.assertRaisesRegex(Refusal, 'unclean control'):
                subject.review(result)

    def test_late_close_cannot_publish_a_passing_cleanup(self):
        child = Child()
        now = [0]
        child._handle = SimpleNamespace(Close=lambda: now.__setitem__(0, 6))
        environment = SimpleNamespace(environment_preflight=lambda root: {})
        with tempfile.TemporaryDirectory() as directory, patch.object(subject, 'ROOT', Path(directory)), \
                patch.object(subject, 'pins', return_value=subject.IMAGES), \
                patch.dict(subject.sys.modules, {'package_effects_review_v6': environment}), \
                patch.object(subject.subprocess, 'Popen', return_value=child), \
                patch.object(subject, 'SnapshotApi'), patch.object(subject, 'observe', return_value=recorded()), \
                patch.object(subject.time, 'monotonic', side_effect=lambda: now[0]):
            result = subject.run('fixture')
            self.assertFalse(result['passed'])
            self.assertIn('late cleanup witness', result['cleanup_errors'])

    def test_first_refusal_survives_cleanup_timeout_and_owned_root_fallback(self):
        child = Child()
        child._handle = SimpleNamespace(Close=unittest.mock.Mock())
        child.wait = unittest.mock.Mock(side_effect=subject.subprocess.TimeoutExpired('owned-root', 5))
        child.kill = unittest.mock.Mock(side_effect=lambda: setattr(child, 'terminal', True))
        environment = SimpleNamespace(environment_preflight=lambda root: {})
        with tempfile.TemporaryDirectory() as directory, patch.object(subject, 'ROOT', Path(directory)), \
                patch.object(subject, 'pins', return_value=subject.IMAGES), \
                patch.dict(subject.sys.modules, {'package_effects_review_v6': environment}), \
                patch.object(subject.subprocess, 'Popen', return_value=child) as spawn, \
                patch.object(subject, 'SnapshotApi'), \
                patch.object(subject, 'observe', side_effect=Refusal('first discovery refusal')):
            result = subject.run('fixture')
            self.assertFalse(result['passed'])
            self.assertEqual(result['refusal'], 'Refusal: first discovery refusal')
            self.assertEqual(len(result['cleanup_errors']), 2)
            child.kill.assert_called_once_with()
            child._handle.Close.assert_called_once_with()
            self.assertEqual(spawn.call_count, 1)

    def test_changed_postflight_image_is_failure_not_another_native_attempt(self):
        child = Child()
        child._handle = SimpleNamespace(Close=unittest.mock.Mock())
        environment = SimpleNamespace(environment_preflight=lambda root: {})
        with tempfile.TemporaryDirectory() as directory, patch.object(subject, 'ROOT', Path(directory)), \
                patch.object(subject, 'pins', side_effect=[subject.IMAGES, Refusal('postflight image drift')]), \
                patch.dict(subject.sys.modules, {'package_effects_review_v6': environment}), \
                patch.object(subject.subprocess, 'Popen', return_value=child) as spawn, \
                patch.object(subject, 'SnapshotApi'), patch.object(subject, 'observe', return_value=recorded()):
            result = subject.run('fixture')
            self.assertFalse(result['passed'])
            self.assertEqual(result['refusal'], 'Refusal: postflight image drift')
            self.assertEqual(spawn.call_count, 1)


if __name__ == '__main__':
    unittest.main()
