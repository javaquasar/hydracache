"""Held archive reconstruction and actual paired-accounting verification."""
from pathlib import Path
import importlib.util
import json
import unittest

spec = importlib.util.spec_from_file_location('native_package_retained_bytes', Path(__file__).with_name('source_package.py'))
p = importlib.util.module_from_spec(spec)
spec.loader.exec_module(p)
OUT = p.ROOT / 'docs/testing/performance/0.74/local-runs/dbus-native-source-2-20261028'
INDEX = '2babe9897ec6bd30ecfcb264af372de14129bde23baab8b5b324cec0fd1cf952'


class RetainedPackage(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        raw = (OUT / 'index.json').read_bytes()
        if p.sha(raw) != INDEX:
            raise ValueError('whole held archive-consumer index')
        cls.index = json.loads(raw)
        cls.members = cls.index['files']

    def member(self, name):
        row = self.members[name]
        raw = (OUT / 'objects' / row['sha256']).read_bytes()
        self.assertEqual(p.sha(raw), row['sha256'])
        self.assertEqual(len(raw), row['bytes'])
        return raw

    def test_all_held_bytes_and_first_inert_failure_are_retained(self):
        self.assertEqual(len(self.members), 165)
        for name in self.members:
            self.member(name)
        refused = json.loads(self.member('consumer/first-inert/refusal.json'))
        self.assertFalse(refused['passed'] or refused['native_executed'])
        self.assertEqual(refused['kind'], 'observed-first-control-summary-not-raw-log')
        self.assertFalse(self.index['release_admitted'] or self.index['registry_qualified']
                         or self.index['root_changed'] or self.index['performance_improvement_proven'])

    def test_reconstruct_from_held_recipe_objects_and_independently_pinned_primary_bytes(self):
        recipe = json.loads(self.member('patches/recipe.json'))
        self.assertEqual(p.sha(self.member('patches/recipe.json')), p.RECIPE_SHA)
        objects = {r['after_sha256']: self.member('patches/objects/' + r['after_sha256'])
                   for r in recipe['changes']}
        packages, final = p.codec.rebuild(recipe, p.archives(recipe), objects)
        self.assertEqual({c: p.sha(raw) for c, raw in packages.items()}, self.index['source_archive_sha256'])
        freeze = json.loads(self.member('consumer2/freeze/result.json'))
        self.assertEqual(sum(map(len, final.values())), 266)
        for crate, members in final.items():
            for name, raw in members.items():
                self.assertEqual(p.sha(raw), freeze['files']['fd/source/' + crate + '/' + name])

    def test_actual_compiler_catalogs_passes_filters_and_retirement(self):
        catalogs = []
        for compiler in ('194', '188'):
            prefix = 'consumer2/units/fd-' + compiler
            listing = json.loads(self.member(prefix + '-list/result.json'))
            catalogs.append(listing['inventory'])
            tested = json.loads(self.member(prefix + '-test/result.json'))
            self.assertEqual(tested['summary'], [['ok', '65', '0', '0', '0', '1528']])
            self.assertEqual(len(set(tested['selected'])), 65)
            self.assertEqual(sorted(n for n, state in tested['rows'] if state == 'ok'), tested['selected'])
            for action in ('list', 'test', 'clippy'):
                execution = json.loads(self.member(prefix + '-' + action + '/execution.json'))
                self.assertTrue(execution['retired'])
                self.assertFalse(execution['errors'])
                self.assertEqual(execution['exit'], 0)
        self.assertEqual(catalogs[0], catalogs[1])
        self.assertEqual(len(set(catalogs[0])), 1593)


if __name__ == '__main__':
    unittest.main()
