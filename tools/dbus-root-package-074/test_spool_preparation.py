"""Inert byte/text whole-preimage controls; no Cargo or fixture execution."""
import unittest
from unittest import mock

import spool_context as s
import format_spool as f


class Preimage(unittest.TestCase):
    def test_byte_and_text_preimages_preserve_every_other_byte(self):
        for prefix, before, after, suffix in ((b'\r\n', b'old', b'new', b'\n'),
                                              ('\n', 'old', 'new', '\r\n')):
            self.assertEqual(s.replace(prefix+before+suffix, before, after), prefix+after+suffix)

    def test_missing_or_duplicate_preimage_refuses_before_any_write(self):
        for body in (b'none', b'oldold', 'none', 'oldold'):
            before = b'old' if isinstance(body, bytes) else 'old'
            with self.assertRaises(ValueError):
                s.replace(body, before, before)

    def test_final_metadata_hashes_fixture_bytes_independently_of_helper_text(self):
        value = s.fixture_result(b'held before', b'held after')
        self.assertEqual(value['before_sha256'], s.c.sha(b'held before'))
        self.assertEqual(value['after_sha256'], s.c.sha(b'held after'))
        self.assertFalse(value['runtime_changed'] or value['trust_pins_changed'] or value['release_admitted'])
        self.assertEqual(value['original_windows_seconds'], [300,60,10])

    def test_formatter_uses_workspace_edition_and_original_owned_mechanism(self):
        owner = mock.Mock()
        owner._owned.return_value = dict(exit=0)
        with mock.patch.dict(f.os.environ, {}, clear=False), \
             mock.patch.object(f.c, 'windows_environment', return_value=(None, {'RUSTFMT':'not-executed'})), \
             mock.patch.object(f.c, 'owned', return_value=owner), \
             mock.patch.object(f.c, 'write') as write:
            with self.assertRaises(SystemExit) as exit:
                f.worker()
        self.assertEqual(exit.exception.code, 0)
        argv = owner._owned.call_args.args[1]
        self.assertEqual(argv, ['not-executed', '--edition', '2021', '--config', 'skip_children=true', str(f.c.ROOT/f.TEST)])
        self.assertEqual(owner._owned.call_args.kwargs, dict(bound=60))
        write.assert_called_once_with(f.OUT, 'execution.json', dict(exit=0))


if __name__ == '__main__':
    unittest.main()
