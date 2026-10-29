import tempfile
import json
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch, Mock
import check as c


class Controls(unittest.TestCase):
    def test_docs_v2_selects_registered_xtask_and_preserves_the_failed_driver(self):
        original = dict(c.commands('docs'))['registry']
        fixed = dict(c.commands('docs-v2'))['registry']
        self.assertEqual(original[-1], 'gated-test-registry-check')
        self.assertEqual(fixed[-1], 'gated-test-check')
        self.assertEqual(original[:-1], fixed[:-1])

    def test_fixed_lanes_do_not_dispatch_or_start_live_attempts(self):
        for lane in ('host', '194', '188'):
            argv = c.commands(lane)
            self.assertEqual(len(argv), 4 if lane == 'host' else 5)
            for _, command in argv:
                self.assertFalse(set(command) & {'--ignored', 'ssh', 'systemctl', 'run', 'bench', 'dispatch'})
                self.assertIn('--locked' if 'fmt' not in command else '--check', command)
        with self.assertRaises(ValueError): c.commands('server')

    def test_exclusive_receipts_cannot_overwrite_a_first_outcome(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'first.json'
            c.publish(path, {'exit': 101})
            with self.assertRaises(FileExistsError): c.publish(path, {'exit': 0})
            self.assertIn('101', path.read_text())

    def test_existing_lane_refuses_before_subprocess(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(c, 'OUTPUT', Path(tmp)):
            (Path(tmp) / '194').mkdir()
            with patch.object(c.subprocess, 'run') as run:
                with self.assertRaises(FileExistsError): c.run('194')
                run.assert_not_called()

    def test_success_failure_timeout_and_source_drift_are_retained_once(self):
        for case in ('success', 'failure', 'timeout', 'drift'):
            with self.subTest(case=case), tempfile.TemporaryDirectory() as tmp:
                result = Mock(stdout=b'out', stderr=b'err', returncode=101 if case == 'failure' else 0)
                error = subprocess.TimeoutExpired(['fixed'], 1800, output=b'partial', stderr=b'timeout')
                with patch.object(c, 'OUTPUT', Path(tmp)), patch.object(c, 'FILES', ['fixed.rs']), \
                     patch.object(c, 'sha', side_effect=['before', 'after' if case == 'drift' else 'before']), \
                     patch.object(c, 'commands', return_value=[('one', ['fixed']), ('two', ['fixed'])]), \
                     patch.object(c.subprocess, 'run', side_effect=error if case == 'timeout' else None,
                                  return_value=result) as run:
                    if case == 'success': c.run('host')
                    else:
                        with self.assertRaises(RuntimeError): c.run('host')
                folder = Path(tmp) / 'host'
                receipt = json.loads((folder / 'result.json').read_bytes())
                self.assertEqual(receipt['failure'] is None, case == 'success')
                self.assertFalse(any(receipt[key] for key in ('product', 'server', 'original_attempt')))
                self.assertEqual(run.call_count, 1 if case in ('failure', 'timeout') else 2)
                row = json.loads((folder / 'one.json').read_bytes())
                self.assertEqual(row['exit'], None if case == 'timeout' else result.returncode)
                self.assertEqual(c.sha(folder / 'one.stdout.log'), row['stdout_sha256'])


if __name__ == '__main__': unittest.main()
