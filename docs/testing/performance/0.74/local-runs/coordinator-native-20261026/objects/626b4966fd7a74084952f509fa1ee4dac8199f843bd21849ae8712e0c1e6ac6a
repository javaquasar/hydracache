from pathlib import Path
import tempfile
import unittest

import syntax_audit


class ModuleClosureControls(unittest.TestCase):
    def test_default_module_and_literal_paths_close(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            (root / 'mod.rs').write_text('mod child;')
            (root / 'child.rs').write_text('')
            (root / 'other.rs').write_text('')
            document = dict(sources=[dict(source='mod.rs', sites=[
                dict(kind='module', scope='child', detail='default-module-path'),
                dict(kind='module', scope='renamed', detail='literal:other.rs')])])
            additions, unresolved = syntax_audit.module_edges(root, document)
            self.assertEqual(set(additions), {'child.rs', 'other.rs'})
            self.assertEqual(unresolved, [])
            document['sources'].extend(dict(source=name, sites=[]) for name in additions)
            self.assertEqual(syntax_audit.module_edges(root, document), ({}, []))

    def test_ambiguous_or_missing_module_does_not_clear(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            (root / 'child.rs').write_text('')
            (root / 'child').mkdir()
            (root / 'child/mod.rs').write_text('')
            document = dict(sources=[dict(source='mod.rs', sites=[
                dict(kind='module', scope='child', detail='default-module-path'),
                dict(kind='module', scope='missing', detail='default-module-path')])])
            additions, unresolved = syntax_audit.module_edges(root, document)
            self.assertEqual(additions, {})
            self.assertEqual(len(unresolved), 2)

    def test_outside_source_edge_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            outer = Path(directory).resolve()
            root = outer / 'root'
            root.mkdir()
            (outer / 'outside.rs').write_text('')
            document = dict(sources=[dict(source='mod.rs', sites=[
                dict(kind='module', scope='bad', detail='literal:../outside.rs')])])
            with self.assertRaises(AssertionError):
                syntax_audit.module_edges(root, document)


if __name__ == '__main__':
    unittest.main()
