"""Offline refusal, source preservation and fixed-caller applicability controls."""
import copy
import json
import re
import unittest
import analyze as a
import common as c


class Accounting(unittest.TestCase):
    def values(self):return a.evidence()
    def baseline(self):
        values=self.values()
        for i in (0,1,4,5):
            peer='B' if i in (0,5) else 'P'
            for before,after in zip(values[i]['rows'],values[2]['rows'],strict=True):
                if peer=='B' and before['name'] in ('effectful-refusal-continue','effectful-refusal-no-cache'):continue
                before['records']=copy.deepcopy(after['records'])
                for record in before['records']:
                    record['writer_stack_bytes']=80
                    for state in record['states']:state['cache']=None
        return values
    def mutate(self,values,fn):
        for i in (2,3):
            for record in values[i]['rows'][0]['records']:fn(record)

    def test_actual_negative_preserves_scope_and_first_result(self):
        result=a.reconcile(self.values())
        self.assertEqual(result,json.loads((c.BASE/'analysis/result.json').read_bytes()))
        self.assertFalse(result['component_resource_gate_pass']);self.assertEqual(result['comparisons'],{'B':7,'P':9})
        c_cases={r['name'] for r in result['allocation_regressions'] if r['peer']=='B' and r['scope']=='c'}
        self.assertEqual(c_cases,{'continue-roomy','cheap-refusal-continue','continue-tight-twice','repeated-cheap-refusal'})
        self.assertEqual({r['name'] for r in result['buffer_capacity_regressions']},{'effectful-refusal-continue','effectful-refusal-no-cache'})
        for k in ('production_regression_proved','fixed_MTU_caller_qualified','CPU_qualified','RSS_qualified','allocator_retention_qualified','root_adopted','release_admitted'):self.assertFalse(result[k])

    def test_equal_component_counts_do_not_promote_other_gates(self):
        result=a.reconcile(self.baseline());self.assertTrue(result['component_resource_gate_pass'])
        self.assertFalse(result['root_adopted']);self.assertFalse(result['fixed_MTU_caller_qualified'])

    def test_every_predeclared_allocation_field_refuses(self):
        for scope,fields in a.FIELDS.items():
            for field in fields:
                values=self.baseline()
                def change(r):
                    stats=r['meter']['rust'] if scope=='logical' else r['meter']['native']['rust' if scope=='physical' else 'c']
                    stats[field]+=1
                self.mutate(values,change);self.assertFalse(a.reconcile(values)['component_resource_gate_pass'])

    def test_individual_capacity_growth_refuses(self):
        values=self.baseline();self.mutate(values,lambda r:r['states'][-1]['capacity'].__setitem__(0,r['states'][-1]['capacity'][0]+1))
        result=a.reconcile(values);self.assertFalse(result['component_resource_gate_pass']);self.assertTrue(result['buffer_capacity_regressions'])

    def test_partial_reordered_payload_budget_or_semantics_refuse(self):
        for kind in ('missing','order','payload','budget','wire'):
            values=self.values()
            for i in (2,3):
                rows=values[i]['rows']
                if kind=='missing':rows.pop()
                elif kind=='order':rows.reverse()
                elif kind=='payload':rows[0]['inputs'][0]='00'
                elif kind=='budget':rows[0]['steps'][0][1]+=1
                else:
                    for r in rows[0]['records']:r['wire']='00'
            with self.assertRaises(ValueError):a.reconcile(values)

    def test_repeatability_and_observer_failures_refuse(self):
        values=self.values();values[2]['rows'][0]['records'][0]['compressions']+=1
        with self.assertRaises(ValueError):a.reconcile(values)
        for field,value in (('live',1),('failed',1),('external_frees',1),('requested',-1),('allocs',False)):
            values=self.values();self.mutate(values,lambda r:r['meter']['native']['c'].__setitem__(field,value))
            with self.assertRaises(ValueError):a.reconcile(values)

    def test_production_methods_and_old_observer_are_unchanged(self):
        producer=c.load(c.TOOLS/'prepare.py','stateful_source_preservation')
        _,get=producer.packet();q='capture/chitchat-commit-Q-20261029-r2/';p='capture/chitchat-commit-P-20261029-r2/'
        for variant in ('B','P','Q'):
            prefix=p if variant=='P' else q
            old=get(prefix+'fixtures/'+variant+'/src/serialize.rs').decode()
            actual=(c.BASE/'fixtures'/variant/'src/serialize.rs').read_text(encoding='utf8')
            at=actual.index('    fn stateful_cache(');start=actual.rfind('#[cfg(test)]',0,at)
            actual=actual[:start].rstrip()+'\n'
            actual=actual.replace('    #[cfg(test)]\n    stateful_review::record();\n','')
            self.assertEqual(actual.strip(),old.strip())
            old=get(prefix+'fixtures/'+variant+'/src/delta.rs').decode()
            actual=(c.BASE/'fixtures'/variant/'src/delta.rs').read_text(encoding='utf8').replace('pub(crate) mod cost_review;','mod cost_review;')
            strip=lambda v:re.sub(r'#\[path = "[^"\n]+"\]\nmod cost_review;', '',v)
            self.assertEqual(strip(actual),strip(old))
        actual=(c.BASE/'observer.rs').read_text(encoding='utf8').split('\npub(crate) fn stateful_capture',1)[0]
        self.assertEqual(actual.strip(),get(q+'observer.rs').decode().strip())
        for name,digest in c.PINS.items():self.assertEqual(c.sha((c.ROOT/name).read_bytes()),digest)

    def test_same_budget_after_exact_tail_must_reject_at_old_floor(self):
        delta=(c.BASE/'fixtures/Q/src/delta.rs').read_text(encoding='utf8')
        self.assertEqual(len(re.findall(r'\bself\.mtu\b',delta)),1)
        self.assertIn('.try_append_with_budget(&delta_op, self.mtu)',delta)
        self.assertIn('stopping as soon as one of this methods returns `false`',delta)
        for threshold in (16,64,1000,16384):
            for output in (0,134,32774):
                for tail in (1,threshold//2,threshold):
                    for gap in (1,2,3):
                        budget=output+tail+4-gap
                        for n in (1,3,threshold,65535):
                            old=output+tail+n+(6 if tail+n>threshold else 3)+1
                            self.assertGreater(old,budget)


if __name__=='__main__':unittest.main()
