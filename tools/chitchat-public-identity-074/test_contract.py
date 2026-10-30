"""Pure source/identity/diagnostic controls; no Cargo, WSL or HTTP execution."""
import copy,json,tomllib,unittest
from pathlib import Path
import contract as s
c=s.c

class Controls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):cls.body=(s.ROOT/'crates/hydracache-cluster-chitchat/src/lib.rs').read_bytes().decode()
    def test_exact_boundary_and_direct_consumer_audit(self):
        result=s.audit(s.ROOT);self.assertEqual(result['public_methods'],21);self.assertEqual(set(result['exposed']),set(s.EXPOSED));self.assertEqual(len(result['direct']),4)
        self.assertFalse(result['full_adapter_compiled']);self.assertFalse(result['root_adopted'])
    def test_transport_boundary_cannot_disappear(self):
        with self.assertRaises(ValueError):s.signatures(self.body.replace('transport: &dyn Transport','transport: &dyn OtherTransport'))
    def test_configuration_type_drift_refuses(self):
        with self.assertRaises(ValueError):s.signatures(self.body.replace('config: FailureDetectorConfig','config: DifferentConfig'))
    def test_extra_public_method_refuses(self):
        with self.assertRaises(ValueError):s.signatures(self.body+'\n    pub fn extra() {}\n')
    def test_manifest_has_two_normal_registry_dependencies(self):
        for v in s.VARIANTS:
            d=tomllib.loads(s.manifest(v).decode());self.assertEqual(set(d['dependencies']),{'legacy','owned'});self.assertNotIn('patch',d)
            self.assertEqual(d['package']['rust-version'],'1.88');self.assertFalse(d['package']['publish'])
            self.assertTrue(all('path' not in r and 'git' not in r for r in d['dependencies'].values()))
    def test_unknown_variant_refuses(self):
        with self.assertRaises(ValueError):s.manifest('publish')
    def test_qualified_parent_and_exact_real_archive(self):
        _,raw=s.parent();self.assertEqual(c.sha(raw),s.CRATE_SHA)
    def diagnostic(self,code='E0308'):
        self.src=s.ROOT/'target/never-created-identity-control/config/src/lib.rs'
        return [dict(reason='compiler-message',target=dict(src_path=str(self.src)),message=dict(level='error',code=dict(code=code),spans=[dict(file_name='src/lib.rs',is_primary=True,line_start=2)])),dict(reason='build-finished',success=False)]
    def test_actual_diagnostic_shape_accepts_expected_nominal_failure(self):self.assertTrue(s.negative(self.diagnostic(),'config',self.src)['complete'])
    def test_wrong_error_code_refuses(self):
        data=self.diagnostic('E0432')
        with self.assertRaises(ValueError):s.negative(data,'config',self.src)
    def test_dependency_build_error_cannot_count(self):
        data=self.diagnostic();data[0]['target']['src_path']='/unrelated/dependency/src/lib.rs'
        with self.assertRaises(ValueError):s.negative(data,'config',self.src)
    def test_missing_primary_probe_span_refuses(self):
        data=self.diagnostic();data[0]['message']['spans'][0]['is_primary']=False
        with self.assertRaises(ValueError):s.negative(data,'config',self.src)
    def test_different_probe_line_refuses(self):
        data=self.diagnostic();data[0]['message']['spans'][0]['line_start']=3
        with self.assertRaises(ValueError):s.negative(data,'config',self.src)
    def test_success_is_not_a_negative_control(self):
        data=self.diagnostic();data[-1]['success']=True
        with self.assertRaises(ValueError):s.negative(data,'config',self.src)
    def test_ambiguous_lock_edge_refuses(self):
        raw=b'[[package]]\nname="root"\nversion="0.0.0"\ndependencies=["child"]\n[[package]]\nname="child"\nversion="1.0.0"\n[[package]]\nname="child"\nversion="2.0.0"\n'
        with self.assertRaises(ValueError):s.edges(raw)
    def test_explicit_lock_version_is_unambiguous(self):
        raw=b'[[package]]\nname="root"\nversion="0.0.0"\ndependencies=["child 2.0.0"]\n[[package]]\nname="child"\nversion="1.0.0"\n[[package]]\nname="child"\nversion="2.0.0"\n'
        self.assertEqual(s.edges(raw)['root','0.0.0'],{('child','2.0.0')})

if __name__=='__main__':unittest.main(verbosity=2)
