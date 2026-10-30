"""Full-lane scope/accounting controls; no WSL, Cargo or registry execution."""
import contextlib,io,json,sys,unittest
from unittest.mock import patch
import context as s

class Controls(unittest.TestCase):
    def setUp(self):
        self.names=[f'tests::case_{i:03}' for i in range(100)]
        self.text='\n'.join('test '+n+' ... ok' for n in self.names)+'\ntest result: ok. 100 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\n'
    def test_complete_original_catalog(self):self.assertEqual(s.full_outcome(self.text.encode(),self.names)['passed'],100)
    def test_omission_refuses(self):
        with self.assertRaises(ValueError):s.full_outcome(self.text.replace('test tests::case_000 ... ok\n','').encode(),self.names)
    def test_duplicate_refuses(self):
        with self.assertRaises(ValueError):s.full_outcome(self.text.replace('case_001','case_000').encode(),self.names)
    def test_ignored_refuses(self):
        with self.assertRaises(ValueError):s.full_outcome(self.text.replace('100 passed; 0 failed; 0 ignored','99 passed; 0 failed; 1 ignored').encode(),self.names)
    def test_failed_refuses(self):
        with self.assertRaises(ValueError):s.full_outcome(self.text.replace('100 passed; 0 failed','99 passed; 1 failed').encode(),self.names)
    def test_filtered_refuses(self):
        with self.assertRaises(ValueError):s.full_outcome(self.text.replace('0 filtered out','1 filtered out').encode(),self.names)
    def test_incomplete_output_refuses(self):
        with self.assertRaises(ValueError):s.full_outcome(self.text.split('test result:')[0].encode(),self.names)
    def test_registry_requires_both_full_compilers(self):self.assertEqual(s.prerequisites('registry'),('library194','library188'))
    def test_caller_requires_actual_registry(self):
        for compiler in ('194','188'):self.assertEqual(s.prerequisites('consumer'+compiler),('registry',))
    def test_unapproved_phase_refuses(self):
        with self.assertRaises(ValueError):s.prerequisites('publish')
    def test_helper_namespace_never_reopens_parent(self):
        parent=s.m.BASE;native=s.prior.NATIVE
        with patch.object(sys,'argv',['control','pure']),contextlib.redirect_stderr(io.StringIO()):
            module=s.registry_functions(s.BASE/'never-created',lambda *a,**kw:None)
        self.assertEqual(module.m.BASE,s.BASE);self.assertEqual(module.N,s.N)
        self.assertEqual(s.m.BASE,parent);self.assertEqual(s.prior.NATIVE,native)
        self.assertNotEqual(module.m.BASE,parent)

if __name__=='__main__':unittest.main(verbosity=2)
