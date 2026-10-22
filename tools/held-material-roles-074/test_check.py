"""Synthetic check-driver controls; no compiler, WSL or original gate runs."""
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import check as c


class Controls(unittest.TestCase):
    def exercise(self, lane, response=None):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); (root / 'source').write_bytes(b'synthetic')
            with patch.object(c, 'ROOT', root), patch.object(c, 'OUTPUT', root), \
                    patch.object(c, 'FILES', ['source']), patch.object(c.subprocess, 'run',
                        side_effect=response, return_value=subprocess.CompletedProcess([], 0, b'ok', b'')) as child:
                c.run(lane)
                return child.call_args_list

    def test_host_only_fixed_offline_tests_compile_lint_and_format(self):
        calls = self.exercise('host')
        self.assertEqual(len(calls), 4)
        self.assertTrue(all(row.args[0][0] == 'cargo' for row in calls))
        self.assertIn('held_material_roles_074', calls[0].args[0])
        self.assertIn('--all-targets', calls[1].args[0])
        self.assertEqual(calls[2].args[0][-3:], ['--', '-D', 'warnings'])

    def test_linux_uses_the_selected_compiler_without_ignored_or_full_live_gate(self):
        for lane, version in (('194', '+1.94.0'), ('188', '+1.88.0')):
            calls = self.exercise(lane)
            self.assertEqual(len(calls), 4)
            for call in calls:
                argv = call.args[0]
                self.assertEqual(argv[:5], ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur'])
                self.assertIn(version, argv); self.assertNotIn('--ignored', argv)
                self.assertEqual(call.kwargs['timeout'], 1800)
            self.assertIn('held_material', calls[0].args[0]); self.assertIn('held_unit_projection', calls[1].args[0])

    def test_timeout_or_failure_stops_before_another_command_and_retains_no_resume(self):
        for outcome in (subprocess.TimeoutExpired(['synthetic'], 1, output=b'partial'),
                        subprocess.CompletedProcess([], 1, b'', b'refused')):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory); (root / 'source').write_bytes(b'synthetic')
                with patch.object(c, 'ROOT', root), patch.object(c, 'OUTPUT', root), patch.object(c, 'FILES', ['source']), \
                        patch.object(c.subprocess, 'run', side_effect=outcome if isinstance(outcome, Exception) else None,
                            return_value=outcome) as child:
                    with self.assertRaises(RuntimeError): c.run('host')
                    child.assert_called_once()
                    first = (root / 'host/result.json').read_bytes()
                    with self.assertRaises(FileExistsError): c.run('host')
                    child.assert_called_once()
                    self.assertEqual((root / 'host/result.json').read_bytes(), first)

    def test_source_drift_refuses_without_modifying_original_receipts(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); source = root / 'source'; source.write_bytes(b'before')
            def change(*args, **kwargs):
                source.write_bytes(b'after')
                return subprocess.CompletedProcess([], 0, b'ok', b'')
            with patch.object(c, 'ROOT', root), patch.object(c, 'OUTPUT', root), patch.object(c, 'FILES', ['source']), \
                    patch.object(c.subprocess, 'run', side_effect=change):
                with self.assertRaisesRegex(RuntimeError, 'Source changed'): c.run('host')
            self.assertIn(b'Source changed', (root / 'host/result.json').read_bytes())

    def test_receipts_are_exclusive(self):
        with tempfile.TemporaryDirectory() as directory:
            file = Path(directory) / 'first.json'; c.publish(file, dict(first=True))
            with self.assertRaises(FileExistsError): c.publish(file, {})


if __name__ == '__main__': unittest.main(verbosity=2)
