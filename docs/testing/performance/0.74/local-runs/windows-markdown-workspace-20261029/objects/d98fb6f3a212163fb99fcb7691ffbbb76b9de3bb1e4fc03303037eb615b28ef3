"""Configuration journal controls with fake tracefs, never live instrumentation."""
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch, Mock

import capture
import einval

GROUP = 'hc074clock_' + 'b' * 32


class Configuration(unittest.TestCase):
    def read(self, path):
        return [json.loads(line) for line in path.read_text().splitlines()]

    def fixture(self, root, fail=None):
        trace = root / 'trace'
        (trace / 'instances').mkdir(parents=True)
        (trace / 'kprobe_events').write_text('')
        calls = []
        def write(path, value, append=False):
            calls.append((path, value, append))
            if fail and path.name == fail:
                raise OSError(22, 'Invalid argument', str(path))
            if path == trace / 'kprobe_events':
                self.assertTrue(append)
                group, name = capture.key(value).split('/')
                event = trace / 'instances' / group / 'events' / group / name
                event.mkdir(parents=True)
                (event / 'format').write_text('fake format\n')
            path.parent.mkdir(parents=True, exist_ok=True)
            with path.open('a' if append else 'w') as stream:
                stream.write(value + '\n')
        return SimpleNamespace(TRACE=trace, key=capture.key, normalized=capture.normalized, write=write), calls

    def test_configuration_order_and_no_buffer_activation(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            cap, calls = self.fixture(root)
            journal = einval.Journal(root / 'operations.jsonl')
            einval.configure(cap, GROUP, capture.definitions(GROUP), journal)
            steps = self.read(journal.path)
            self.assertEqual([s['phase'] for s in steps], ['before', 'after'] * (len(steps) // 2))
            self.assertEqual([s['sequence'] for s in steps[::2]], list(range(1, len(steps) // 2 + 1)))
            self.assertEqual([p.name for p, _, _ in calls[:4]], ['tracing_on', 'buffer_size_kb', 'trace_clock', 'stacktrace'])
            self.assertFalse(any(p.name == 'tracing_on' and v != '0' for p, v, _ in calls))
            self.assertEqual(sum(p.name == 'kprobe_events' for p, _, _ in calls), 7)

    def test_first_failure_path_traceback_and_no_later_operation(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            cap, calls = self.fixture(root, fail='stacktrace')
            journal = einval.Journal(root / 'operations.jsonl')
            with self.assertRaises(OSError):
                einval.configure(cap, GROUP, capture.definitions(GROUP), journal)
            steps = self.read(journal.path)
            self.assertEqual(steps[-1]['phase'], 'failed')
            self.assertEqual(steps[-1]['sequence'], 5)
            self.assertEqual(steps[-1]['errno'], 22)
            self.assertEqual(steps[-1]['path'], str(cap.TRACE / 'instances' / GROUP / 'options/stacktrace'))
            self.assertIn('Invalid argument', steps[-1]['traceback'])
            self.assertEqual(len(calls), 4)
            self.assertFalse(any(p.name == 'kprobe_events' for p, _, _ in calls))

    def test_durable_before_record_must_precede_effect(self):
        with tempfile.TemporaryDirectory() as folder:
            journal = einval.Journal(Path(folder) / 'operations.jsonl')
            observed = []
            def effect():
                observed.extend(self.read(journal.path))
            journal.perform('owned', '/own/path', '1', effect)
            self.assertEqual([s['phase'] for s in observed], ['before'])
            untouched = Mock()
            with patch.object(einval.os, 'fsync', side_effect=OSError('injected evidence write failure')):
                with self.assertRaises(OSError):
                    journal.perform('must-not-run', '/own/other', None, untouched)
            untouched.assert_not_called()

    def test_read_failure_is_recorded_and_not_retried(self):
        with tempfile.TemporaryDirectory() as folder:
            journal = einval.Journal(Path(folder) / 'operations.jsonl')
            effect = Mock(side_effect=OSError(22, 'read failed', '/own/format'))
            with self.assertRaises(OSError):
                journal.perform('read', '/own/format', None, effect)
            effect.assert_called_once()
            self.assertEqual([s['phase'] for s in self.read(journal.path)], ['before', 'failed'])


if __name__ == '__main__':
    unittest.main()
