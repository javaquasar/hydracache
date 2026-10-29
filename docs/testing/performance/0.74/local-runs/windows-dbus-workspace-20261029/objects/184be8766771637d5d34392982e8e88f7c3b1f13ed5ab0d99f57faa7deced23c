"""Offline receipt schema/refusal controls, no WSL or mutation commands."""
from pathlib import Path
import tempfile
import unittest
import seal


class Controls(unittest.TestCase):
    def test_missing_receipts_cannot_supply_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(ValueError, 'Missing'): seal.call_rows(Path(directory))

    def test_identity_is_from_exact_retained_receipts_not_version_or_success_flags(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); (root / 'calls').mkdir()
            for index, output in enumerate(('s ":1.0"\n', 'u 1\n', 'u 0\n', 'version\n', 'missing\n')):
                prefix = root / 'calls' / f'{index:03}'
                for suffix, value in (('.argv', 'fixed'), ('.exit', '0'), ('.stdout', output), ('.stderr', '')):
                    prefix.with_suffix(suffix).write_text(value)
            self.assertEqual(len(seal.call_rows(root)), 5)
            (root / 'calls/001.stdout').write_text('u 74\n')
            with self.assertRaisesRegex(ValueError, 'identity differs'): seal.call_rows(root)


if __name__ == '__main__': unittest.main(verbosity=2)
