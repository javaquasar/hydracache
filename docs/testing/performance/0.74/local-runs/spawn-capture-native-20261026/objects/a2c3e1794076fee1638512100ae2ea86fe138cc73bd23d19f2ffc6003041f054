import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import tree_locality as locality
import sweep_review as sweep


def encode(entries):
    return b''.join(f'100644 blob {oid}\t{path}'.encode() + b'\0' for path, oid in entries)


def fixture():
    trees = []
    for _, _, count, tail, _ in sweep.SWEEPS:
        names = [locality.SCOPE + f'/{index}.rs' for index in range(count)] + list(tail)
        trees.append(encode((name, hashlib.sha1(name.encode()).hexdigest()) for name in names))
    return trees


class LocalityControls(unittest.TestCase):
    def test_capture_binds_commands_outputs_completion_and_original_source(self):
        trees = fixture()
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            intent = dict(source_commit=sweep.COMMIT, source_sha256=sweep.SOURCE_SHA256,
                          revisions=list(locality.REVISIONS), retry=False)
            records = []
            for index, raw in enumerate(trees):
                (folder / f'{index}-stdout.bin').write_bytes(raw)
                (folder / f'{index}-stderr.bin').write_bytes(b'')
                records.append(dict(index=index, exit=0, argv=locality.command(index),
                                    stdout_sha256=sweep.review.sha(raw), stderr_sha256=sweep.review.sha(b'')))
            result = dict(complete=True, release_admission=False, records=records)
            def save():
                (folder / 'intent.json').write_text(json.dumps(intent))
                (folder / 'result.json').write_text(json.dumps(result))
            save()
            self.assertEqual(locality.load_capture(folder), locality.analyze_trees(trees))
            for owner, key, bad in [(intent, 'source_commit', '0' * 40), (intent, 'retry', True),
                                    (result, 'complete', False), (result, 'release_admission', True),
                                    (records[0], 'index', False), (records[0], 'exit', False),
                                    (records[0], 'argv', ['git', 'show']),
                                    (records[0], 'stdout_sha256', '0' * 64)]:
                original = owner[key]
                owner[key] = bad
                save()
                with self.subTest(key=key), self.assertRaises(ValueError):
                    locality.load_capture(folder)
                owner[key] = original
            save()
            (folder / '0-stderr.bin').write_bytes(b'first refusal')
            with self.assertRaises(ValueError):
                locality.load_capture(folder)

    def test_distinct_keys_can_share_objects_but_have_no_exact_key_hits(self):
        result = locality.analyze_trees(fixture())
        self.assertEqual(result['logical_revision_path_requests'], 3116)
        self.assertEqual(result['unique_blob_oids'], 451)
        self.assertEqual(result['references_beyond_first_per_blob'], 2665)
        self.assertEqual(result['exact_key_cache_hits_within_selected_sweeps'], 0)
        self.assertFalse(result['speedup_measured'])
        self.assertFalse(result['original_replacement_refs_or_config_attested'])

    def test_missing_duplicate_aliased_foreign_mode_oid_and_tail_refuse(self):
        good = fixture()[0]
        first, rest = good.split(b'\0', 1)
        for raw in (b'', good[:-1], good + first + b'\0', rest,
                    good.replace(b'100644', b'120000', 1), good.replace(b'blob', b'tree', 1),
                    good.replace(b'/0.rs', b'/../0.rs', 1), good.replace(b'/0.rs', b'\\0.rs', 1),
                    good.replace(locality.SCOPE.encode(), b'foreign', 1),
                    good.replace(b'Cargo.lock', b'Cargo.other', 1),
                    good.replace(first.split(b' ')[2][:1], b'G', 1)):
            with self.subTest(raw=raw[:30]), self.assertRaises(ValueError):
                locality.parse_tree(raw, 446, sweep.TAIL)

    def test_protocol_cannot_drop_a_sweep_or_expand_source_scope(self):
        trees = fixture()
        with self.assertRaises(ValueError):
            locality.analyze_trees(trees[:-1])
        trees[0] += encode([('foreign', 'a' * 40)])
        with self.assertRaises(ValueError):
            locality.analyze_trees(trees)

    def test_command_uses_only_pinned_local_object_reads(self):
        for index in range(7):
            command = locality.command(index)
            self.assertIn('--no-replace-objects', command)
            self.assertIn('-z', command)
            self.assertEqual(command[8], locality.REVISIONS[index])
            self.assertEqual(command[9:11], ['--', locality.SCOPE])


if __name__ == '__main__':
    unittest.main()
