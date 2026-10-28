"""Inert root publication closure, exact manifest and source-provenance controls."""
from pathlib import Path
import copy
import importlib.util
import json
import tomllib
import unittest

spec = importlib.util.spec_from_file_location('root_adoption_pure_review', Path(__file__).with_name('review.py'))
r = importlib.util.module_from_spec(spec)
spec.loader.exec_module(r)


class Review(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = tomllib.loads(r.checked('Cargo.toml', r.PROTECTED['Cargo.toml']).decode())
        cls.manifests = {p: tomllib.loads((r.ROOT / p / 'Cargo.toml').read_text())
                         for p in cls.root['workspace']['members']}

    def test_complete_conservative_publication_closure_includes_all_targets_and_kinds(self):
        scope = r.workspace_scope(self.root, self.manifests)
        self.assertEqual(len(scope['members']), 36)
        self.assertEqual(len(scope['publishable']), 23)
        self.assertNotIn(r.PKG, scope['reachable_from_publishable'])
        self.assertTrue(scope['only_unpublished_tool_uses_zbus'])
        changed = copy.deepcopy(self.manifests)
        published = scope['publishable'][0]
        changed[published].setdefault('target', {})['cfg(target_os = "linux")'] = {
            'build-dependencies': {'hidden-tool': {'package': 'hydracache-long-run-supervisor-074',
                'path': str((r.ROOT / r.PKG).relative_to(r.ROOT))}}}
        # A full absolute local path is legal Cargo input and must also be traversed.
        changed[published]['target']['cfg(target_os = "linux")']['build-dependencies']['hidden-tool']['path'] = str(r.ROOT / r.PKG)
        with self.assertRaises(ValueError):
            r.workspace_scope(self.root, changed)

    def test_missing_members_external_paths_or_inherited_override_cannot_hide_edges(self):
        changed = copy.deepcopy(self.manifests)
        changed.pop(next(iter(changed)))
        with self.assertRaises(ValueError):
            r.workspace_scope(self.root, changed)
        for dep in ({'path': '../../../outside'}, {'workspace': True, 'path': '../hidden'}):
            changed = copy.deepcopy(self.manifests)
            changed[r.PKG].setdefault('build-dependencies', {})['hydracache'] = dep
            with self.assertRaises(ValueError):
                r.workspace_scope(self.root, changed)

    def test_root_manifest_keeps_all_product_members_floor_and_existing_dependencies(self):
        raw = r.checked('Cargo.toml', r.PROTECTED['Cargo.toml'])
        after = tomllib.loads(r.root_manifest(raw).decode())
        self.assertEqual(after['workspace']['members'], self.root['workspace']['members'])
        self.assertEqual(after['workspace']['package'], self.root['workspace']['package'])
        self.assertEqual(after['workspace']['dependencies'], self.root['workspace']['dependencies'])
        self.assertEqual(after['workspace']['exclude'], [r.VENDOR + '/' + n for n in ('zbus', 'zbus_macros', 'zbus_utils')])
        self.assertEqual(set(after['patch']['crates-io']), {'zbus_macros', 'zbus_utils'})
        with self.assertRaises(ValueError):
            r.root_manifest(raw + b'\n[patch.crates-io]\n')

    def test_direct_manifest_preserves_unrelated_linux_unix_windows_dependencies(self):
        raw = r.checked(r.PKG + '/Cargo.toml', r.PROTECTED[r.PKG + '/Cargo.toml'])
        before, after = tomllib.loads(raw.decode()), tomllib.loads(r.supervisor_manifest(raw).decode())
        self.assertFalse(after['package']['publish'])
        for target in ('cfg(unix)', 'cfg(windows)'):
            self.assertEqual(before['target'][target], after['target'][target])
        for dep in ('base64', 'tokio', 'ring'):
            self.assertEqual(before['target']['cfg(target_os = "linux")']['dependencies'][dep],
                             after['target']['cfg(target_os = "linux")']['dependencies'][dep])
        self.assertNotIn('zbus', after['target']['cfg(target_os = "linux")']['dependencies'])
        self.assertEqual(after['target']['cfg(target_os = "linux")']['dev-dependencies']['zbus6']['features'], ['hc074-unix-fixture'])
        with self.assertRaises(ValueError):
            r.supervisor_manifest(raw.replace(b'5.19.0', b'5.18.0'))

    def test_exact_source_recipe_and_all_compiled_archive_members(self):
        package = r.source_package()
        recipe, _, final = package.reviewed()
        freeze = json.loads(r.held('consumer2/freeze/result.json'))
        self.assertEqual(sum(map(len, final.values())), 266)
        self.assertEqual(recipe['source_archive_sha256'], freeze['source_archive_sha256'])
        for crate, members in final.items():
            for name, raw in members.items():
                self.assertEqual(r.sha(raw), freeze['files']['fd/source/' + crate + '/' + name])


if __name__ == '__main__':
    unittest.main()
