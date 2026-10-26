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
        body = mutate((self.root / 'objects' / row['sha256']).read_bytes())
        digest = reader.sha(body)
        (self.root / 'objects' / digest).write_bytes(body)
        index['members'][name] = dict(sha256=digest, bytes=len(body))
        raw = (json.dumps(index, indent=2, sort_keys=True) + '\n').encode()
        (self.root / 'index.json').write_bytes(raw)
        return reader.sha(raw)

    def test_exact_packet_replays_without_executing_retained_code(self):
        self.assertEqual(reader.verify(self.root)['current_and_msrv_distinct_cases_each'], 33)

    def test_raw_corruption_refuses(self):
        index = json.loads((self.root / 'index.json').read_bytes())
        row = next(iter(index['members'].values()))
        path = self.root / 'objects' / row['sha256']
        path.write_bytes(path.read_bytes() + b'x')
        with self.assertRaisesRegex(ValueError, 'raw member'):
            reader.verify(self.root)

    def test_index_cannot_self_refresh(self):
        self.rewrite('source-final/tools/authority-reader-phase-074/main-rules.json', lambda raw: raw + b' ')
        with self.assertRaisesRegex(ValueError, 'fixed index'):
            reader.verify(self.root)

    def test_rehashed_index_cannot_erase_first_refusal(self):
        digest = self.rewrite('checks/authority-continuation-first-targets/stdout.log', lambda _: b'passed')
        with patch.object(reader, 'INDEX_SHA256', digest):
            with self.assertRaisesRegex(ValueError, 'cohort accounting'):
                reader.verify(self.root)

    def test_old_reader_pin_is_independent_of_evidence_index(self):
        digest = self.rewrite('baseline/crates/xtask/tests/owned_case_074.rs', lambda raw: raw + b'\n// drift')
        with patch.object(reader, 'INDEX_SHA256', digest):
            with self.assertRaisesRegex(ValueError, 'independent old reader'):
                reader.verify(self.root)

    def test_fixed_main_producer_cannot_be_substituted(self):
        name = 'producer/a67a5299ab335424e9b11330dc06bd749ea9b859/crates/xtask/tests/performance_contract_074.rs'
        digest = self.rewrite(name, lambda raw: raw + b'\n// drift')
        with patch.object(reader, 'INDEX_SHA256', digest):
            with self.assertRaisesRegex(ValueError, 'producer differs'):
                reader.verify(self.root)


if __name__ == '__main__':
    unittest.main()
