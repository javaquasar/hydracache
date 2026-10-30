"""Pure source/identity guards; no Cargo, registry or archived code execution."""
import copy,json,tomllib,unittest,contextlib,io,tempfile
from pathlib import Path
import model as m

class Controls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):cls.upstream,cls.u,cls.lock=m.parent();cls.final=m.project(cls.u,cls.lock)
    def test_only_reversible_style_then_exact_U(self):
        expected={n:b for n,b in self.u.items() if n.endswith('.rs')}
        self.assertEqual({n:b for n,b in m.owned_style.change(self.final,True).items() if n.endswith('.rs')},expected)
        self.assertEqual(m.verify_source(self.final,self.u)['algorithm_changes'],0)
    def test_exactly_one_resolver_at_standalone_workspace(self):
        doc=tomllib.loads(self.final['Cargo.toml'].decode())
        self.assertNotIn('resolver',doc['package']);self.assertEqual(doc['workspace']['resolver'],'2')
    def test_only_eight_reviewed_sites_and_unchanged_serializer(self):
        self.assertEqual(len(m.owned_style.RULES['sites']),8)
        self.assertEqual(len(m.owned_style.RULES['format_sites']),1)
        self.assertEqual(self.final['src/serialize.rs'],self.u['src/serialize.rs'])
        changed={n for n,b in self.final.items() if n.endswith('.rs') and b!=self.u[n]}
        self.assertEqual(changed,{'src/lib.rs','src/state.rs','src/server.rs','src/transport/channel.rs'})
    def test_exclusive_export_summary_and_refusal_preserve_source(self):
        import stage
        original=m.BASE
        with tempfile.TemporaryDirectory(prefix='hc074-owned-export-',dir=m.ROOT/'target') as temporary:
            root=Path(temporary).resolve();self.assertTrue(root.is_relative_to((m.ROOT/'target').resolve()))
            try:
                m.BASE=root/'export';out=io.StringIO()
                with contextlib.redirect_stdout(out):stage.main()
                self.assertTrue(json.loads(out.getvalue())['complete'])
                before={p.relative_to(m.BASE).as_posix():m.sha(p.read_bytes()) for p in m.BASE.rglob('*') if p.is_file()}
                with self.assertRaises(FileExistsError):stage.main()
                self.assertEqual(before,{p.relative_to(m.BASE).as_posix():m.sha(p.read_bytes()) for p in m.BASE.rglob('*') if p.is_file()})
            finally:m.BASE=original
    def test_original_U_is_not_an_owned_shipping_identity(self):
        with self.assertRaises(ValueError):m.verify_source(self.u,self.u)
    def test_stale_lock_and_vcs_only_survive_as_historical_data(self):
        for name in ('Cargo.lock','Cargo.toml.orig','.cargo_vcs_info.json'):
            self.assertEqual(self.final['upstream/'+name],self.u[name])
        self.assertNotIn('.cargo_vcs_info.json',self.final)
        lock=tomllib.loads(self.final['Cargo.lock'].decode());root=[r for r in lock['package'] if 'source' not in r]
        self.assertEqual([(r['name'],r['version']) for r in root],[(m.NAME,m.VERSION)])
    def test_alias_and_features_keep_upstream_public_surface(self):
        old=tomllib.loads(self.u['Cargo.toml'].decode());new=tomllib.loads(self.final['Cargo.toml'].decode())
        for name in ('dependencies','dev-dependencies','features','lib','test'):self.assertEqual(old[name],new[name])
        self.assertEqual(new['package']['rust-version'],'1.88');self.assertEqual(new['lib']['name'],'chitchat')
        self.assertNotIn('patch',new)
    def test_changed_runtime_refuses_even_with_updated_receipt(self):
        files=dict(self.final);files['src/serialize.rs']+=b'\n'
        with self.assertRaises(ValueError):m.verify_source(files,self.u)
    def test_original_source_map_tampering_refuses(self):
        files=dict(self.final);row=json.loads(files['BACKPORT074.json']);row['canonical_U']['src/serialize.rs']='0'*64;files['BACKPORT074.json']=m.encoded(row)
        with self.assertRaises(ValueError):m.verify_source(files,self.u)
    def test_root_and_registry_scope_promotion_refuse(self):
        for key in ('root_adopted','registry_qualified','release_admitted'):
            files=dict(self.final);row=json.loads(files['BACKPORT074.json']);row[key]=True;files['BACKPORT074.json']=m.encoded(row)
            with self.subTest(key=key),self.assertRaises(ValueError):m.verify_source(files,self.u)
    def test_canonical_archive_is_deterministic_and_reconstructs(self):
        codec=m.codec();first=codec.pack(self.final,m.PREFIX)
        self.assertEqual(first,codec.pack(dict(reversed(list(self.final.items()))),m.PREFIX))
        self.assertEqual(codec.unpack(first,m.PREFIX,True),self.final)
    def test_missing_or_extra_source_member_refuses(self):
        for key in ('missing','extra'):
            files=dict(self.final)
            if key=='missing':del files['src/serialize.rs']
            else:files['src/foreign.rs']=b'// unreviewed\n'
            with self.subTest(key=key),self.assertRaises(ValueError):m.verify_source(files,self.u)
    def test_lock_preserves_every_registry_package(self):
        before=tomllib.loads(self.lock.decode());after=tomllib.loads(self.final['Cargo.lock'].decode())
        self.assertEqual([r for r in before['package'] if 'source' in r],[r for r in after['package'] if 'source' in r])
    def test_registry_row_has_explicit_upstream_dependency_sources(self):
        row=m.registry_row(self.final,'a'*64);manifest=tomllib.loads(self.final['Cargo.toml'].decode())
        self.assertEqual(row['name'],m.NAME);self.assertEqual(row['cksum'],'a'*64)
        self.assertTrue(all(d['registry']==m.CRATE_INDEX for d in row['deps']))
        self.assertEqual(row['features'],manifest['features']);self.assertFalse(row['yanked'])
    def crate_files(self):
        files=dict(self.final);files['Cargo.toml.orig']=files['Cargo.toml']
        files['Cargo.toml']=files['Cargo.toml'].split(b'\n[workspace]')[0]+b'\n'
        return files
    def test_normalized_complete_crate_reconciles(self):
        files=self.crate_files();raw=m.codec().pack(files,m.PREFIX)
        self.assertEqual(m.verify_crate(raw,self.u,self.final),files)
    def test_normalized_crate_cannot_hide_an_added_runtime(self):
        files=self.crate_files();files['src/foreign.rs']=b'// unreviewed\n'
        with self.assertRaises(ValueError):m.verify_crate(m.codec().pack(files,m.PREFIX),self.u,self.final)
    def test_normalized_crate_rejects_public_api_or_dependency_drift(self):
        for before,after in [(b'version = "=0.14.3"',b'version = "0.13"'),(b'name = "chitchat"\npath',b'name = "other"\npath')]:
            files=self.crate_files();files['Cargo.toml']=files['Cargo.toml'].replace(before,after)
            with self.subTest(before=before),self.assertRaises(ValueError):m.verify_crate(m.codec().pack(files,m.PREFIX),self.u,self.final)
    def test_normalized_crate_refuses_stale_manifest_or_lock(self):
        for name in ('Cargo.toml.orig','Cargo.lock'):
            files=self.crate_files();files[name]=self.u[name]
            with self.subTest(name=name),self.assertRaises(ValueError):m.verify_crate(m.codec().pack(files,m.PREFIX),self.u,self.final)

if __name__=='__main__':unittest.main(verbosity=2)
