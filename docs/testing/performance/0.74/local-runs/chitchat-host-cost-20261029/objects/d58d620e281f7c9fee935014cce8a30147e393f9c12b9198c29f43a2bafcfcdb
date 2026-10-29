"""Offline method/derivation mutations before any clean-cost observations."""
import copy
import json
import unittest
import analyze as a
import common as c


def sample(variant='Q',mode='cpu',pid=1):
    stamp=dict(raw=10_000_000,mono=10_000_000,boot=10_000_000,cpu=5_000_000)
    snap=dict(rss=40960,pss=32768,private_clean=4096,private_dirty=8192,threads=1,
        heap={k:0 for k in a.HEAP});snap['heap'].update(arena=32768,uordblks=8192,fordblks=24576)
    return dict(variant=variant,mode=mode,seed=0x0741028,rows=json.loads((c.BASE/'input/expected-caller.json').read_bytes()),
        test_cfg=False,allocation_wrapper=False,cpu_repetitions=32,memory_epochs=6,memory_sweeps=8,warm_sweeps=4,
        pid=pid,start_ticks=1,boot_id='a'*36,resolutions_ns=[1]*4,elapsed=stamp,
        timings=[stamp.copy() for _ in range(30)] if mode=='cpu' else [],memory=[copy.deepcopy(snap) for _ in range(7)])


class Method(unittest.TestCase):
    def test_valid_controls(self):
        for mode in ('cpu','memory','control'):a.validate(sample(mode=mode),'Q',mode)

    def test_semantic_and_scope_refusals(self):
        for change in (lambda r:r.update(seed=0),lambda r:r['rows'].pop(),lambda r:r.update(test_cfg=True),
            lambda r:r.update(allocation_wrapper=True),lambda r:r.update(cpu_repetitions=33)):
            row=sample();change(row)
            with self.assertRaises(ValueError):a.validate(row,'Q','cpu')

    def test_clock_refusals(self):
        for change in (lambda r:r['elapsed'].update(boot=20_000_000),lambda r:r['elapsed'].update(raw=0),
            lambda r:r['timings'][0].update(cpu=0),lambda r:r['timings'][0].update(cpu=20_000_000)):
            row=sample();change(row)
            with self.assertRaises(ValueError):a.validate(row,'Q','cpu')

    def test_memory_refusals(self):
        for change in (lambda r:r['memory'].pop(),lambda r:r['memory'][1].update(threads=2),lambda r:r['memory'][1]['heap'].pop('arena')):
            row=sample(mode='memory');change(row)
            with self.assertRaises(ValueError):a.validate(row,'Q','memory')

    def test_estimator_and_no_deletion(self):
        self.assertEqual(a.interval([1]*5)['upper95'],1)
        self.assertAlmostEqual(a.interval([2]*5)['ratio'],2)
        for bad in ([1]*4,[0]*5,[float('nan')]*5):
            with self.assertRaises(ValueError):a.interval(bad)
        order=('B','P','Q','Q','P','B')*5
        cpu=[sample(v,'cpu',i+1) for i,v in enumerate(order)]
        memory=[sample(v,'memory',i+31) for i,v in enumerate(order)]
        self.assertTrue(a.reconcile(cpu,memory)['cpu_nonregression_gate_pass'])
        cpu[2]['timings'][0]['cpu']*=2
        self.assertFalse(a.reconcile(cpu,memory)['cpu_nonregression_gate_pass'])
        memory[2]['memory'][1]['rss']+=4096
        self.assertFalse(a.reconcile(cpu,memory)['memory_growth_screen_pass'])
        with self.assertRaises(ValueError):a.reconcile(cpu[:-1],memory)

    def test_exact_runtime_derivation(self):
        prepare=c.load(c.TOOLS/'prepare.py','clean_cost_source_derivation');members,get=prepare.packet()
        for variant in ('B','P','Q'):
            prefix=prepare.PREFIX+'fixtures/'+variant+'/'
            original={n[len(prefix):]:get(n) for n in members if n.startswith(prefix)}
            folder=c.BASE/'fixtures'/variant
            self.assertEqual(set(c.files(folder)),set(original)|{'cost-main.rs'})
            for name,body in original.items():
                actual=(folder/name).read_bytes()
                if name=='Cargo.toml':body=body.replace(b'[dev-dependencies.serde_json]',b'[dependencies.serde_json]')
                if name in ('src/delta.rs','src/lib.rs','Cargo.toml'):self.assertTrue(actual.startswith(body),variant+'/'+name)
                else:self.assertEqual(actual,body,variant+'/'+name)
            probe=(c.BASE/('probe-'+variant+'.rs')).read_bytes()
            self.assertTrue(probe.startswith((c.TOOLS/'probe.rs').read_bytes()))
        c.source();self.assertEqual(c.protocol()['seed'],0x0741028)


if __name__=='__main__':unittest.main()
