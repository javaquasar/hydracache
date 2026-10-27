import os
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import gnu_package as p


class Child:
    def __init__(self, group, code): self.group, self.code = group, code
    def poll(self): return self.code
    def wait(self, timeout):
        if not self.group.retired: raise RuntimeError('early reap')
    def close(self): pass


class Group:
    def __init__(self, code):
        self.retired = False
        self.code, self.terminated, self.closed = code, False, False
    def is_empty(self):
        if self.code is not None or self.terminated: self.retired = True
        return self.retired
    def terminate(self): self.terminated = True
    def close(self):
        if not self.retired: raise RuntimeError('live close')
        self.closed = True


class PureControls(unittest.TestCase):
    def test_restored_attempt_preserves_package_contract(self):
        import gnu_review as review
        self.assertEqual(review.TARGET.as_posix(), '/home/artur/.cache/hydracache-074-194-check')
        self.assertEqual(review.command(), ['/home/artur/.cargo/bin/cargo', '+1.94.0',
            'test', '--offline', '--locked', '--no-fail-fast', '--target-dir',
            str(review.TARGET), '-p', 'xtask',
            '--lib', '--tests', '--', '--test-threads=4'])
        self.assertEqual((p.BOUND, p.FLOOR), (780, 4 * 1024**3))
        self.assertEqual(p.ENV, dict(RUSTUP_AUTO_INSTALL='0', CARGO_INCREMENTAL='0', CARGO_BUILD_JOBS='2'))
        self.assertEqual(p.OUT, p.CONTROL / 'target/native-full-package-capacity-restored-20261027')
        self.assertEqual(p.CONTROL_PHASE, 'native-full-coordinator-restored-final-controls')

    def test_default_and_unknown_cli_never_dispatch(self):
        run = subprocess.run([sys.executable, str(Path(p.__file__))], capture_output=True, timeout=10)
        self.assertEqual(run.returncode, 0)
        self.assertIn(b'"execute": false', run.stdout)
        run = subprocess.run([sys.executable, str(Path(p.__file__)), '--timeout', '900'], capture_output=True, timeout=10)
        self.assertEqual(run.returncode, 2)

    def run_model(self, group, capacity=lambda: None, clock=lambda: 0):
        with patch.object(p, 'spawn_separate', return_value=Child(group, group.code)):
            return p.owned(group, ['only-model'], Path('.'), Path('.'), capacity,
                pause=lambda _: None, now=clock)

    def test_success_and_failure_require_retirement(self):
        for code in (0, 101):
            group = Group(code)
            receipt = self.run_model(group)
            self.assertEqual(receipt['exit'], code)
            self.assertTrue(receipt['retired'] and group.closed)
            self.assertEqual((receipt['bound_seconds'], receipt['cleanup_seconds']), (780, 5))

    def test_timeout_and_floor_do_not_become_success(self):
        clock_values = iter([0, 781, 782, 782, 782])
        group = Group(None)
        receipt = self.run_model(group, clock=lambda: next(clock_values))
        self.assertEqual(receipt['classification'], 'timeout')
        self.assertEqual(receipt['exit'], 124)
        self.assertTrue(group.terminated and group.closed)
        group = Group(None)
        def floor(): raise p.CapacityError('floor')
        receipt = self.run_model(group, capacity=floor)
        self.assertEqual(receipt['classification'], 'resource-floor')
        self.assertNotEqual(receipt['exit'], 0)

    def test_cleanup_error_overrides_zero_child_exit(self):
        group = Group(0)
        group.close = lambda: (_ for _ in ()).throw(RuntimeError('close fault'))
        receipt = self.run_model(group)
        self.assertEqual((receipt['classification'], receipt['exit']), ('runner-loss', 125))

    def test_spawn_failure_restores_descriptors_and_environment(self):
        before = {name: os.environ.get(name) for name in p.ENV}
        def identities():
            return [(s.st_dev, s.st_ino, s.st_mode, getattr(s, 'st_rdev', None))
                    for s in (os.fstat(1), os.fstat(2))]
        original = identities()
        with tempfile.TemporaryDirectory() as folder:
            class Refuse:
                def spawn(self, *args): raise OSError('original spawn failure')
            with self.assertRaisesRegex(OSError, 'original spawn'):
                p.spawn_separate(Refuse(), ['never-spawn'], Path('.'), Path(folder), 1)
            # Output readers can change atime and writers can change size/mtime.
            # Those values do not identify the restored original descriptors.
            self.assertEqual(identities(), original)
            self.assertEqual({name: os.environ.get(name) for name in p.ENV}, before)
            with self.assertRaises(FileExistsError):
                p.spawn_separate(Refuse(), ['never-spawn'], Path('.'), Path(folder), 1)

    def test_controls_gate_refuses_missing_red_stale_or_filtered_receipt(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            with self.assertRaises(FileNotFoundError): p.check_controls(root)
            phase = root / 'target/roots-lock-view-20261027' / p.CONTROL_PHASE
            phase.mkdir(parents=True)
            sources = {}
            for name in ('gnu_package.py', 'test_gnu_package.py', 'analysis.py', 'gnu_review.py'):
                relative = 'tools/package-closure-offline-074/' + name
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b'fixture')
                sources[relative] = p.sha(b'fixture')
            argv = ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--exec', '/usr/bin/python3', '-m',
                'unittest', 'discover', '-s', '/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/package-closure-offline-074',
                '-p', 'test_gnu_package.py', '-v']
            intent = dict(argv=argv, source=sources)
            result = dict(exit=0, outer_refusal=None, source_unchanged=True, commit_unchanged=True, source_after=sources)
            def write():
                (phase / 'intent.json').write_text(json.dumps(intent))
                (phase / 'result.json').write_text(json.dumps(result))
            write()
            p.check_controls(root)
            result['exit'] = 1
            write()
            with self.assertRaisesRegex(ValueError, 'not green'): p.check_controls(root)
            result['exit'] = 0
            intent['argv'] = argv + ['--exact', 'subset']
            write()
            with self.assertRaisesRegex(ValueError, 'command drift'): p.check_controls(root)
            intent['argv'] = argv
            write()
            (root / 'tools/package-closure-offline-074/gnu_package.py').write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'source drift'): p.check_controls(root)


