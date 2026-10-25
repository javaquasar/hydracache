import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

import verify as reader


class Controls(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        packet = Path(__file__).resolve().parent
        shutil.copy2(packet / 'index.json', self.root / 'index.json')
        shutil.copytree(packet / 'objects', self.root / 'objects')

    def rewrite(self, name, mutate):
        index = json.loads((self.root / 'index.json').read_bytes())
        row = index['members'][name]
        raw = mutate((self.root / 'objects' / row['sha256']).read_bytes())
        digest = reader.sha(raw)
        (self.root / 'objects' / digest).write_bytes(raw)
        index['members'][name] = dict(sha256=digest, bytes=len(raw))
        body = (json.dumps(index, indent=2, sort_keys=True) + '\n').encode()
        (self.root / 'index.json').write_bytes(body)
        return reader.sha(body)

    def test_exact_packet_replays_without_code_execution(self):
        result = reader.verify(self.root)
        self.assertEqual((result['paths'], result['edits']), (5, 38))
        self.assertFalse(result['code_executed'])

    def test_raw_corruption_refuses(self):
        index = json.loads((self.root / 'index.json').read_bytes())
        row = next(iter(index['members'].values()))
        path = self.root / 'objects' / row['sha256']
        path.write_bytes(path.read_bytes() + b'x')
        with self.assertRaisesRegex(ValueError, 'raw member'):
            reader.verify(self.root)

    def test_index_cannot_self_refresh(self):
        self.rewrite('checks/successor-view-first-red/stdout.log', lambda _: b'passed')
        with self.assertRaisesRegex(ValueError, 'fixed index'):
            reader.verify(self.root)

    def test_rehashed_index_cannot_suppress_first_refusal(self):
        digest = self.rewrite('checks/successor-view-first-red/stdout.log', lambda _: b'passed')
        with patch.object(reader, 'INDEX_SHA256', digest):
            with self.assertRaisesRegex(ValueError, 'first canary refusal'):
                reader.verify(self.root)

    def test_rules_pin_is_independent_of_index(self):
        digest = self.rewrite('source-final/tools/source-successor-view-074/rules.json', lambda raw: raw + b' ')
        with patch.object(reader, 'INDEX_SHA256', digest):
            with self.assertRaisesRegex(ValueError, 'independent rules'):
                reader.verify(self.root)

    def test_producer_cannot_be_substituted_after_rehash(self):
        path = 'producer/a67a5299ab335424e9b11330dc06bd749ea9b859/docs/COMPAT.md'
        digest = self.rewrite(path, lambda raw: raw + b'unknown change')
        with patch.object(reader, 'INDEX_SHA256', digest):
            with self.assertRaisesRegex(ValueError, 'fixed producer body'):
                reader.verify(self.root)


if __name__ == '__main__':
    unittest.main()
