import unittest

import collision


class CollisionControls(unittest.TestCase):
    def test_first_candidate_is_reproducible(self):
        raw = b'tree ' + b'0' * 40 + b'\n\nfixture\n'
        first = collision.candidate(raw, 0)
        value = collision.collide(raw, collision.oid(first)[:4], 1)
        self.assertEqual(bytes.fromhex(value['body_hex']), first)
        self.assertEqual(value['oid'], collision.oid(first))
        self.assertEqual(value['ordinal'], 0)
        self.assertFalse(value['authority'])

    def test_exhaustion_does_not_retry(self):
        raw = b'fixture'
        prefix = '0000' if not collision.oid(collision.candidate(raw, 0)).startswith('0000') else '1111'
        with self.assertRaisesRegex(ValueError, 'exhausted'):
            collision.collide(raw, prefix, 1)

    def test_scope_and_budget_refuse(self):
        for raw, prefix, limit in [(b'', 'abcd', 1), (b'x' * 65537, 'abcd', 1),
                                   (b'x', 'abc', 1), (b'x', 'ABCD', 1),
                                   (b'x', 'abcd', 0), (b'x', 'abcd', (1 << 20) + 1)]:
            with self.subTest(prefix=prefix, limit=limit):
                with self.assertRaises(ValueError):
                    collision.collide(raw, prefix, limit)


if __name__ == '__main__':
    unittest.main()
