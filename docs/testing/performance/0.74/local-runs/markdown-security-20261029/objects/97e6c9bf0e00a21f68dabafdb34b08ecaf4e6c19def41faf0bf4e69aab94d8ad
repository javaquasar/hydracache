"""Offline mocks only: no elevated shell, WSL query or process workload."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock

import root_namespace as r


class RootNamespaceTests(unittest.TestCase):
    def test_nonadmin_never_dispatches_or_creates_lane(self):
        with mock.patch.object(r, 'administrator', return_value=False), \
                mock.patch.object(r.subprocess, 'run') as run, \
                mock.patch.object(r.subprocess, 'check_output') as check:
            with self.assertRaisesRegex(RuntimeError, 'no WSL command was run'):
                r.collect()
            run.assert_not_called()
            check.assert_not_called()

    def test_inventory_utf16_and_utf8_and_drift(self):
        for encoding in ('utf-8', 'utf-16le'):
            self.assertEqual(r.inventory('Ubuntu\r\ndocker-desktop\r\n'.encode(encoding)),
                             ['Ubuntu', 'docker-desktop'])
        for text in ('', 'Ubuntu', 'Ubuntu\ndocker-desktop\nHydraCache074Native',
                     'Ubuntu\nUbuntu\ndocker-desktop'):
            with self.assertRaises(ValueError):
                r.inventory(text.encode())

    def test_script_has_only_fixed_readonly_targets_and_guards(self):
        script = r.SCRIPT.decode('ascii')
        self.assertIn('for pid in 1 210 1157 1126 1112 920;', script)
        self.assertIn(r.identity.BOOT, script)
        self.assertIn("time:[4026531834]", script)
        self.assertNotIn('/proc/$pid/environ', script)
        self.assertNotIn('/proc/$pid/cmdline', script)
        for command in ('sudo ', 'nsenter ', 'mount ', 'kill ', 'systemctl ', 'date ',
                        'adjtimex ', 'clock_settime', ' >', ' >>'):
            self.assertNotIn(command, script)

    def test_shell_first_success_failure_timeout_and_missing_completion(self):
        for outcome in ('complete', 'failure', 'timeout', 'partial', 'reversed'):
            with self.subTest(outcome=outcome), tempfile.TemporaryDirectory() as directory:
                out = Path(directory)

                def invoke(argv, **kwargs):
                    self.assertEqual(argv, [r.WSL, '--debug-shell'])
                    self.assertEqual(kwargs['input'], r.SCRIPT)
                    self.assertEqual(kwargs['timeout'], 30)
                    if outcome == 'reversed':
                        kwargs['stdout'].write(b'HC074_ROOT_METADATA_END\nHC074_ROOT_METADATA_BEGIN\n')
                    else:
                        kwargs['stdout'].write(b'HC074_ROOT_METADATA_BEGIN\r\n')
                    if outcome == 'complete':
                        kwargs['stdout'].write(b'HC074_ROOT_METADATA_END\r\n')
                    if outcome == 'timeout':
                        raise subprocess.TimeoutExpired(argv, 30)
                    return subprocess.CompletedProcess(argv, 1 if outcome == 'failure' else 0)

                with mock.patch.object(r, 'OUT', out), mock.patch.object(r.subprocess, 'run', side_effect=invoke) as run:
                    if outcome == 'complete':
                        r.shell()
                    else:
                        with self.assertRaises((RuntimeError, ValueError)):
                            r.shell()
                    self.assertEqual(run.call_count, 1)
                    receipt = json.loads((out / 'debug-shell.json').read_bytes())
                    self.assertEqual(receipt['exit'], {'complete': 0, 'partial': 0, 'reversed': 0, 'failure': 1, 'timeout': 124}[outcome])
                    self.assertFalse(receipt['retry'])
                    with self.assertRaises(FileExistsError):
                        r.shell()
                    self.assertEqual(run.call_count, 1)

    def test_collector_stops_before_lookup_on_inventory_drift(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / 'lane'
            with mock.patch.object(r, 'OUT', out), mock.patch.object(r, 'administrator', return_value=True), \
                    mock.patch.object(r.base, 'OUT', out), mock.patch.object(r.base, 'sources', return_value={}), \
                    mock.patch.object(r.identity, 'verify'), mock.patch.object(r.subprocess, 'check_output',
                        side_effect=lambda *a, **kw: 'fake-commit' if kw.get('text') else b''), \
                    mock.patch.object(r.base, 'command') as command, mock.patch.object(r, 'shell') as shell:
                def log(*args):
                    (out / 'running-before.stdout.log').write_bytes(b'Ubuntu\n')
                command.side_effect = log
                with self.assertRaisesRegex(ValueError, 'Running distro inventory changed'):
                    r.collect()
                shell.assert_not_called()
                self.assertEqual(command.call_count, 1)

    def test_collector_retains_postflight_after_lookup_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / 'lane'
            with mock.patch.object(r, 'OUT', out), mock.patch.object(r, 'administrator', return_value=True), \
                    mock.patch.object(r.base, 'OUT', out), mock.patch.object(r.base, 'sources', return_value={}), \
                    mock.patch.object(r.identity, 'verify'), mock.patch.object(r.subprocess, 'check_output',
                        side_effect=lambda *a, **kw: 'fake-commit' if kw.get('text') else b''), \
                    mock.patch.object(r.base, 'command') as command, \
                    mock.patch.object(r, 'shell', side_effect=RuntimeError('first failure')) as shell:
                def log(name, *args):
                    (out / (name + '.stdout.log')).write_bytes(b'Ubuntu\ndocker-desktop\n')
                command.side_effect = log
                with self.assertRaisesRegex(RuntimeError, 'first failure'):
                    r.collect()
                self.assertEqual(command.call_count, 2)
                shell.assert_called_once()
                with self.assertRaises(FileExistsError):
                    r.collect()
                self.assertEqual(shell.call_count, 1)

    def test_collector_success_is_not_admission_and_writes_exact_stdin(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / 'lane'
            with mock.patch.object(r, 'OUT', out), mock.patch.object(r, 'administrator', return_value=True), \
                    mock.patch.object(r.base, 'OUT', out), mock.patch.object(r.base, 'sources', return_value={}), \
                    mock.patch.object(r.identity, 'verify'), mock.patch.object(r.subprocess, 'check_output',
                        side_effect=lambda *a, **kw: 'fake-commit' if kw.get('text') else b''), \
                    mock.patch.object(r.base, 'command') as command, mock.patch.object(r, 'shell') as shell, \
                    mock.patch('builtins.print') as printed:
                def log(name, *args):
                    (out / (name + '.stdout.log')).write_bytes(b'Ubuntu\ndocker-desktop\n')
                command.side_effect = log
                r.collect()
                shell.assert_called_once()
                self.assertEqual((out / 'stdin.sh').read_bytes(), r.SCRIPT)
                intent = json.loads((out / 'intent.json').read_bytes())
                self.assertEqual(intent['original_windows'], [300, 60, 10])
                self.assertTrue(all(intent[k] is False for k in r.identity.FALSE_FLAGS))
                self.assertFalse(json.loads(printed.call_args.args[0])['admission'])

    def test_dirty_worktree_refuses_before_proof_or_lookup(self):
        with mock.patch.object(r, 'administrator', return_value=True), \
                mock.patch.object(r.subprocess, 'check_output', return_value=b' M changed'), \
                mock.patch.object(r.base, 'sources') as sources, mock.patch.object(r, 'shell') as shell:
            with self.assertRaisesRegex(ValueError, 'clean worktree'):
                r.collect()
            sources.assert_not_called()
            shell.assert_not_called()

    def test_source_drift_after_lookup_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / 'lane'
            with mock.patch.object(r, 'OUT', out), mock.patch.object(r, 'administrator', return_value=True), \
                    mock.patch.object(r.base, 'OUT', out), mock.patch.object(r.base, 'sources', side_effect=[{}, {'drift': True}]), \
                    mock.patch.object(r.identity, 'verify'), mock.patch.object(r.subprocess, 'check_output',
                        side_effect=lambda *a, **kw: 'fake-commit' if kw.get('text') else b''), \
                    mock.patch.object(r.base, 'command') as command, mock.patch.object(r, 'shell') as shell:
                def log(name, *args):
                    (out / (name + '.stdout.log')).write_bytes(b'Ubuntu\ndocker-desktop\n')
                command.side_effect = log
                with self.assertRaisesRegex(ValueError, 'Postflight'):
                    r.collect()
                shell.assert_called_once()


if __name__ == '__main__':
    unittest.main()
