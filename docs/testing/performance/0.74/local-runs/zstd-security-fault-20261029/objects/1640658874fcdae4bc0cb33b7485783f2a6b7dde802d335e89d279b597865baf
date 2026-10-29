"""One bounded Windows CI-only fault cohort, retaining every first case result."""
from pathlib import Path
import ctypes
import json
import os
import subprocess
import sys
import time
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve()


def write(path, value):
    with path.open('x', encoding='utf-8') as stream:
        json.dump(value, stream, sort_keys=True)
        stream.write('\n')


def fixture(mode, folder):
    if mode == 'leaf':
        write(folder / 'leaf-ready.json', {'pid': os.getpid()})
        started = time.monotonic()
        with (folder / 'writes.bin').open('ab', buffering=0) as stream:
            while time.monotonic() - started < 8:
                stream.write(b'x')
                time.sleep(0.02)
        return
    if mode == 'tree':
        subprocess.Popen([sys.executable, str(HERE), 'fixture', 'leaf', str(folder)])
        started = time.monotonic()
        while not (folder / 'release-root').exists() and time.monotonic() - started < 8:
            time.sleep(0.01)
        return  # Intentionally leave a live descendant, not a successful whole tree.
    if mode == 'suspended-owner':
        from watchdog_windows_job import Api, WindowsJob
        api = Api()
        job = WindowsJob(api)

        def stop_before_resume(_):
            write(folder / 'suspended-ready.json', {'pid': job.child.pid})
            time.sleep(8)
            raise RuntimeError('fixture resume cut expired without owner loss')

        try:
            with patch.object(api.dll, 'ResumeThread', side_effect=stop_before_resume):
                job.spawn([sys.executable, str(HERE), 'fixture', 'leaf', str(folder)], None, None,
                          time.monotonic() + 5)
        finally:
            job.terminate()
            if job.child is not None:
                job.child.wait(timeout=5)
                job.child.close()
            job.close()
        return
    raise ValueError('fixed fixture modes only')


