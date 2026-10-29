import json
from pathlib import Path
import tempfile
import unittest

import recording as r


class Recording(unittest.TestCase):
    def row(self):
        return dict(id='seeded/0', exit=0, timeout=False,
                    value=dict(id='seeded/0', parsed=True, text='😀 Кириллица'))

    def test_partial_record_recovers_only_preserved_native_response(self):
        row = self.row()
        native = json.dumps(row, indent=2, ensure_ascii=False).replace('\n', '\r\n')
        partial = native[:native.index('😀')].encode('cp1252')
        self.assertEqual(r.recover(row['id'], partial, json.dumps(row['value']).encode(), b''), row)

    def test_exit_timeout_order_and_identity_are_not_inferred(self):
        row = self.row()
        for changed in (dict(row, exit=1), dict(row, exit=False), dict(row, timeout=True),
                        dict(row, id='other'), dict(value=row['value'], id=row['id'], exit=0, timeout=False)):
            with self.assertRaises(ValueError):
                r.recover(row['id'], json.dumps(changed, indent=2).replace('\n', '\r\n').encode(),
                          json.dumps(row['value']).encode(), b'')

    def test_bad_raw_output_and_stderr_refuse(self):
        row = self.row()
        native = json.dumps(row, indent=2).replace('\n', '\r\n').encode()
        for stdout, stderr in ((b'{}', b''), (b'{} {}', b''), (b'\xff', b''),
                               (json.dumps(row['value']).encode(), b'panic')):
            with self.assertRaises((ValueError, UnicodeError)):
                r.recover(row['id'], native, stdout, stderr)

    def test_utf8_exclusive_writer_preserves_unicode(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)/'record.json'
            r.write_record(path, self.row())
            self.assertEqual(json.loads(path.read_bytes()), self.row())
            self.assertIn('😀'.encode(), path.read_bytes())
            with self.assertRaises(FileExistsError):
                r.write_record(path, self.row())
