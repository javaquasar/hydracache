"""Offline fault controls; never executes evidence or replays a cohort."""
import copy
import json
import unittest
import analyze as a
import common as c


class Accounting(unittest.TestCase):
    def values(self):return a.evidence()
    def mutate(self,values,fn):
        for position in (1,2):
            for record in values[position]['rows'][0]['records']:fn(record)

    def test_exact_positive_is_only_a_subgate(self):
        result=a.reconcile(self.values())
        self.assertTrue(result['allocation_dominance_pass']);self.assertEqual(result['allocation_changes'],[])
        self.assertEqual(result,json.loads((c.BASE/'analysis/result.json').read_bytes()))
        for key in ('CPU_qualified','RSS_qualified','retained_memory_qualified','stateful_retention_qualified','MSRV_qualified','Windows_qualified','root_adopted','release_admitted'):
            self.assertFalse(result[key])

    def test_each_allocation_metric_refuses_independently(self):
        for scope,fields in a.FIELDS.items():
            for field in fields:
                values=self.values()
                def change(r):
                    stats=r['rust'] if scope=='logical' else r['native']['rust' if scope=='physical' else 'c']
                    stats[field]+=1
                self.mutate(values,change)
                result=a.reconcile(values)
                self.assertFalse(result['allocation_dominance_pass']);self.assertEqual(len(result['allocation_regressions']),1)

    def test_missing_duplicate_reordered_semantics_refuse(self):
        for kind in ('missing','duplicate','order','wire','admission'):
            values=self.values()
            for i in (1,2):
                rows=values[i]['rows']
                if kind=='missing':rows.pop()
                elif kind=='duplicate':rows[1]=copy.deepcopy(rows[0])
                elif kind=='order':rows.reverse()
                elif kind=='wire':rows[0]['wire']='00'
                else:rows[0]['accepted']=not rows[0]['accepted']
            with self.assertRaises(ValueError):a.reconcile(values)

    def test_repeatability_seed_scope_and_counter_refusals(self):
        values=self.values();values[1]['rows'][0]['records'][0]['native']['c']['requested']+=1
        with self.assertRaises(ValueError):a.reconcile(values)
        values=self.values();values[1]['rows'].pop()
        with self.assertRaises(ValueError):a.reconcile(values)
        for field,value in (('live',1),('failed',1),('external_frees',1),('allocs',False),('requested',-1),('peak',0.5)):
            values=self.values();self.mutate(values,lambda r:r['native']['c'].__setitem__(field,value))
            with self.assertRaises(ValueError):a.reconcile(values)
        for field,value in (('seed',1),('scope','all-allocators'),('CPU_qualified',True),('allocator_retention_qualified',True)):
            values=self.values()
            for i in (1,2):values[i][field]=value
            with self.assertRaises(ValueError):a.reconcile(values)

    def test_baseline_precedes_Q_and_root_pins_remain_exact(self):
        p=c.ROOT/'target/chitchat-commit-P-20261029-r2'
        baseline=json.loads((p/'regression-P/native/regressions.json').read_bytes())
        self.assertEqual((baseline['passed'],baseline['failed']),(4,3))
        self.assertEqual(len(baseline['expected_refusals']),3)
        self.assertTrue(json.loads((p/'regression-P/result.json').read_bytes())['complete'])
        self.assertEqual(json.loads((c.BASE/'derivation.json').read_bytes())['profile'],'Q')
        for name,digest in c.PINS.items():self.assertEqual(c.sha((c.ROOT/name).read_bytes()),digest)

    def test_source_capsule_is_exact_and_old_allocator_unchanged(self):
        import zipfile
        frame=json.loads((c.BASE/'frame.json').read_bytes())
        with zipfile.ZipFile(c.BASE/'source.zip') as z:
            self.assertEqual(set(z.namelist()),set(frame)|{c.RELBASE+'/frame.json'})
            for name,digest in frame.items():self.assertEqual(c.sha(z.read(name)),digest)
        old=c.load(c.TOOLS/'prepare.py','commit_test_preparation')
        _,get=old.packet()
        self.assertEqual((c.BASE/'observer.rs').read_bytes(),get(old.PREFIX+'observer.rs'))
        for variant in ('B','Q'):
            for name in ('Cargo.toml','Cargo.lock'):
                owner='P' if variant=='Q' else variant
                self.assertEqual((c.BASE/'fixtures'/variant/name).read_bytes(),get(old.PREFIX+'fixtures/'+owner+'/'+name))


if __name__=='__main__':unittest.main()