@unittest.skipUnless(sys.platform.startswith('linux'), 'existing Linux pidfd CI primitive')
class NativeFixtures(unittest.TestCase):
    def group(self):
        root = Path(__file__).resolve().parents[2]
        return p.load(root / 'scripts/ci/watchdog_linux_group.py', 'checked_native_fixture_group', p.GROUP_SHA).LinuxGroup()

    def test_real_separate_streams_and_owned_exit(self):
        with tempfile.TemporaryDirectory() as folder:
            out = Path(folder)
            receipt = p.owned(self.group(), ['/usr/bin/python3', '-c',
                'import sys; print("stdout-marker"); print("stderr-marker",file=sys.stderr)'],
                out, out, lambda: None, bound=3)
            self.assertEqual(receipt['classification'], 'success')
            self.assertTrue(receipt['retired'])
            self.assertEqual((out / 'stdout.log').read_bytes(), b'stdout-marker\n')
            self.assertEqual((out / 'stderr.log').read_bytes(), b'stderr-marker\n')

    def test_real_timeout_retires_detached_grandchild(self):
        with tempfile.TemporaryDirectory() as folder:
            out = Path(folder)
            script = ('import os,time,pathlib; child=os.fork(); '
                'os.setsid() if child==0 else None; '
                'pathlib.Path("grandchild-ready").write_text(str(os.getpid())) if child==0 else None; '
                'time.sleep(10)')
            receipt = p.owned(self.group(), ['/usr/bin/python3', '-c', script],
                out, out, lambda: None, bound=0.2)
            self.assertEqual((receipt['classification'], receipt['exit']), ('timeout', 124))
            self.assertTrue(receipt['retired'])
            self.assertGreater(int((out / 'grandchild-ready').read_text()), 0)

    def test_real_floor_refusal_retires_original_child(self):
        calls = 0
        def capacity():
            nonlocal calls
            calls += 1
            if calls > 2:
                raise p.CapacityError('synthetic floor after spawn')
        with tempfile.TemporaryDirectory() as folder:
            out = Path(folder)
            receipt = p.owned(self.group(), ['/usr/bin/python3', '-c', 'import time; time.sleep(10)'],
                out, out, capacity, bound=3)
            self.assertEqual(receipt['classification'], 'resource-floor')
            self.assertTrue(receipt['retired'])


if __name__ == '__main__':
    unittest.main()
