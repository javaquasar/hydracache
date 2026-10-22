"""Offline packet-negative controls; no native image, clock or service actions."""
import copy
from pathlib import Path
import tempfile
import unittest
import review


class Controls(unittest.TestCase):
    def test_exact_inventory_then_byte_and_membership_mutations(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'first').write_bytes(b'original')
            rows = review.inventory(root)
            review.verify_inventory(root, rows)
            (root / 'first').write_bytes(b'mutation')
            with self.assertRaises(ValueError):
                review.verify_inventory(root, rows)
            (root / 'first').write_bytes(b'original')
            (root / 'second').write_bytes(b'extra')
            with self.assertRaises(ValueError):
                review.verify_inventory(root, rows)

    def test_unsafe_paths_refuse(self):
        for path in ('../other', '/absolute', 'C:/drive', 'a\\b', 'a//b', './a', ''):
            with self.assertRaises(ValueError):
                review.member(dict(path=path, bytes=0, sha256='0' * 64))

    def test_duplicate_members_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'only').write_bytes(b'')
            rows = review.inventory(root)
            with self.assertRaises(ValueError):
                review.verify_inventory(root, rows + copy.deepcopy(rows))

    def test_invalid_digest_and_size_refuse(self):
        for size, digest in ((-1, '0' * 64), (0, 'z' * 64), (0, '0' * 63)):
            with self.assertRaises(ValueError):
                review.member(dict(path='only', bytes=size, sha256=digest))


if __name__ == '__main__':
    unittest.main(verbosity=2)
