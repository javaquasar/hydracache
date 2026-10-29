"""Pure refusal controls; no preflight, Git, test dispatcher or live observation."""
import tempfile
import unittest
from pathlib import Path

import gnu_frontier as g


class Controls(unittest.TestCase):
    def transcript(self):
        return ''.join(f'Running tests/{name}.rs (owned-target)\n'
            f'test result: ok. {count} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\n'
            for name, count in g.TARGETS.items()).encode()

    def test_exact_command_contract(self):
        argv = g.command()
        self.assertIn('--offline', argv)
        self.assertIn('--locked', argv)
        self.assertEqual(argv.count('--test'), 5)
        self.assertEqual(argv[-2:], ['--', '--test-threads=4'])
        self.assertEqual(sum(g.TARGETS.values()), 31)

    def test_complete_receipt_only(self):
        raw = self.transcript()
        self.assertTrue(g.complete(0, raw))
        for exit_code, bad in [(1, raw), (0, b''), (0, raw + raw),
                (0, raw.replace(b'9 passed', b'8 passed')), (0, raw.replace(b'0 ignored', b'1 ignored')),
                (0, raw.replace(b'0 filtered out', b'1 filtered out')),
                (0, raw.replace(b'frontier_current_view_074', b'unknown'))]:
            with self.subTest(exit=exit_code, raw=bad[:64]):
                self.assertFalse(g.complete(exit_code, bad))

    def test_only_directional_text_eol_change(self):
        self.assertEqual(g.reviewed_bytes('x.rs', b'a\n', b'a\r\n'), 'reviewed-crlf-to-lf')
        self.assertEqual(g.reviewed_bytes('object', b'a\r\n', b'a\r\n'), 'raw-exact')
        for name, native, archive in [('object', b'a\n', b'a\r\n'), ('x.rs', b'b\n', b'a\r\n'),
                ('x.rs', b'a\r\n', b'a\n'), ('x.rs', b'\0\n', b'\0\r\n')]:
            with self.subTest(name=name, native=native):
                with self.assertRaises(ValueError):
                    g.reviewed_bytes(name, native, archive)

    def test_archived_object_is_data_and_hash_bound(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            objects = root / 'packet/objects'
            objects.mkdir(parents=True)
            raw = b'raise RuntimeError("never execute")'
            digest = g.sha(raw)
            (objects / digest).write_bytes(raw)
            doc = {'members': {'input': {'sha256': digest, 'bytes': len(raw)}}}
            self.assertEqual(g.member(root, 'packet', doc, 'input'), raw)
            (objects / digest).write_bytes(b'changed')
            with self.assertRaises(ValueError):
                g.member(root, 'packet', doc, 'input')
            doc['members']['input']['sha256'] = '../escape'
            with self.assertRaises(ValueError):
                g.member(root, 'packet', doc, 'input')

    def test_duplicate_keys_refuse(self):
        with self.assertRaises(ValueError):
            g.decode(b'{"source":1,"source":2}')


if __name__ == '__main__':
    unittest.main()
