"""Offline fault, source-equivalence and original fixture-refusal controls."""
import copy
import json
import re
import unittest
import common as c
import analyze as a


class Accounting(unittest.TestCase):
    def values(self):return a.evidence()
    def mutate(self,values,fn,index=0):
        for i in (2,3):
            for record in values[i]['rows'][index]['records']:fn(record)

    def test_exact_actual_positive_does_not_erase_component_negative(self):
        result=a.reconcile(self.values());self.assertEqual(result,json.loads((c.BASE/'analysis/result.json').read_bytes()))
        self.assertTrue(result['fixed_caller_resource_gate_pass']);self.assertEqual(result['comparisons'],{'B':30,'P':30})
        self.assertEqual(result['allocation_regressions'],[]);self.assertEqual(result['buffer_capacity_regressions'],[])
        for key in ('generic_component_gate_pass','CPU_qualified','RSS_qualified','allocator_retention_qualified','MSRV_qualified','Windows_qualified','root_adopted','release_admitted'):self.assertFalse(result[key])

    def test_every_allocation_field_refuses_individually(self):
        for scope,fields in a.FIELDS.items():
            for field in fields:
                values=self.values()
                def change(r):
                    stats=r['meter']['rust'] if scope=='logical' else r['meter']['native']['rust' if scope=='physical' else 'c']
                    stats[field]+=1
                self.mutate(values,change);self.assertFalse(a.reconcile(values)['fixed_caller_resource_gate_pass'])

    def test_each_capacity_growth_refuses(self):
        for buffer in range(3):
            values=self.values();self.mutate(values,lambda r:r['states'][-1]['capacity'].__setitem__(buffer,r['states'][-1]['capacity'][buffer]+1))
            self.assertFalse(a.reconcile(values)['fixed_caller_resource_gate_pass'])

    def test_order_payload_budget_or_semantics_cannot_drift(self):
        for kind in ('missing','order','payload','mtu','wire'):
            values=self.values()
            for i in (2,3):
                rows=values[i]['rows']
                if kind=='missing':rows.pop()
                elif kind=='order':rows.reverse()
                elif kind=='payload':rows[0]['input']['requests'][1]['value']='changed'
                elif kind=='mtu':rows[0]['input']['mtu']+=1
                else:
                    for r in rows[0]['records']:r['wire']='00'
            with self.assertRaises(ValueError):a.reconcile(values)

    def test_ambiguous_branch_and_stop_on_false_are_required(self):
        values=self.values();self.mutate(values,lambda r:r['paths'].__setitem__(1,0),21)
        with self.assertRaises(ValueError):a.reconcile(values)
        values=self.values()
        def change(r):r['decisions'].append(True);r['states'].append(copy.deepcopy(r['states'][-1]))
        self.mutate(values,change)
        with self.assertRaises(ValueError):a.reconcile(values)

    def test_observer_errors_nonrepeatability_and_scope_refuse(self):
        values=self.values();values[2]['rows'][0]['records'][0]['compressions']+=1
        with self.assertRaises(ValueError):a.reconcile(values)
        for field,value in (('live',1),('failed',1),('external_frees',1),('allocs',False),('requested',-1)):
            values=self.values();self.mutate(values,lambda r:r['meter']['native']['c'].__setitem__(field,value))
            with self.assertRaises(ValueError):a.reconcile(values)
        for field,value in (('CPU_qualified',True),('RSS_qualified',True),('scope','whole-product'),('seed',1)):
            values=self.values()
            for i in (2,3):values[i][field]=value
            with self.assertRaises(ValueError):a.reconcile(values)

    def test_first_fixture_refusals_and_current_canonical_table_remain(self):
        validator=c.load(c.TOOLS/'validate_table.py','caller_table_validator')
        with self.assertRaises(ValueError):validator.validate(json.loads((c.TOOLS/'cases-r1.json').read_bytes()))
        validator.validate(json.loads((c.TOOLS/'cases-r2.json').read_bytes()))
        generator=c.load(c.TOOLS/'generate_table.py','caller_independent_table')
        self.assertEqual(generator.table()['cases'],a.expected())
        review=json.loads((c.ROOT/'target/chitchat-caller-20261029/table-review/result.json').read_bytes())
        self.assertTrue(review['initial_table_refused']);self.assertFalse(review['Q_data_observed'])
        failure=(c.ROOT/'target/chitchat-caller-20261029-r1/control-B/native/6/stdout.log').read_text(encoding='utf8')
        self.assertIn('left: 50',failure);self.assertIn('right: 60',failure)

    def test_complete_runtime_bodies_and_observer_prefix_are_unchanged(self):
        producer=c.load(c.TOOLS/'prepare.py','caller_source_equivalence');_,get=producer.packet()
        for variant in ('B','P','Q'):
            prefix='capture/fixtures/'+variant+'/src/'
            actual=(c.BASE/'fixtures'/variant/'src/serialize.rs').read_text(encoding='utf8')
            at=actual.index('    pub(crate) fn caller_inventory(');start=actual.rfind('#[cfg(test)]',0,at)
            actual=actual[:start].rstrip()+'\n'
            actual=actual.replace('    #[cfg(test)]\n    crate::delta::cost_review::actual_caller::record_compression();\n','')
            self.assertEqual(actual.strip(),get(prefix+'serialize.rs').decode().strip())
            actual=(c.BASE/'fixtures'/variant/'src/delta.rs').read_text(encoding='utf8')
            actual=re.sub(r'\n#\[cfg\(test\)\]\nconst CALLER_VARIANT: &str = "[BPQ]";\n','\n',actual)
            strip=lambda v:re.sub(r'#\[path = "[^"\n]+"\]\npub\(crate\) mod cost_review;','',v).strip()
            self.assertEqual(strip(actual),strip(get(prefix+'delta.rs').decode()))
        observer=(c.BASE/'observer.rs').read_text(encoding='utf8')
        observer=re.sub(r'\n#\[path = "[^"\n]+"\]\npub\(crate\) mod actual_caller;\n','\n',observer)
        self.assertEqual(observer.strip(),get('capture/observer.rs').decode().strip())
        for name,digest in c.PINS.items():self.assertEqual(c.sha((c.ROOT/name).read_bytes()),digest)


if __name__=='__main__':unittest.main()
