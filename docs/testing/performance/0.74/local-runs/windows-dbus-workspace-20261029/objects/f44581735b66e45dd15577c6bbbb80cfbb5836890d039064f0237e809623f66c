"""Pure temporary-file controls; neither a Rust program nor a fixture is run."""
from pathlib import Path
import subprocess
import tempfile
import unittest

import module_context_v5 as candidate
import syntax_audit as previous


def module(scope, detail='default-module-path'):
    return dict(kind='module', scope=scope, detail=detail)


class ContextControls(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()

    def file(self, name):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(b'')
        return path

    def document(self, entries):
        for name in entries:
            self.file(name)
        return dict(sources=[dict(source=name, sites=sites) for name, sites in entries.items()])

    def test_literal_import_uses_parent_for_default_children(self):
        doc = self.document({
            'crates/xtask/tests/case.rs': [module('renamed', 'literal:../../../tools/reader.rs')],
            'tools/reader.rs': [module('child')],
            'tools/child.rs': [],
        })
        self.assertEqual(len(previous.module_edges(self.root, doc)[1]), 1)
        self.assertEqual(candidate.module_edges(self.root, doc), ({}, []))

    def test_ordinary_and_mod_rs_children_keep_their_directories(self):
        doc = self.document({
            'crates/xtask/src/lib.rs': [module('parent')],
            'crates/xtask/src/parent.rs': [module('nested')],
            'crates/xtask/src/parent/nested/mod.rs': [module('leaf')],
        })
        self.file('crates/xtask/src/parent/nested/leaf.rs')
        additions, unresolved = candidate.module_edges(self.root, doc)
        self.assertEqual(set(additions), {'crates/xtask/src/parent/nested/leaf.rs'})
        self.assertEqual(unresolved, [])

    def test_ambiguous_or_missing_files_do_not_fallback(self):
        doc = self.document({'crates/xtask/src/lib.rs': [module('child'), module('missing')]})
        self.file('crates/xtask/src/child.rs')
        self.file('crates/xtask/src/child/mod.rs')
        self.assertEqual(candidate.module_edges(self.root, doc)[0], {})
        self.assertEqual(len(candidate.module_edges(self.root, doc)[1]), 2)

    def test_conflicting_declaration_contexts_refuse(self):
        doc = self.document({
            'crates/xtask/src/lib.rs': [module('reader'), module('alias', 'literal:reader.rs')],
            'crates/xtask/src/reader.rs': [module('child')],
            'crates/xtask/src/child.rs': [],
            'crates/xtask/src/reader/child.rs': [],
        })
        self.assertEqual(len(candidate.module_edges(self.root, doc)[1]), 1)

    def test_unowned_source_does_not_infer_a_context_from_existing_files(self):
        doc = self.document({'tools/reader.rs': [module('child')], 'tools/reader/child.rs': []})
        self.assertEqual(len(candidate.module_edges(self.root, doc)[1]), 1)

    def test_unknown_duplicate_and_outside_inputs_refuse(self):
        doc = self.document({'crates/xtask/src/lib.rs': [module('bad', 'computed')]})
        with self.assertRaisesRegex(ValueError, 'unsupported module declaration'):
            candidate.module_edges(self.root, doc)
        doc['sources'][0]['sites'] = [module('bad', 'literal:../../../../outside.rs')]
        with self.assertRaisesRegex(ValueError, 'outside'):
            candidate.module_edges(self.root, doc)
        doc['sources'].append(doc['sources'][0])
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            candidate.module_edges(self.root, doc)

    def test_nested_inline_scope_and_reordered_rows(self):
        doc = self.document({
            'crates/xtask/tests/case.rs': [module('outer::renamed', 'literal:../../../../tools/reader.rs')],
            'tools/reader.rs': [module('child')],
            'tools/child.rs': [],
        })
        # Match Rust's path traversal, whose intermediate directory must exist.
        (self.root / 'crates/xtask/tests/outer').mkdir()
        self.assertEqual(candidate.module_edges(self.root, doc), ({}, []))
        doc['sources'].reverse()
        self.assertEqual(candidate.module_edges(self.root, doc), ({}, []))

    def test_rustc_metadata_confirms_literal_and_ordinary_contexts_without_execution(self):
        for literal in (True, False):
            folder = self.root / ('literal' if literal else 'ordinary')
            folder.mkdir()
            (folder / 'reader').mkdir()
            (folder / 'lib.rs').write_text(
                ('#[path="reader.rs"] mod renamed;' if literal else 'mod reader;') +
                'pub fn value() -> u8 {' + ('renamed' if literal else 'reader') + '::value()}',
                encoding='utf-8')
            (folder / 'reader.rs').write_text('mod child; pub fn value() -> u8 {child::value()}', encoding='utf-8')
            for path, good in ((folder / 'child.rs', literal), (folder / 'reader/child.rs', not literal)):
                path.write_text('pub fn value() -> ' + ('u8' if good else 'u16') + ' {1}', encoding='utf-8')
            result = subprocess.run(['rustc', '+1.94.0', '--crate-type=lib', '--emit=metadata',
                                     str(folder / 'lib.rs'), '-o', str(folder / 'fixture.rmeta')],
                                    capture_output=True, timeout=30, check=False)
            self.assertEqual(result.returncode, 0, result.stderr.decode(errors='replace'))
            self.assertTrue((folder / 'fixture.rmeta').is_file())


if __name__ == '__main__':
    unittest.main()
