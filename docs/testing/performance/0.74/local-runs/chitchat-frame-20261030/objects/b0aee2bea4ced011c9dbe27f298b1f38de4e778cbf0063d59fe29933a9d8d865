"""Offline controls; no build, benchmark or archived producer execution."""
import copy,json,unittest
import checks as a
import packet as storage

def sample():
    native={k:0 for k in a.NATIVE}
    return dict(rust=dict(allocs=0,reallocs=0,requested=0,live=0,peak=0),
      native=dict(c=dict(native),rust=dict(native),errors=0),c_free_checkpoints=4)

class Controls(unittest.TestCase):
    def test_complete_windows_bodies(self):self.assertEqual(a.windows()['selected_positive_executions'],172)
    def test_original_F_remains_negative(self):
        result=json.loads(a.reference('capture/retention/analysis.json'))
        self.assertFalse(result['resource_gate_pass']);self.assertEqual(result['delta_bytes'],12)
    def test_meter_all_twelve_fields(self):
        value=a.meter(sample(),'operation')
        self.assertEqual(sum(len(v) for v in a.FIELDS.values()),12);self.assertEqual(set(value),set(a.FIELDS))
    def test_refuses_retained_C(self):
        row=sample();row['native']['c']['live']=1
        with self.assertRaises(ValueError):a.meter(row,'operation')
    def test_refuses_missing_physical_field(self):
        row=sample();del row['native']['rust']['calloc_calls']
        with self.assertRaises(ValueError):a.meter(row,'operation')
    def test_refuses_bool_counter(self):
        row=sample();row['native']['c']['peak']=False
        with self.assertRaises(ValueError):a.meter(row,'operation')
    def test_refuses_unexecuted_C_checkpoints(self):
        row=sample();row['c_free_checkpoints']=0
        with self.assertRaises(ValueError):a.meter(row,'operation')
    def test_refuses_incomplete_raw_marker(self):
        with self.assertRaises(ValueError):a.diagnostic(b'not-a-report','HC074_NATIVE_ALLOC:')
    def test_refuses_duplicate_raw_marker(self):
        with self.assertRaises(ValueError):a.diagnostic(b'HC074_NATIVE_ALLOC:{}\nHC074_NATIVE_ALLOC:{}\n','HC074_NATIVE_ALLOC:')

class WholeMatrixControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):cls.values=a.load_native()
    def mutate_G(self,fn):
        values=copy.deepcopy(self.values)
        for i in (2,3):fn(values[i])
        return values
    def test_complete_first_result_matches(self):
        result=json.loads((a.BASE/'analysis/native.json').read_bytes())
        self.assertEqual(a.review(self.values),result)
        self.assertEqual(result['cases'],[239,9,30])
        self.assertFalse(result['root_adopted']);self.assertFalse(result['CPU_qualified'])
    def test_complete_native_semantics(self):
        result=a.native_semantics(lambda name:(a.BASE/name).read_bytes())
        self.assertEqual(result['selected_positive_executions'],206)
    def test_native_images_were_fresh(self):
        self.assertEqual(a.native_builds(lambda name:(a.BASE/name).read_bytes())['fresh_images'],4)
    def test_refuses_missing_process(self):
        with self.assertRaises(ValueError):a.review(self.values[:-1])
    def test_refuses_changed_repeat(self):
        values=copy.deepcopy(self.values);values[3]['operation']['rows'][0]['records'][0]['rust']['peak']+=1
        with self.assertRaises(ValueError):a.review(values)
    def test_refuses_changed_full_workload(self):
        values=self.mutate_G(lambda cell:cell['caller']['rows'][0]['input'].__setitem__('mtu',1))
        with self.assertRaises(ValueError):a.review(values)
    def test_refuses_changed_operation_wire(self):
        values=self.mutate_G(lambda cell:cell['operation']['rows'][0].__setitem__('wire','00'))
        with self.assertRaises(ValueError):a.review(values)
    def test_flags_one_extra_physical_peak(self):
        def mutation(cell):
            for record in cell['operation']['rows'][0]['records']:record['native']['rust']['peak']+=10**9
        self.assertFalse(a.review(self.mutate_G(mutation))['allocation_retention_gate_pass'])
    def test_flags_returned_output_capacity(self):
        def mutation(cell):
            for record in cell['stateful']['rows'][0]['records']:record['finish_capacity']+=10**9
        result=a.review(self.mutate_G(mutation));self.assertFalse(result['allocation_retention_gate_pass'])
        self.assertTrue(any(r.get('checkpoint')=='finish' for r in result['regressions']))
    def test_refuses_retained_C_in_full_body(self):
        def mutation(cell):
            for record in cell['caller']['rows'][0]['records']:record['meter']['native']['c']['live']=1
        with self.assertRaises(ValueError):a.review(self.mutate_G(mutation))

class ArchiveControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.index=json.loads((storage.PACKET/'index.json').read_bytes())
        cls.read=lambda h:(storage.PACKET/'objects'/h).read_bytes()
    def test_refuses_scope_promotion(self):
        index=copy.deepcopy(self.index);index['CPU_qualified']=True
        with self.assertRaises(ValueError):storage.validate(index,type(self).read)
    def test_refuses_path_escape(self):
        index=copy.deepcopy(self.index);index['members']['../escape']=next(iter(index['members'].values()))
        with self.assertRaises(ValueError):storage.validate(index,type(self).read)
    def test_refuses_changed_whole_object(self):
        with self.assertRaises(ValueError):storage.validate(self.index,lambda h:type(self).read(h)+b'\n')
    def test_refuses_coherently_rehashed_green_result(self):
        index=copy.deepcopy(self.index);result=index['resource_result'];result['allocation_retention_gate_pass']=True;result['regressions']=[]
        raw=json.dumps(result).encode();digest=a.sha(raw)
        index['members']['capture/analysis/native.json']=dict(sha256=digest,bytes=len(raw))
        with self.assertRaises(ValueError):storage.validate(index,lambda h:raw if h==digest else type(self).read(h))
    def test_refuses_coherently_rehashed_cached_build(self):
        index=copy.deepcopy(self.index);name='capture/native-build-r2/build-G194/stdout.log'
        raw=type(self).read(index['members'][name]['sha256']);rows=raw.splitlines()
        for i,line in enumerate(rows):
            if not line.startswith(b'{'):continue
            row=json.loads(line)
            if row.get('reason')=='compiler-artifact' and row.get('executable'):
                row['fresh']=True;rows[i]=json.dumps(row).encode()
        raw=b'\n'.join(rows)+b'\n';digest=a.sha(raw);index['members'][name]=dict(sha256=digest,bytes=len(raw))
        with self.assertRaises(ValueError):storage.validate(index,lambda h:raw if h==digest else type(self).read(h))

if __name__=='__main__':unittest.main(verbosity=2)
