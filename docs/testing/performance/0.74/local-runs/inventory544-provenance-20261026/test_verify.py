import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

import verify as reader

PACKET = Path(__file__).resolve().parent


class Controls(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name) / 'packet'
        self.root.mkdir()
        shutil.copy2(PACKET / 'index.json', self.root / 'index.json')
        shutil.copytree(PACKET / 'objects', self.root / 'objects')

    def rewrite(self, name, mutate):
        index = json.loads((self.root / 'index.json').read_bytes())
        row = index['members'][name]
        body = mutate((self.root / 'objects' / row['sha256']).read_bytes())
        digest = reader.sha(body)
        (self.root / 'objects' / digest).write_bytes(body)
        index['members'][name] = dict(sha256=digest, bytes=len(body))
        raw = (json.dumps(index, indent=2, sort_keys=True)+'\n').encode()
        (self.root / 'index.json').write_bytes(raw)
        return reader.sha(raw)

    def test_exact_historical_data_replays_without_code_execution(self):
        result = reader.verify(self.root)
        self.assertEqual((result['previous'], result['current'], result['unchanged']), (530, 544, 527))
        self.assertFalse(result['code_executed'])

    def test_raw_corruption_refuses(self):
        index = json.loads((self.root / 'index.json').read_bytes())
        path = self.root / 'objects' / index['members']['phase-ledger.json']['sha256']
        path.write_bytes(path.read_bytes()+b'changed')
        with self.assertRaisesRegex(ValueError, 'raw member'):
            reader.verify(self.root)

    def test_index_cannot_self_refresh(self):
        self.rewrite('phase-ledger.json', lambda raw: raw+b' ')
        with self.assertRaisesRegex(ValueError, 'fixed provenance index'):
            reader.verify(self.root)

    def test_ledger_pin_is_independent_of_rehashed_index(self):
        digest = self.rewrite('phase-ledger.json', lambda raw: raw.replace(b'544', b'545', 1))
        with patch.object(reader, 'INDEX_SHA256', digest):
            with self.assertRaisesRegex(ValueError, 'fixed phase ledger'):
                reader.verify(self.root)

    def test_pinned_source_cannot_be_replaced_by_a_worktree_claim(self):
        name = 'source/' + reader.NEW + '/tools/long-run-supervisor-074/src/diagnostic_kernel_fd_identity.rs'
        digest = self.rewrite(name, lambda raw: raw+b'\n// unknown edit\n')
        with patch.object(reader, 'INDEX_SHA256', digest):
            with self.assertRaisesRegex(ValueError, 'current source identity'):
                reader.verify(self.root)

    def test_stale_index_cannot_be_substituted(self):
        digest = self.rewrite('observed-index.raw', lambda raw: raw.split(b'\n', 1)[1])
        with patch.object(reader, 'INDEX_SHA256', digest):
            with self.assertRaisesRegex(ValueError, 'observed index'):
                reader.verify(self.root)


if __name__ == '__main__':
    unittest.main()