class Native(unittest.TestCase):
    def setUp(self):
        self.folder = EVIDENCE / self._testMethodName
        self.folder.mkdir(exist_ok=False)
        self.process = self.root = self.lineage = None
        helper = ROOT / 'tools/xtask-package-attribution-074'
        sys.path.insert(0, str(helper))
        from owned_process_observer import OwnedObserver
        from process_lineage import HeldLineage, SnapshotApi
        self.Observer, self.Lineage, self.api = OwnedObserver, HeldLineage, SnapshotApi()
        self.result = {'pass': False, 'server': False, 'product': False, 'retry': False}
        self.stdout = (self.folder / 'stdout.log').open('xb')
        self.stderr = (self.folder / 'stderr.log').open('xb')

    def start(self, suspended=False):
        if suspended:
            argv = [sys.executable, str(HERE), 'fixture', 'suspended-owner', str(self.folder)]
        else:
            argv = [sys.executable, str(HERE.with_name('run-with-heartbeat.py')),
                    '--timeout-seconds', '5', '--heartbeat-seconds', '0.05',
                    '--status-json', str(self.folder / 'receipt.json'), '--',
                    sys.executable, str(HERE), 'fixture', 'tree', str(self.folder)]
        write(self.folder / 'intent.json', {'argv': argv, 'readiness_seconds': 5,
            'terminal_seconds': 5, 'node_self_exit_seconds': 8, 'original_marker_test': False})
        self.process = subprocess.Popen(argv, stdout=self.stdout, stderr=self.stderr,
                                        stdin=subprocess.DEVNULL)
        ready = self.folder / ('suspended-ready.json' if suspended else 'leaf-ready.json')
        deadline = time.monotonic() + 5
        while not ready.exists():
            if self.process.poll() is not None or time.monotonic() >= deadline:
                self.fail('fixture did not reach first readiness cut')
            time.sleep(0.01)
        image = str(Path(sys.executable).resolve())
        self.root = self.Observer(self.process, image, self.api)
        self.lineage = self.Lineage(self.root, self.api, [(image,) if suspended else (image, image)])
        self.result['identities'] = self.lineage.identities
        self.result['before'] = [self.api.read(h) for h in [self.root.handle, *self.lineage.handles]]

    def terminal(self):
        deadline = time.monotonic() + 5
        handles = [self.root.handle, *self.lineage.handles]
        while True:
            rows = [self.api.read(h) for h in handles]
            if all(row['terminal'] for row in rows):
                break
            if time.monotonic() >= deadline:
                self.fail('original objects not terminal inside fixed observation bound')
            time.sleep(0.01)
        self.result['terminal'] = rows
        for identity, row in zip(self.lineage.identities, rows):
            self.assertEqual(identity['creation_100ns'], row['creation_100ns'])
        path = self.folder / 'writes.bin'
        before = path.stat().st_size if path.exists() else 0
        time.sleep(0.2)
        after = path.stat().st_size if path.exists() else 0
        self.result['post_terminal_sizes'] = [before, after]
        self.assertEqual(before, after)

    def tearDown(self):
        try:
            if self.process is not None:
                if self.process.poll() is None:
                    self.process.kill()  # Original owned controller handle only.
                self.result['controller_exit'] = self.process.wait(timeout=5)
            if self.lineage is not None:
                self.result['cleanup_objects'] = [self.api.read(h) for h in
                                                  [self.root.handle, *self.lineage.handles]]
                self.lineage.close()
            if self.root is not None:
                self.root.close()
        finally:
            self.stdout.close()
            self.stderr.close()
            write(self.folder / 'result.json', self.result)

    def test_successful_root_leaving_writer_is_timeout_not_success(self):
        self.start()
        (self.folder / 'release-root').touch(exist_ok=False)
        self.assertEqual(self.process.wait(timeout=8), 124)
        receipt = json.loads((self.folder / 'receipt.json').read_bytes())
        self.assertEqual(receipt['classification'], 'timeout')
        self.assertEqual(receipt['exit_code'], 0)
        self.terminal()
        self.result['pass'] = True

    def test_owner_loss_terminates_running_root_and_writer(self):
        self.start()
        self.process.kill()
        self.process.wait(timeout=5)
        self.terminal()
        self.assertFalse((self.folder / 'receipt.json').exists())
        self.result['pass'] = True

    def test_owner_loss_before_resume_terminates_original_suspended_child(self):
        self.start(suspended=True)
        self.process.kill()
        self.process.wait(timeout=5)
        self.terminal()
        self.assertFalse((self.folder / 'leaf-ready.json').exists())
        self.assertFalse((self.folder / 'writes.bin').exists())
        self.result['pass'] = True

    def test_post_create_setup_refusal_never_executes_child(self):
        from watchdog_windows_job import Api, WindowsJob, Win32Failure
        api = Api()
        job = WindowsJob(api)
        try:
            with patch.object(api.dll, 'IsProcessInJob', side_effect=Win32Failure('injected post-create')):
                with self.assertRaisesRegex(Win32Failure, 'injected post-create'):
                    job.spawn([sys.executable, str(HERE), 'fixture', 'leaf', str(self.folder)], None,
                              self.stdout, time.monotonic() + 5)
            self.assertIsNotNone(job.child)
            job.terminate()
            job.child.wait(timeout=5)
            deadline = time.monotonic() + 5
            while not job.is_empty():
                if time.monotonic() >= deadline:
                    self.fail('setup-refused Job did not become empty')
                time.sleep(0.01)
            self.result['child_terminal'] = self.api.read(job.child.handle)
            self.assertTrue(self.result['child_terminal']['terminal'])
            self.assertFalse((self.folder / 'leaf-ready.json').exists())
            self.result['pass'] = True
        finally:
            if job.child is not None:
                job.child.close()
            job.close()

    def test_unsupported_job_list_refuses_without_creating_child(self):
        from watchdog_windows_job import Api, WindowsJob, Win32Failure
        api = Api()
        job = WindowsJob(api)

        def unsupported(*_):
            ctypes.set_last_error(50)
            return 0

        try:
            with patch.object(api.dll, 'UpdateProcThreadAttribute', side_effect=unsupported), \
                 patch.object(api.dll, 'CreateProcessW') as create:
                with self.assertRaisesRegex(Win32Failure, 'JOB_LIST.*WinError 50') as refusal:
                    job.spawn([sys.executable, '-c', 'pass'], None, self.stdout, time.monotonic() + 5)
                create.assert_not_called()
            self.result['refusal'] = str(refusal.exception)
            self.assertIsNone(job.child)
            self.assertTrue(job.is_empty())
            self.result['pass'] = True
        finally:
            job.close()

    def test_cli_cwd_unicode_arguments_and_inherited_streams(self):
        cwd = self.folder / 'directory with spaces \u03a9'
        cwd.mkdir()
        receipt_path = self.folder / 'receipt.json'
        values = ['space value', 'quote"value', 'tail\\', '\u03a9']
        code = ("import json,os,sys; data={'args':sys.argv[1:], 'cwd':os.getcwd(),"
                "'stdin':sys.stdin.read()}; print('child-json '+json.dumps(data)); "
                "sys.stderr.write('child-stderr\\n')")
        argv = [sys.executable, str(HERE.with_name('run-with-heartbeat.py')),
                '--timeout-seconds', '5', '--status-json', str(receipt_path), '--cwd', str(cwd), '--',
                sys.executable, '-c', code, *values]
        write(self.folder / 'intent.json', {'argv': argv, 'timeout_seconds': 5, 'product': False})
        self.process = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=self.stdout, stderr=self.stderr)
        self.process.communicate(input=b'input-data\n', timeout=8)
        self.assertEqual(self.process.returncode, 0)
        self.stdout.flush()
        self.stderr.flush()
        lines = (self.folder / 'stdout.log').read_text(encoding='utf-8').splitlines()
        rows = [json.loads(line.removeprefix('child-json ')) for line in lines if line.startswith('child-json ')]
        self.assertEqual(rows, [{'args': values, 'cwd': str(cwd), 'stdin': 'input-data\n'}])
        self.assertIn(b'child-stderr', (self.folder / 'stderr.log').read_bytes())
        receipt = json.loads(receipt_path.read_bytes())
        self.assertEqual(receipt['classification'], 'success')
        self.assertEqual(set(receipt), {'schema_version', 'attempt_id', 'classification', 'command', 'detail',
            'elapsed_seconds', 'exit_code', 'finished_at', 'log_file', 'started_at'})
        self.result['pass'] = True


if __name__ == '__main__':
    if len(sys.argv) == 4 and sys.argv[1] == 'fixture':
        fixture(sys.argv[2], Path(sys.argv[3]))
    else:
        import argparse
        parser = argparse.ArgumentParser()
        parser.add_argument('--evidence-dir', type=Path, required=True)
        parser.add_argument('--case', choices=['test_cli_cwd_unicode_arguments_and_inherited_streams',
                                             'test_unsupported_job_list_refuses_without_creating_child'])
        options = parser.parse_args()
        if os.name != 'nt':
            raise SystemExit('Windows native cohort required, not a skipped pass')
        EVIDENCE = options.evidence_dir.resolve()
        EVIDENCE.mkdir(exist_ok=False)
        unittest.main(argv=[sys.argv[0], *(['Native.' + options.case] if options.case else []), '-v'])
