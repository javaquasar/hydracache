"""Mocks and temporary fake payloads only; never run the real lookup or WSL."""
import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

import root_namespace as original
import root_namespace_logged as logged

HERE = Path(__file__).resolve().parent
POWERSHELL = Path('C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe')


class PayloadTests(unittest.TestCase):
    def fixture(self, directory):
        journal = Path(directory) / 'journal'
        journal.mkdir()
        for name in ('stdout.log', 'stderr.log', 'launcher-errors.log'):
            (journal / name).touch()
        (journal / 'launcher-intent.json').write_text(json.dumps({
            'schema': 'hc074-preflight-journal-intent-v1', 'retry': False}), encoding='utf-8')
        return journal

    def test_original_source_and_unknown_record_stay_pinned(self):
        self.assertEqual(hashlib.sha256((HERE / 'root_namespace.py').read_bytes()).hexdigest(), logged.ORIGINAL_SHA)
        self.assertEqual(hashlib.sha256(logged.FIRST.read_bytes()).hexdigest(), logged.FIRST_SHA)
        first = json.loads(logged.FIRST.read_bytes())
        self.assertIsNone(first['exit_code'])
        self.assertIsNone(first['execution_confirmed'])
        self.assertFalse(first['replacement_attempt_authorized'])

    def test_success_uses_new_payload_lane_and_restores_original_globals(self):
        with tempfile.TemporaryDirectory() as directory:
            journal = self.fixture(directory)
            old = Path(directory) / 'old-absent'
            old_base = original.base.OUT
            with mock.patch.object(logged, 'JOURNAL', journal), mock.patch.object(original, 'OUT', old), \
                    mock.patch.object(original, 'collect') as collect:
                def check():
                    self.assertEqual(original.OUT, journal / 'lookup')
                    self.assertEqual(original.base.OUT, journal / 'lookup')
                collect.side_effect = check
                self.assertEqual(logged.execute(journal), 0)
                collect.assert_called_once()
                self.assertEqual(original.OUT, old)
                self.assertEqual(original.base.OUT, old_base)
                self.assertFalse(json.loads((journal / 'payload-result.json').read_bytes())['admission'])

    def test_actual_nonadmin_preflight_is_logged_without_dispatch(self):
        with tempfile.TemporaryDirectory() as directory:
            journal = self.fixture(directory)
            err = io.StringIO()
            with mock.patch.object(logged, 'JOURNAL', journal), \
                    mock.patch.object(original, 'OUT', Path(directory) / 'old-absent'), \
                    mock.patch.object(original, 'administrator', return_value=False), \
                    mock.patch.object(original.subprocess, 'run') as run, \
                    mock.patch.object(original.subprocess, 'check_output') as check, contextlib.redirect_stderr(err):
                self.assertEqual(logged.execute(journal), 1)
                run.assert_not_called()
                check.assert_not_called()
            self.assertIn('no WSL command was run', err.getvalue())
            self.assertEqual(json.loads((journal / 'payload-failure.json').read_bytes())['phase'], 'original-preflight-and-lookup')
            self.assertFalse((journal / 'lookup').exists())

    def test_missing_logs_changed_pin_and_appeared_first_artifacts_refuse(self):
        for case in ('missing-log', 'changed-first', 'old-artifacts'):
            with self.subTest(case=case), tempfile.TemporaryDirectory() as directory:
                journal = self.fixture(directory)
                old = Path(directory) / 'old'
                if case == 'missing-log':
                    (journal / 'stderr.log').unlink()  # Only an owned temporary fixture.
                if case == 'old-artifacts':
                    old.mkdir()
                with mock.patch.object(logged, 'JOURNAL', journal), mock.patch.object(original, 'OUT', old), \
                        mock.patch.object(logged, 'FIRST_SHA', 'bad' if case == 'changed-first' else logged.FIRST_SHA), \
                        mock.patch.object(original, 'collect') as collect, contextlib.redirect_stderr(io.StringIO()):
                    self.assertEqual(logged.execute(journal), 1)
                    collect.assert_not_called()
                    self.assertTrue((journal / 'payload-failure.json').is_file())

    def test_captured_subprocess_bytes_in_exception_chain_are_retained(self):
        with tempfile.TemporaryDirectory() as directory:
            journal = self.fixture(directory)
            def fail():
                try:
                    raise subprocess.CalledProcessError(9, ['git'], output=b'out\xff', stderr=b'err\x00')
                except subprocess.CalledProcessError as error:
                    raise ValueError('postflight also refused') from error
            with mock.patch.object(logged, 'JOURNAL', journal), \
                    mock.patch.object(original, 'OUT', Path(directory) / 'old-absent'), \
                    mock.patch.object(original, 'collect', side_effect=fail), contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(logged.execute(journal), 1)
            self.assertEqual((journal / 'exception-01-stdout.bin').read_bytes(), b'out\xff')
            self.assertEqual((journal / 'exception-01-stderr.bin').read_bytes(), b'err\x00')
            failure = json.loads((journal / 'payload-failure.json').read_bytes())
            self.assertIn('postflight also refused', failure['traceback'])
            self.assertEqual(len(failure['captured_outputs']), 2)

    def test_child_tee_keeps_success_bytes_args_timeouts_and_order(self):
        with tempfile.TemporaryDirectory() as directory:
            journal = Path(directory)
            result = subprocess.CompletedProcess(['git'], 0, stdout=b'dirty status\xff', stderr=b'warning')
            delegate = mock.Mock(return_value=result)
            run = logged.journaled_run(journal, delegate)
            self.assertIs(run(['git'], capture_output=True, timeout=15), result)
            delegate.assert_called_once_with(['git'], capture_output=True, timeout=15)
            self.assertEqual((journal / 'subprocess-00-stdout.bin').read_bytes(), result.stdout)
            self.assertEqual((journal / 'subprocess-00-stderr.bin').read_bytes(), result.stderr)
            self.assertEqual(json.loads((journal / 'subprocess-00-intent.json').read_bytes())['timeout'], 15)

    def test_child_tee_timeout_is_not_retried_or_reported_as_completed(self):
        with tempfile.TemporaryDirectory() as directory:
            journal = Path(directory)
            error = subprocess.TimeoutExpired(['git'], 30, output=b'partial', stderr=b'error')
            delegate = mock.Mock(side_effect=error)
            with self.assertRaises(subprocess.TimeoutExpired) as seen:
                logged.journaled_run(journal, delegate)(['git'], timeout=30)
            self.assertIs(seen.exception, error)
            delegate.assert_called_once_with(['git'], timeout=30)
            self.assertFalse((journal / 'subprocess-00-result.json').exists())
            self.assertEqual((journal / 'subprocess-00-stdout.bin').read_bytes(), b'partial')

    def test_wrong_journal_argument_does_not_write_elsewhere(self):
        with tempfile.TemporaryDirectory() as directory, contextlib.redirect_stderr(io.StringIO()):
            other = Path(directory)
            self.assertEqual(logged.execute(other), 1)
            self.assertEqual(list(other.iterdir()), [])

    def test_real_dirty_git_check_captured_output_is_preserved_once(self):
        with tempfile.TemporaryDirectory() as directory:
            journal = self.fixture(directory)
            delegate = mock.Mock(return_value=subprocess.CompletedProcess(['git'], 0, stdout=b' M dirty-file\n', stderr=None))
            with mock.patch.object(logged, 'JOURNAL', journal), \
                    mock.patch.object(original, 'OUT', Path(directory) / 'old-absent'), \
                    mock.patch.object(original, 'administrator', return_value=True), \
                    mock.patch.object(original.subprocess, 'run', delegate), \
                    mock.patch.object(original.base, 'sources') as sources, contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(logged.execute(journal), 1)
                delegate.assert_called_once()
                sources.assert_not_called()
            self.assertEqual((journal / 'subprocess-00-stdout.bin').read_bytes(), b' M dirty-file\n')
            self.assertIn('clean worktree', (journal / 'payload-failure.json').read_text())


