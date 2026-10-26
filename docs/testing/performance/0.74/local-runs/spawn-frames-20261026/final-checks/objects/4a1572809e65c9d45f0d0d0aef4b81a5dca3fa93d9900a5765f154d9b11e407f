"""Integrity and independent source-boundary controls; never run retained code."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('spawn_packet_verify', HERE/'verify.py')
subject = importlib.util.module_from_spec(spec)
spec.loader.exec_module(subject)


class Integrity(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.members = subject.load(HERE)

    def test_positive(self):
        result = subject.audit(copy.deepcopy(self.members))
        self.assertEqual((result['new_models'], result['combined_cases'], result['native_attempts']), (15, 130, 0))

    def test_index_missing_and_byte_corruption(self):
        with patch.object(Path, 'read_bytes', return_value=b'{}'), self.assertRaisesRegex(ValueError, 'index differs'):
            subject.load(HERE)
        with patch.object(Path, 'read_bytes', side_effect=FileNotFoundError('missing')), self.assertRaises(FileNotFoundError):
            subject.load(HERE)
        index = json.loads((HERE/'index.json').read_bytes())
        entry, read = next(iter(index['members'].values())), Path.read_bytes
        target = HERE/'objects'/entry['sha256']
        with patch.object(Path, 'read_bytes', lambda p: read(p)+b'x' if p == target else read(p)), \
             self.assertRaisesRegex(ValueError, 'raw member differs'):
            subject.load(HERE)

    def test_provenance_source_receipt_and_closed_protocol_faults(self):
        for key in ('baseline/capture-leaves-native-20261026-index.json',
                    'baseline/leaf-attribution-20261026-index.json', 'qualification-manifest.toml',
                    subject.PREFIX+'spawn_frames.py'):
            members = copy.deepcopy(self.members)
            members[key] += b' '
            with self.assertRaises(ValueError):
                subject.audit(members)
        members = copy.deepcopy(self.members)
        key = 'checks/spawn-frames-models-first/result.json'
        row = json.loads(members[key])
        row['source_unchanged'] = False
        members[key] = json.dumps(row).encode()
        with self.assertRaises(ValueError):
            subject.audit(members)

    def compare(self, candidate):
        subject.compare_bodies(self.members['source/scripts/ci/watchdog_windows_job.py'], candidate,
                               self.members[subject.PREFIX+'capture_leaves.py'], candidate)

    def test_nontelemetry_mutation_cannot_be_stripped(self):
        raw = self.members[subject.PREFIX+'spawn_frames.py']
        for old, new in ((b'previous != 1', b'previous != 2'),
                         (b'time.monotonic() < deadline', b'time.monotonic() < deadline + 1'),
                         (b'owner.child = child', b'owner.child = None'),
                         (b'0x2000D', b'0x20002'),
                         (b"raise c.WinError(c.get_last_error())", b"raise RuntimeError('hidden')")):
            self.assertIn(old, raw)
            with self.assertRaisesRegex(ValueError, 'operational AST differs'):
                self.compare(raw.replace(old, new))

    def test_mark_relocation_and_unsupported_mark_refuse(self):
        raw = self.members[subject.PREFIX+'spawn_frames.py']
        mark = b"            self.spawn_frames.end('create_to_retention')\n"
        assignment = b'        self._child = value  # No added observation before original retention.\n'
        moved = raw.replace(mark, b'').replace(assignment, b"        self.spawn_frames.end('create_to_retention')\n"+assignment)
        with self.assertRaises(ValueError):
            self.compare(moved)
        with self.assertRaisesRegex(ValueError, 'unsupported stripped mark'):
            self.compare(raw.replace(b"begin('preparation')", b"begin('foreign')"))
        mark = b"            owner.spawn_frames.begin('create_to_retention')\n"
        statement = b'            child = WindowsChild(self, info.process, info.thread, info.pid)\n'
        moved = raw.replace(mark, b'').replace(statement, mark+statement)
        with self.assertRaisesRegex(ValueError, 'successful create/publication gap'):
            self.compare(moved)


if __name__ == '__main__':
    unittest.main()
