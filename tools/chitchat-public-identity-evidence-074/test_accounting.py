"""Coherently rehashed nominal evidence still must retain all real controls."""
import json,unittest
import packet as p
c=p.c

class Controls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):cls.data=p.collect();cls.result=p.a.reconcile(cls.data.__getitem__)
    def check(self,data):
        objects={c.sha(b):b for b in data.values()};index=dict(schema='hc074-chitchat-public-identity-v1',parent_sha256=p.s.PARENT_SHA,result=self.result,
            members={n:dict(sha256=c.sha(b),bytes=len(b)) for n,b in data.items()})
        return p.validate(index,objects.__getitem__)
    def mutate(self,name,key,value):
        data=dict(self.data);row=json.loads(data[name]);row[key]=value;data[name]=p.m.encoded(row)
        with self.assertRaises((ValueError,KeyError)):self.check(data)
    def test_whole_actual_outcomes(self):self.assertEqual(self.check(self.data),self.data)
    def test_retirement_required(self):self.mutate('native/build188/case-00/execution.json','retired',False)
    def test_original_deadline_required(self):self.mutate('native/negative194/config/execution.json','bound_seconds',781)
    def test_wrong_diagnostic_refuses(self):
        data=dict(self.data);name='native/negative194/config/stdout.log';rows=[json.loads(v) for v in data[name].decode().splitlines() if v.startswith('{')]
        for row in rows:
            if row.get('reason')=='compiler-message' and row['message']['level']=='error':row['message']['code']['code']='E0432'
        data[name]=b'\n'.join(json.dumps(r).encode() for r in rows)+b'\n'
        with self.assertRaises(ValueError):self.check(data)
    def test_positive_case_cannot_be_ignored(self):
        data=dict(self.data);name='native/build194/case-00/stdout.log';data[name]=data[name].replace(b'1 passed; 0 failed; 0 ignored',b'0 passed; 0 failed; 1 ignored')
        with self.assertRaises(ValueError):self.check(data)
    def test_aliases_cannot_be_swapped(self):
        data=dict(self.data);name='native/resolve/metadata-positive/stdout.log';row=json.loads(data[name]);root=next(v for v in row['packages'] if v['id']==row['resolve']['root'])
        for dep in root['dependencies']:dep['rename']='owned' if dep['rename']=='legacy' else 'legacy'
        data[name]=p.m.encoded(row)
        with self.assertRaises(ValueError):self.check(data)
    def test_complete_original_root_frame(self):
        data=dict(self.data);name='native/build188/intent.json';row=json.loads(data[name]);row['source']['Cargo.lock']='0'*64;data[name]=p.m.encoded(row)
        with self.assertRaises(ValueError):self.check(data)
    def test_probe_source_is_bound(self):
        data=dict(self.data);data['source/positive/src/lib.rs']+=b'\n'
        with self.assertRaises(ValueError):self.check(data)
    def test_adapter_compilation_cannot_be_inferred(self):self.mutate('native/build194/session-result.json','adapter_compiled',True)
    def test_name_query_cannot_prove_ownership(self):self.mutate('public-registry/response.json','ownership_proven',True)
    def test_actual_root_cannot_be_replaced(self):
        data=dict(self.data);data['reference/root/Cargo.toml']=data['reference/root/Cargo.toml'].replace(b'chitchat = "0.11.0"',b'chitchat = "0.12"')
        with self.assertRaises(ValueError):self.check(data)
    def test_unapplied_proposal_is_not_adoption(self):self.mutate('proposal/root-change.json','proposal_applied',True)
    def test_negative_must_actually_fail_compilation(self):self.mutate('native/negative188/transport/execution.json','exit',0)

if __name__=='__main__':unittest.main(verbosity=2)
