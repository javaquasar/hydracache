"""Offline receipt mutations and source derivation; never dispatch any peer."""
import json
import unittest
import analyze as a
import common as c
prepare=c.load(c.TOOLS/'prepare.py','compat_exact_offline_preparation')


class Accounting(unittest.TestCase):
    def mutate(self,name,operation):
        def get(path):
            raw=a.local(path)
            if path==name:
                row=json.loads(raw);operation(row);return json.dumps(row).encode()
            return raw
        return get

    def refuse(self,name,operation):
        with self.assertRaises((ValueError,KeyError)):a.reconcile(self.mutate(name,operation))

    def test_complete_matrix(self):
        result=a.reconcile();self.assertTrue(result['compatibility_gate_pass']);self.assertEqual(result['test_executions'],340)
        self.assertFalse(result['root_adopted']);self.assertFalse(result['CPU_qualified']);self.assertFalse(result['RSS_qualified'])

    def test_ownership_retirement(self):
        self.refuse(a.R1+'/windows-188/tests/execution.json',lambda r:r.update(retired=False))

    def test_owner_error(self):
        self.refuse(a.R0+'/gnu-194/native-envelope.json',lambda r:r.update(errors=['unproven retirement']))

    def test_nonzero(self):
        self.refuse(a.R1+'/gnu-188/native/check/execution.json',lambda r:r.update(exit=1))

    def test_whole_peer_required(self):
        self.refuse(a.R1+'/gnu-188/result.json',lambda r:r.update(complete=False))

    def test_original_inner_pass_cannot_replace_timeout(self):
        first=a.R0+'/gnu-188/'
        self.assertTrue(json.loads(a.local(a.R0+'/diagnosis/recovered-gnu-188/raw/session-result.json'))['complete'])
        with self.assertRaises(ValueError):a.peer(a.local,a.R0,'gnu-188',{})
        self.refuse(first+'result.json',lambda r:r.update(complete=True))

    def test_original_timeout_immutable(self):
        self.refuse(a.R0+'/windows-188/execution.json',lambda r:r.update(classification='success',exit=0))

    def test_bounds_and_poll_unchanged(self):
        for field,value in (('bound_seconds',1201),('capacity_poll_seconds',6),('cleanup_seconds',6)):
            with self.subTest(field=field):self.refuse(a.R1+'/windows-188/tests/execution.json',lambda r:r.update({field:value}))
        self.refuse(a.R1+'/windows-188/intent.json',lambda r:r.update(session_bound=1101))

    def test_no_deadline_reset(self):
        self.refuse(a.R1+'/windows-188/check/intent.json',lambda r:r.update(deadline=r['deadline']+1))

    def test_wrong_built_image(self):
        self.refuse(a.R1+'/gnu-188/native/tests/intent.json',lambda r:r.update(image_sha256='0'*64))

    def test_missing_test(self):
        self.refuse(a.R1+'/windows-188/cases.json',lambda r:r['selected'].pop())

    def test_wrong_catalog(self):
        self.refuse(a.R0+'/gnu-194/native/catalog.json',lambda r:r.pop())

    def test_no_resource_promotion(self):
        for flag in ('CPU_qualified','native_C_qualified','RSS_qualified','root_adopted'):
            with self.subTest(flag=flag):self.refuse(a.R1+'/windows-188/session-result.json',lambda r:r.update({flag:True}))

    def test_raw_semantics_and_qualifiers(self):
        name=a.R1+'/windows-188/tests/stdout.log';raw=a.local(name).decode()
        for change in (lambda r:r.update(seed=0),lambda r:r.update(resources_qualified=True),lambda r:r['rows'].pop()):
            body=c.diagnostic(raw,'HC074_COMPAT_CALLER:');change(body)
            altered='\n'.join('HC074_COMPAT_CALLER:'+json.dumps(body) if line.startswith('HC074_COMPAT_CALLER:') else line for line in raw.splitlines())
            with self.assertRaises(ValueError):a.semantic(a.local,a.R1,altered)

    def test_raw_test_accounting(self):
        name=a.R1+'/windows-188/tests/stdout.log';raw=a.local(name)
        for old,new in ((b'85 passed;',b'84 passed;'),(b'test delta::cost_review::allocator_control ... ok',b'test delta::cost_review::unaccounted ... ok')):
            self.assertIn(old,raw)
            with self.assertRaises(ValueError):a.reconcile(lambda n:raw.replace(old,new) if n==name else a.local(n))

    def test_original_source_derivation(self):
        members,get=prepare.packet();context=c.ROOT/'target'/a.R1
        prefix=prepare.PREFIX+'fixtures/Q/'
        expected={n[len(prefix):]:members[n]['sha256'] for n in members if n.startswith(prefix)}
        # Observer relative path happens to remain identical at the same depth.
        self.assertEqual(c.files(context/'fixtures/Q'),expected)
        portable=(c.ROOT/'tools/chitchat-inplace-074/allocation_tests_portable.rs').read_bytes()
        self.assertEqual(c.sha(portable),'ab5cf5e12f5dfbb956ebcee59f8af554cf3030544eba07bd2e928ad2d0b109df')
        self.assertTrue((context/'observer.rs').read_bytes().startswith(portable))
        original=get('source/tools/chitchat-caller-074/caller.rs')
        self.assertEqual((context/'caller.rs').read_bytes(),original+b'\n'+(c.TOOLS/'semantic_export.rs').read_bytes())
        self.assertFalse((context/'gnu-194').exists());self.assertFalse((context/'windows-194').exists())
        c.source() # Fresh immutable Root and ownership pins, no Git call.

    def test_original_selectors_unchanged(self):
        import run
        self.assertEqual(tuple(run.GROUPS),a.GROUPS);self.assertEqual(tuple(run.EXACT),a.EXACT)
        self.assertIn('should panic',a.local(a.R1+'/windows-188/tests/stdout.log').decode())


if __name__=='__main__':unittest.main()
