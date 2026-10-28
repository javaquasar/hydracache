"""A dependency update cannot grant a blanket source-catalog refresh."""
from pathlib import Path
import copy
import tempfile
import unittest
from unittest.mock import patch

import security_lock as s


def original_sys_successor():
    root = Path(__file__).resolve().parents[2]
    raw = (root/'docs/testing/performance/0.74/local-runs/security-review-20261028/objects'/s.CURRENT).read_bytes()
    if s.digest(raw) != s.CURRENT:
        raise ValueError('immutable original sys successor fixture')
    return raw


class SecurityLock(unittest.TestCase):
    def test_exact_current_reconstructs_complete_predecessor(self):
        previous = s.predecessor(original_sys_successor())
        self.assertEqual(s.digest(previous), s.PREVIOUS)
        self.assertEqual(previous.count(s.OLD), 1)
        self.assertNotIn(s.NEW, previous)

    def test_same_line_ending_rule_as_inherited_closure(self):
        raw = original_sys_successor()
        self.assertEqual(s.predecessor(raw), s.predecessor(raw.replace(b'\r\n', b'\n').replace(b'\n', b'\r\n')))

    def test_mutated_version_checksum_other_body_and_old_lock_refuse(self):
        raw = original_sys_successor()
        for bad in (raw+b'\n', raw.replace(b'2.1.2+zstd.1.5.7', b'2.1.3+zstd.1.5.7'),
                    raw.replace(s.NEW, s.OLD), raw.replace(b'481ada9c', b'581ada9c')):
            with self.subTest(size=len(bad)), self.assertRaises(ValueError):
                s.predecessor(bad)

    def test_exact_delta_requires_complete_predecessor_even_with_current_digest(self):
        raw = b'prefix\n' + s.NEW + b'suffix\n'
        with patch.object(s, 'CURRENT', s.digest(raw)), self.assertRaises(ValueError):
            s.predecessor(raw)

    def test_duplicate_record_refuses_even_with_current_digest(self):
        raw = s.NEW * 2
        with patch.object(s, 'CURRENT', s.digest(raw)), self.assertRaises(ValueError):
            s.predecessor(raw)

    def test_only_exact_lock_catalog_entry_can_change(self):
        data = dict(auxiliary={'Cargo.lock': s.PREVIOUS, 'deny.toml': 'original'},
                    sources={'original.rs': 'original'}, targets=['original'])
        before = copy.deepcopy(data)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'Cargo.lock').write_bytes(original_sys_successor())
            result = s.reviewed_inputs(root, data)
        self.assertEqual(data, before)
        expected = copy.deepcopy(before)
        expected['auxiliary']['Cargo.lock'] = s.CURRENT
        self.assertEqual(result, expected)
        data['auxiliary']['Cargo.lock'] = 'unknown'
        with self.assertRaises(ValueError):
            # This refusal is before reading the now-retired temporary fixture.
            s.reviewed_inputs(root, data)

    def test_wrong_root_does_not_read_outside_workspace(self):
        with tempfile.TemporaryDirectory() as folder, self.assertRaises((ValueError, FileNotFoundError)):
            s.reviewed_inputs(Path(folder), dict(auxiliary={'Cargo.lock': s.PREVIOUS}))
