"""Coherently rehashed complete-lane evidence must reject incomplete work."""
import json,unittest
import packet as p
c=p.c

class Controls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data=p.collect();cls.old=p.old_data()
        cls.result=p.r.reconcile(lambda n:cls.data['capture/'+n],cls.old)
    def check(self,data):
        objects={c.sha(b):b for b in data.values()}
        index=dict(schema='hc074-chitchat-package-complete-v1',parent_sha256=p.s.SHA,result=self.result,
            members={n:dict(sha256=c.sha(b),bytes=len(b)) for n,b in data.items()})
        return p.validate(index,objects.__getitem__)
    def mutate(self,name,key,value):
        data=dict(self.data);row=json.loads(data[name]);row[key]=value;data[name]=p.m.encoded(row)
        with self.assertRaises((ValueError,KeyError)):self.check(data)
    def test_whole_first_data(self):self.assertEqual(self.check(self.data),self.data)
    def test_full_library_bound_is_frozen(self):self.mutate('capture/native/library194/full-library/execution.json','bound_seconds',781)
    def test_retirement_required(self):self.mutate('capture/native/consumer188/case-00/execution.json','retired',False)
    def test_scope_cannot_be_promoted(self):self.mutate('capture/native/registry/registry-review.json','public_registry_qualified',True)
    def test_filtered_case_cannot_pass(self):
        data=dict(self.data);name='capture/native/library188/full-library/stdout.log'
        data[name]=data[name].replace(b'0 filtered out;',b'1 filtered out;')
        with self.assertRaises(ValueError):self.check(data)
    def test_checksum_refusal_required(self):self.mutate('capture/native/registry/checksum-negative/execution.json','exit',0)
    def test_no_patch_fallback(self):self.mutate('capture/native/registry/registry-review.json','patch_fallback',True)
    def test_whole_root_map(self):
        name='capture/native/library188/intent.json';data=dict(self.data);row=json.loads(data[name]);row['source']['Cargo.lock']='0'*64;data[name]=p.m.encoded(row)
        with self.assertRaises(ValueError):self.check(data)
    def test_registry_source_cannot_be_path(self):self.mutate('capture/state/consumer.json','registry','path+file:///fake')
    def test_library_workload_cannot_be_filtered(self):
        name='capture/native/library194/full-library/intent.json';data=dict(self.data);row=json.loads(data[name]);row['argv'].append('only-one');data[name]=p.m.encoded(row)
        with self.assertRaises(ValueError):self.check(data)
    def test_negative_parent_cannot_be_rewritten(self):
        data=dict(self.data);data['reference/parent-index.json']+=b'\n'
        with self.assertRaises(ValueError):self.check(data)
    def test_parent_import_namespace_is_isolated(self):
        import sys
        child=sys.modules['review'];loaded=p.r.load_parent()
        self.assertIs(sys.modules['review'],child)
        self.assertIsNot(loaded.r,child)
        self.assertEqual(loaded.r.BASE,p.s.PARENT)

if __name__=='__main__':unittest.main(verbosity=2)