@unittest.skipUnless(os.name == 'nt' and POWERSHELL.is_file(), 'Windows PowerShell fixture control')
class LauncherTests(unittest.TestCase):
    def fixture(self, directory, source):
        root = Path(directory) / 'workspace with spaces'
        tool = root / 'tools/wsl-clock-trace-074'
        tool.mkdir(parents=True)
        (root / 'target').mkdir()
        shutil.copyfile(HERE / 'root_namespace_logged.ps1', tool / 'root_namespace_logged.ps1')
        (tool / 'root_namespace_logged.py').write_text(source, encoding='utf-8')
        return root, tool, root / 'target/wsl-clock-root-namespace-logged-20261023'

    def run_launcher(self, tool, env=None):
        return subprocess.run([str(POWERSHELL), '-NoProfile', '-ExecutionPolicy', 'Bypass',
                               '-File', str(tool / 'root_namespace_logged.ps1')],
                              env=env, capture_output=True, timeout=25, check=False)

    def test_both_pipes_large_utf8_and_exclusive_first_result(self):
        source = "import sys\nprint('Журнал')\nsys.stdout.write('x'*200000)\nsys.stderr.write('y'*200000)\nraise SystemExit(7)\n"
        with tempfile.TemporaryDirectory() as directory:
            root, tool, journal = self.fixture(directory, source)
            result = self.run_launcher(tool)
            self.assertEqual(result.returncode, 7, result.stderr.decode(errors='replace'))
            command = json.loads((journal / 'launcher-command.json').read_bytes())
            self.assertEqual(Path(command['executable']).resolve(), Path(sys.executable).resolve())
            self.assertTrue(json.loads((journal / 'launcher-started.json').read_bytes())['child_started'])
            self.assertEqual((journal / 'stdout.log').read_bytes(), 'Журнал\r\n'.encode('utf-8') + b'x' * 200000)
            self.assertEqual((journal / 'stderr.log').read_bytes(), b'y' * 200000)
            before = {p.name: p.read_bytes() for p in journal.iterdir()}
            self.assertNotEqual(self.run_launcher(tool).returncode, 0)
            self.assertEqual(before, {p.name: p.read_bytes() for p in journal.iterdir()})

    def test_interpreter_resolution_failure_has_logs_before_any_python(self):
        with tempfile.TemporaryDirectory() as directory:
            root, tool, journal = self.fixture(directory, "raise AssertionError('must not run')")
            env = dict(os.environ, PATH=str(root))
            result = self.run_launcher(tool, env)
            self.assertNotEqual(result.returncode, 0)
            for name in ('stdout.log', 'stderr.log', 'launcher-errors.log', 'launcher-intent.json'):
                self.assertTrue((journal / name).is_file())
            self.assertEqual(json.loads((journal / 'launcher-failure.json').read_bytes())['stage'], 'resolve-python')
            self.assertFalse(json.loads((journal / 'launcher-failure.json').read_bytes())['child_started'])
            self.assertTrue((journal / 'launcher-errors.log').read_bytes())

    def test_python_import_failure_stderr_survives_closed_child(self):
        with tempfile.TemporaryDirectory() as directory:
            root, tool, journal = self.fixture(directory, 'import nonexistent_hc074_fixture_module\n')
            self.assertNotEqual(self.run_launcher(tool).returncode, 0)
            self.assertIn(b'ModuleNotFoundError', (journal / 'stderr.log').read_bytes())
            self.assertNotEqual(json.loads((journal / 'launcher-result.json').read_bytes())['child_exit'], 0)


if __name__ == '__main__':
    unittest.main()
