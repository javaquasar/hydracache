"""Pure declaration/diagnostic controls, never a substitute for real Cargo."""
import copy,json,tomllib,unittest
import caller_context as s
runner=s.c.load(s.HERE/'runner.py','hc074_typed_caller_diagnostic_controls')

class Controls(unittest.TestCase):
    def test_legacy_and_owned_are_coordinated_explicitly(self):
        b=tomllib.loads(s.manifest('B').decode());o=tomllib.loads(s.manifest('O').decode())
        self.assertEqual(b['dependencies']['chitchat'],'=0.11.1')
        self.assertEqual(o['dependencies']['chitchat'],dict(package='hydra-chitchat',version='=0.11.1-hydra.1',registry='hc074-draft'))
    def test_mixed_uses_real_owned_adapter_and_legacy_argument(self):
        d=tomllib.loads(s.manifest('mixed').decode());self.assertIn('/copies/O/crates/',d['dependencies']['hydracache-cluster-chitchat']['path']);self.assertEqual(d['dependencies']['chitchat'],'=0.11.1')
    def test_each_is_a_separate_nonpublishing_workspace(self):
        for v in s.VARIANTS:
            d=tomllib.loads(s.manifest(v).decode());self.assertFalse(d['package']['publish']);self.assertIn('workspace',d);self.assertEqual(d['package']['rust-version'],'1.88')
    def test_unknown_variant_cannot_dispatch(self):
        with self.assertRaises(ValueError):s.manifest('publish')
    def messages(self):
        source=(s.HERE/'caller.rs').read_bytes();rows=[]
        for i,line in enumerate(source.decode().splitlines(),1):
            if any(x in line for x in ('config.failure_detector_config(detector)','discovery.chitchat_id()','spawn_with_transport(config, channel)')):
                rows.append(dict(reason='compiler-message',package_id='actual-mixed',message=dict(level='error',code=dict(code='E0277' if 'spawn_with_transport' in line else 'E0308'),spans=[dict(is_primary=True,file_name='src/lib.rs',line_start=i)])))
        rows.append(dict(reason='build-finished',success=False));return source,rows
    def run_rows(self,rows,source):return runner.negative(b'\n'.join(json.dumps(r).encode() for r in rows),source,'actual-mixed')
    def test_three_exact_boundary_diagnostics_are_accounted(self):
        source,rows=self.messages();self.assertTrue(self.run_rows(rows,source)['expected_refusal'])
    def test_absolute_original_span_is_supported(self):
        source,rows=self.messages()
        for row in rows[:-1]:row['message']['spans'][0]['file_name']=str(s.N/'source/mixed/src/lib.rs')
        self.assertTrue(self.run_rows(rows,source)['complete'])
    def test_dependency_error_cannot_count_as_type_control(self):
        source,rows=self.messages();rows[0]['package_id']='dependency'
        with self.assertRaises(ValueError):self.run_rows(rows,source)
    def test_same_codes_on_wrong_lines_refuse(self):
        source,rows=self.messages();rows[0]['message']['spans'][0]['line_start']=1
        with self.assertRaises(ValueError):self.run_rows(rows,source)
    def test_swapped_boundary_codes_refuse(self):
        source,rows=self.messages();rows[0]['message']['code']['code'],rows[2]['message']['code']['code']=rows[2]['message']['code']['code'],rows[0]['message']['code']['code']
        with self.assertRaises(ValueError):self.run_rows(rows,source)
    def test_unrelated_same_named_source_refuses(self):
        source,rows=self.messages();rows[0]['message']['spans'][0]['file_name']='/tmp/other/src/lib.rs'
        with self.assertRaises(ValueError):self.run_rows(rows,source)
    def test_missing_or_extra_error_refuses(self):
        source,rows=self.messages()
        for bad in (rows[1:],rows+[copy.deepcopy(rows[0])]):
            with self.assertRaises(ValueError):self.run_rows(bad,source)
    def test_successful_build_cannot_count_as_refusal(self):
        source,rows=self.messages();rows[-1]['success']=True
        with self.assertRaises(ValueError):self.run_rows(rows,source)
    def test_missing_finished_record_refuses(self):
        source,rows=self.messages()
        with self.assertRaises(ValueError):self.run_rows(rows[:-1],source)
    def test_actual_download_map_matches_whole_qualified_archive(self):
        raw=(s.parent.INPUT/'owned.crate').read_bytes();values={n:s.c.sha(b) for n,b in s.m.codec().unpack(raw,'hydra-chitchat-0.11.1-hydra.1').items()};s.download(raw,values)
        values['src/lib.rs']='0'*64
        with self.assertRaises(ValueError):s.download(raw,values)
    def test_changed_qualified_archive_refuses(self):
        raw=(s.parent.INPUT/'owned.crate').read_bytes()
        with self.assertRaises(ValueError):s.download(raw+b'changed',{})

if __name__=='__main__':unittest.main(verbosity=2)
