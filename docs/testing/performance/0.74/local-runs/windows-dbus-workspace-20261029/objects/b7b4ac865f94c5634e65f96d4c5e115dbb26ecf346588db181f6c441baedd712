"""Pure source inventory, canonical archive, stale-input and export controls."""
from pathlib import Path
import copy
import importlib.util
import json
import tempfile
import tomllib
import unittest

spec = importlib.util.spec_from_file_location('native_source_package_review', Path(__file__).with_name('source_package.py'))
p = importlib.util.module_from_spec(spec)
spec.loader.exec_module(p)


class Package(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.recipe, cls.packages, cls.final = p.reviewed()
        cls.archives = p.archives(cls.recipe)
        cls.objects = {r['after_sha256']: (p.PATCHES / 'objects' / r['after_sha256']).read_bytes()
                       for r in cls.recipe['changes']}

    def test_deterministic_archives_match_actual_tested_source_inventory(self):
        self.assertEqual(len(self.recipe['changes']), 12)
        self.assertEqual(sum(map(len, self.final.values())), 266)
        freeze = json.loads(p.member('dbus-native-transport-20261028', p.NATIVE, 'f2/freeze/result.json'))
        for crate, members in self.final.items():
            for name, body in members.items():
                self.assertEqual(p.sha(body), freeze['files']['fd/source/' + crate + '/' + name])
            prefix = crate + '-' + self.recipe['bases'][crate]['version']
            self.assertEqual(p.codec.pack(dict(reversed(list(members.items()))), prefix), self.packages[crate])
            self.assertEqual(p.codec.unpack(self.packages[crate], prefix, True), members)

    def test_nondefault_factory_manifest_and_explicit_modified_provenance(self):
        package = self.final['zbus']
        self.assertEqual(package['Cargo.toml'], package['Cargo.toml.orig'])
        manifest = tomllib.loads(package['Cargo.toml'].decode())
        self.assertEqual(manifest['features']['hc074-unix-fixture'], [])
        self.assertNotIn('hc074-unix-fixture', manifest['features']['default'])
        self.assertEqual(manifest['package']['rust-version'], '1.88')
        provenance = json.loads(package['BACKPORT074.json'])
        self.assertEqual(provenance['source_revision'], 'hc074-zbus-native-fixture-source-2')
        self.assertEqual(provenance['compiler_manifest_sha256'], p.sha(package['Cargo.toml']))
        self.assertFalse(self.recipe['root_adopted'] or self.recipe['published'] or self.recipe['registry_qualified'])

    def test_changed_preimage_object_inventory_or_archive_refuses(self):
        recipe = copy.deepcopy(self.recipe)
        recipe['changes'][0]['before_sha256'] = '0' * 64
        with self.assertRaises(ValueError):
            p.codec.rebuild(recipe, self.archives, self.objects)
        objects = dict(self.objects)
        key = next(iter(objects))
        objects[key] += b'\n'
        with self.assertRaises(ValueError):
            p.codec.rebuild(self.recipe, self.archives, objects)
        recipe = copy.deepcopy(self.recipe)
        recipe['final_members']['zbus'].pop('Cargo.toml')
        with self.assertRaises(ValueError):
            p.codec.rebuild(recipe, self.archives, self.objects)
        archives = dict(self.archives)
        archives['zbus'] += b'\x00'
        with self.assertRaises(ValueError):
            p.codec.rebuild(self.recipe, archives, self.objects)

    def test_retained_only_export_is_exact_unsigned_and_never_overwritten(self):
        with tempfile.TemporaryDirectory(dir=p.ROOT / 'target') as name:
            out = Path(name) / 'export'
            receipt = p.export(out)
            self.assertFalse(receipt['code_executed'] or receipt['registry_qualified'] or receipt['root_adopted'])
            self.assertTrue(receipt['unsigned'])
            before = {a.name: a.read_bytes() for a in out.iterdir()}
            with self.assertRaises(FileExistsError):
                p.export(out)
            self.assertEqual(before, {a.name: a.read_bytes() for a in out.iterdir()})


if __name__ == '__main__':
    unittest.main()
