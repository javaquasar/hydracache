"""Inert refusal, deterministic archive and original CI-contract controls."""
import copy
import io
import json
from pathlib import Path
import tarfile
import tempfile
import tomllib
import unittest

import run
import native


class Controls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.recipe, cls.packages, cls.final = run.reviewed_packages()
        original, _ = run.predecessors()
        cls.archives = {name: original('capture/primary/' + name + '-' + row['version'] + '.crate')
                        for name, row in cls.recipe['bases'].items()}
        cls.objects = {r['after_sha256']: (run.ROOT / 'tools/dbus-source-package-074/patches/objects' /
                       r['after_sha256']).read_bytes() for r in cls.recipe['changes']}

    def rebuild(self, recipe=None, archives=None, objects=None):
        return run.codec.rebuild(recipe or self.recipe, archives or self.archives,
                                 self.objects if objects is None else objects)

    def test_whole_base_and_final_inventory_and_two_rebuilds(self):
        self.assertEqual(sum(len(r['members']) for r in self.recipe['bases'].values()), 263)
        self.assertEqual(sum(map(len, self.final.values())), 266)
        self.assertEqual(len(self.recipe['changes']), 11)
        self.assertEqual(self.rebuild(), self.rebuild())

    def test_canonical_order_clock_and_metadata_have_no_host_inputs(self):
        for name, raw in self.packages.items():
            prefix = name + '-' + self.recipe['bases'][name]['version']
            members = self.final[name]
            self.assertEqual(run.codec.pack(dict(reversed(list(members.items()))), prefix), raw)
            self.assertEqual(run.codec.unpack(raw, prefix, True), members)

    def test_export_needs_no_prior_builds_and_never_overwrites(self):
        with tempfile.TemporaryDirectory(prefix='hc074-source-export-') as temporary:
            out = Path(temporary) / 'source'
            self.assertEqual(run.export(out), self.recipe['source_archive_sha256'])
            receipt = json.loads((out / 'receipt.json').read_bytes())
            self.assertEqual(receipt['recipe_sha256'], run.RECIPE_SHA)
            self.assertFalse(receipt['code_executed'])
            self.assertFalse(receipt['registry_qualified'])
            for crate, raw in self.packages.items():
                name = crate + '-' + self.recipe['bases'][crate]['version'] + '-hc074-source.tar'
                self.assertEqual((out / name).read_bytes(), raw)
            before = run.files(out)
            with self.assertRaises(FileExistsError):
                run.export(out)
            self.assertEqual(run.files(out), before)

    def test_unsafe_and_aliased_paths_refuse(self):
        for name in ('', '/abs', '../x', 'x/../y', 'a//b', './x', 'a\\b', 'C:/x', 'a\x00b', 'a\nb'):
            with self.subTest(name=name), self.assertRaises(ValueError):
                run.codec.safe_name(name)

    @staticmethod
    def tar(entries):
        out = io.BytesIO()
        with tarfile.open(fileobj=out, mode='w') as archive:
            for name, kind in entries:
                info = tarfile.TarInfo(name)
                info.type = kind
                info.size = 1 if kind == tarfile.REGTYPE else 0
                info.linkname = 'foreign'
                archive.addfile(info, io.BytesIO(b'x') if kind == tarfile.REGTYPE else None)
        return out.getvalue()

    def test_links_duplicates_and_foreign_prefix_refuse_without_extraction(self):
        for entries in ([('pkg/a', tarfile.SYMTYPE)], [('pkg/a', tarfile.LNKTYPE)],
                        [('pkg/a', tarfile.REGTYPE), ('pkg/a', tarfile.REGTYPE)],
                        [('other/a', tarfile.REGTYPE)], [('pkg/../a', tarfile.REGTYPE)]):
            with self.subTest(entries=entries), self.assertRaises(ValueError):
                run.codec.unpack(self.tar(entries), 'pkg')

    def test_canonical_metadata_tampering_refuses(self):
        raw = self.tar([('pkg/a', tarfile.REGTYPE)])
        self.assertEqual(run.codec.unpack(raw, 'pkg'), {'a': b'x'})
        with self.assertRaises(ValueError):
            run.codec.unpack(raw, 'pkg', True)

    def test_wrong_base_and_missing_or_changed_replacement_refuse(self):
        archives = dict(self.archives)
        archives['zbus'] += b'\n'
        with self.assertRaises(ValueError):
            self.rebuild(archives=archives)
        for objects in ({}, dict(self.objects, foreign=b'x')):
            with self.assertRaises(ValueError):
                self.rebuild(objects=objects)
        changed = dict(self.objects)
        changed[next(iter(changed))] += b'\n'
        with self.assertRaises(ValueError):
            self.rebuild(objects=changed)

    def test_stale_preimage_final_map_and_archive_pins_refuse(self):
        for field in ('preimage', 'inventory', 'archive'):
            recipe = copy.deepcopy(self.recipe)
            if field == 'preimage':
                recipe['changes'][0]['before_sha256'] = '0' * 64
            elif field == 'inventory':
                recipe['final_members']['zbus']['Cargo.toml'] = '0' * 64
            else:
                recipe['source_archive_sha256']['zbus'] = '0' * 64
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.rebuild(recipe=recipe)

    def test_duplicate_and_foreign_change_owners_refuse(self):
        for foreign in (False, True):
            recipe = copy.deepcopy(self.recipe)
            if foreign:
                recipe['changes'][0]['crate'] = 'foreign'
            else:
                recipe['changes'].append(dict(recipe['changes'][0]))
            with self.assertRaises(ValueError):
                self.rebuild(recipe=recipe)

    def test_compiler_bodies_manifests_and_license_match_previous_consumer(self):
        source = run.ROOT / 'target/dbus-reply-state-20261028/alias-fixed/input/fd'
        for crate, members in self.final.items():
            for name, raw in members.items():
                if name.endswith('.rs') or name in ('Cargo.toml', 'LICENSE', 'Cargo.lock'):
                    self.assertEqual((source / crate / name).read_bytes(), raw)
            self.assertEqual(tomllib.loads(members['Cargo.toml'].decode()),
                             tomllib.loads(members['Cargo.toml.orig'].decode()))
            self.assertEqual(json.loads(members['BACKPORT074.json'])['release_admitted'], False)
        self.assertFalse(self.recipe['published'])
        self.assertFalse(self.recipe['registry_qualified'])

    def test_readable_diffs_and_replacement_objects_are_pinned(self):
        for name, digest in self.recipe['review_diff_sha256'].items():
            self.assertEqual(run.sha((run.ROOT / 'tools/dbus-source-package-074/patches' /
                                      (name + '.patch')).read_bytes()), digest)

    def test_native_worker_and_original_limits_and_toolchain_context(self):
        owner = native.configured(run.ROOT)
        self.assertEqual(owner.BASE, run.BASE)
        self.assertEqual(owner.NATIVE.as_posix(), run.NATIVE)
        self.assertEqual(owner.prepared(), run.BASE / 'prepare/result.json')
        self.assertEqual(owner.session.__globals__['__file__'], str(Path(native.__file__)))
        argv, env, _, _, folder, _ = owner.command('fd-188-clippy', {})
        self.assertIn('/1.88.0-', argv[0].replace('\\', '/'))
        self.assertIn('--locked', argv)
        self.assertIn('--offline', argv)
        self.assertIn('-D', argv)
        self.assertEqual(folder, owner.NATIVE / 'fd')
        self.assertEqual(env['CARGO_INCREMENTAL'], '0')
        for token in ('deadline=time.monotonic()+1100', 'bound=1200', '4*1024**3',
                      'kernel-owned phase retirement'):
            self.assertIn(token, (run.ROOT / 'tools/dbus-linux-074/native.py').read_text(encoding='utf8'))

    def test_prepared_source_archives_catalog_and_root_guards(self):
        review = json.loads((run.BASE / 'prepare/result.json').read_bytes())
        self.assertEqual(run.files(run.BASE / 'input'), review['files'])
        self.assertEqual(run.parent.fragment.prior.parent.old.root_inputs(), review['root_inputs'])
        self.assertEqual(len(set(review['baseline'] + review['extras'])), 67)
        for crate, raw in self.packages.items():
            name = crate + '-' + self.recipe['bases'][crate]['version'] + '-hc074-source.tar'
            self.assertEqual((run.BASE / 'input/packages' / name).read_bytes(), raw)
        with self.assertRaises(ValueError):
            run.native_command(3)


if __name__ == '__main__':
    unittest.main()
