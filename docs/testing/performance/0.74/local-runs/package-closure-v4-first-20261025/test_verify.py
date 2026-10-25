"""Mutation controls operate on copied data, never run package code."""
import copy
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
        self.folder = Path(self.directory.name) / 'packet'
        self.folder.mkdir()
        shutil.copy2(PACKET / 'index.json', self.folder / 'index.json')
        shutil.copytree(PACKET / 'objects', self.folder / 'objects')

    def rewrite(self, member, mutate):
        path = self.folder / 'index.json'
        index = json.loads(path.read_bytes())
        value = json.loads((self.folder / 'objects' / index['members'][member]['sha256']).read_bytes())
        value = copy.deepcopy(value)
        mutate(value)
        body = (json.dumps(value, indent=2, sort_keys=True) + '\n').encode()
        digest = reader.sha(body)
        (self.folder / 'objects' / digest).write_bytes(body)
        index['members'][member] = dict(sha256=digest, bytes=len(body))
        raw = (json.dumps(index, indent=2, sort_keys=True) + '\n').encode()
        path.write_bytes(raw)
        return reader.sha(raw)

    def test_original_incomplete_result_replays(self):
        result = reader.verify(self.folder)
        self.assertEqual((result['completed_targets'], result['expected_targets']), (114, 138))
        self.assertEqual((result['passed_in_completed'], result['failed_in_completed']), (777, 46))
        self.assertEqual((result['observed_passed'], result['observed_failed']), (881, 76))
        self.assertEqual((result['incomplete_observations'], result['unstarted_targets']), (134, 23))
        self.assertFalse(result['package_pass'])

    def test_raw_mutation_refuses(self):
        index = json.loads((self.folder / 'index.json').read_bytes())
        path = self.folder / 'objects' / index['members']['package/stdout.log']['sha256']
        path.write_bytes(path.read_bytes() + b'altered')
        with self.assertRaisesRegex(ValueError, 'raw member'):
            reader.verify(self.folder)

    def test_first_index_cannot_self_refresh(self):
        self.rewrite('analysis-first.json', lambda value: value.update(package_pass=True))
        with self.assertRaisesRegex(ValueError, 'fixed first index'):
            reader.verify(self.folder)

    def test_forged_analysis_refuses_even_with_rehashed_objects(self):
        digest = self.rewrite('analysis-first.json', lambda value: value.update(package_pass=True))
        with patch.object(reader, 'INDEX_SHA256', digest):
            with self.assertRaisesRegex(ValueError, 'derived result'):
                reader.verify(self.folder)

    def test_changed_source_cannot_be_hidden_by_boolean(self):
        digest = self.rewrite('package/result.json', lambda value: value['source_after'].pop('Cargo.lock'))
        with patch.object(reader, 'INDEX_SHA256', digest):
            with self.assertRaisesRegex(ValueError, 'source/HEAD changed'):
                reader.verify(self.folder)

    def test_shortened_catalog_refuses(self):
        digest = self.rewrite('preflight/gate.json', lambda value: value['targets'].pop())
        with patch.object(reader, 'INDEX_SHA256', digest):
            with self.assertRaisesRegex(ValueError, 'full catalog'):
                reader.verify(self.folder)

    def test_timeout_cannot_be_rewritten_as_successful_natural_exit(self):
        digest = self.rewrite('package/result.json', lambda value: value.update(exit=0, outer_refusal=None))
        with patch.object(reader, 'INDEX_SHA256', digest):
            with self.assertRaisesRegex(ValueError, 'natural exit with incomplete'):
                reader.verify(self.folder)


if __name__ == '__main__':
    unittest.main()
