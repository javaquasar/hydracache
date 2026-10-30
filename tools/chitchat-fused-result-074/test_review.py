"""Offline negative controls. No execution, compilation or measurement."""
import copy
import ast
import json
import unittest
import archive as a

class Review(unittest.TestCase):
    @classmethod
    def setUpClass(cls):cls.data=a.collect()
    def reject(self,name,mutation):
        data=dict(self.data);value=json.loads(data[name]);mutation(value);data[name]=json.dumps(value).encode()
        with self.assertRaises((ValueError,KeyError,AssertionError)):a.r.reconcile(data.__getitem__)
    def test_complete_declined_result(self):self.assertFalse(a.r.reconcile(self.data.__getitem__)['resource_gate_pass'])
    def test_cached_candidate_artifact(self):
        data=dict(self.data);name='capture/retention/build3-F/stdout.log';rows=[]
        for line in data[name].decode().splitlines():
            row=json.loads(line)
            if row.get('reason')=='compiler-artifact' and row.get('executable'):row['fresh']=True
            rows.append(json.dumps(row))
        data[name]='\n'.join(rows).encode()
        with self.assertRaises(ValueError):a.r.reconcile(data.__getitem__)
    def test_changed_workload(self):self.reject('capture/retention/probe-F/result.json',lambda v:v['observation'].update(budget=168))
    def test_removed_first_failure(self):self.reject('capture/counterexample-B/result.json',lambda v:v.update(expected_failure=False))
    def test_reclassified_negative(self):self.reject('capture/retention/analysis.json',lambda v:v.update(resource_gate_pass=True))
    def test_missing_case(self):self.reject('capture/length-guard/case9-F/result.json',lambda v:v.update(passed=0))
    def test_raised_bound(self):self.reject('capture/length-guard/case9-F/intent.json',lambda v:v.update(bound=6))
    def test_stale_producer_source(self):
        data=dict(self.data);data['capture/contexts/pre-stop/tools/chitchat-fused-074/run.py']+=b'\n'
        with self.assertRaises(ValueError):a.r.reconcile(data.__getitem__)
    def test_corrupt_archive_object(self):
        raw=b'payload';index=dict(schema='hc074-chitchat-fused-declined-v1',parent_sha256=a.PARENT_SHA,
          decision='declined-first-retained-output-regression',root_adopted=False,release_admitted=False,
          CPU_qualified=False,RSS_qualified=False,further_measurements_allowed=False,
          members={'capture/x':dict(sha256=a.r.sha(raw),bytes=len(raw))})
        with self.assertRaises(ValueError):a.validate(index,lambda h:b'changed')
    def test_archive_path_escape(self):
        index=dict(schema='hc074-chitchat-fused-declined-v1',parent_sha256=a.PARENT_SHA,
          decision='declined-first-retained-output-regression',root_adopted=False,release_admitted=False,
          CPU_qualified=False,RSS_qualified=False,further_measurements_allowed=False,members={'../escape':{}})
        with self.assertRaises(ValueError):a.validate(index,lambda h:b'')
    def test_public_traits_unchanged(self):
        source=[self.data['capture/fixtures/'+v+'/src/serialize.rs'].decode() for v in ('B','F')]
        for marker,end in [('pub trait Serializable {','pub trait Deserializable'),('pub trait Deserializable: Sized {','impl Serializable for u8')]:
            parts=[s[s.index(marker):s.index(end)] for s in source];self.assertEqual(parts[0],parts[1])
    def test_stack_envelope_is_not_promoted(self):
        value=json.loads(self.data['capture/preparation/bounds.json'])
        self.assertEqual(value['thresholds'],65535);self.assertEqual(value['storage_envelope'],262146)
        self.assertIs(value['stack_RSS_qualified'],False);self.assertIs(value['mtu_upper_limit_added'],False)
    def test_execution_seals_precede_effects(self):
        for name,function in [('tools/chitchat-fused-074/run.py','main'),('tools/chitchat-fused-074/run.py','command'),
          ('tools/chitchat-fused-result-074/native.py','outer'),('tools/chitchat-fused-result-074/retention.py','command')]:
            tree=ast.parse((a.ROOT/name).read_text());node=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name==function)
            self.assertIn('STOP.json',ast.unparse(node.body[0]))
            self.assertIn('p.EXECUTION_ALLOWED',ast.unparse(node.body[0]))
        tree=ast.parse((a.ROOT/'tools/chitchat-fused-074/recipe.py').read_text())
        node=next(n for n in tree.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='EXECUTION_ALLOWED' for t in n.targets))
        self.assertIs(ast.literal_eval(node.value),False)

if __name__=='__main__':unittest.main(verbosity=2)
