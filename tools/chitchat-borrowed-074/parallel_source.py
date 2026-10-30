"""Independent-file CI reads; unchanged openat2 predicates and fresh bytes."""
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import sys
import tempfile
import unittest


def hashes(reader,root,names):
    names=list(names)
    if len(names)!=len(set(names)):raise ValueError('unique complete source members')
    if not names:return {}
    count=min(8,len(names));size=(len(names)+count-1)//count
    chunks=[names[n:n+size] for n in range(0,len(names),size)]
    with ThreadPoolExecutor(max_workers=count) as pool:
        jobs=[pool.submit(reader.hashes,root,chunk) for chunk in chunks]
        result={}
        for job in jobs:result.update(job.result())
    if list(result)!=names:raise ValueError('exact ordered complete fresh source inventory')
    return result


if __name__=='__main__':
    import recipe as p
    c=p.c
    path=c.ROOT/'tools/dbus-root-package-074/source_guard.py'
    c.need(c.sha(path.read_bytes())=='dddb71d7a1b1c81b434477531a5a5ae01e15858782aea1b18b8b705efe0c00a6','qualified source reader')
    reader=c.load(path,'parallel_control_original_reader')
    expected=json.loads(Path(sys.argv[1]).read_bytes())['source']
    class Controls(unittest.TestCase):
        def test_whole_actual_inventory(self):
            self.assertEqual(hashes(reader,c.ROOT,list(expected)),expected)
        def test_differential_fresh_bytes_missing_and_alias(self):
            with tempfile.TemporaryDirectory() as directory:
                root=Path(directory)
                names=['file'+str(i) for i in range(17)]
                for i,n in enumerate(names):(root/n).write_bytes(bytes([i])*37)
                self.assertEqual(hashes(reader,root,names),reader.hashes(root,names))
                (root/'file0').write_bytes(b'changed')
                self.assertEqual(hashes(reader,root,names),reader.hashes(root,names))
                with self.assertRaises(FileNotFoundError):hashes(reader,root,names+['absent'])
                (root/'alias').symlink_to('file0')
                with self.assertRaises(OSError):hashes(reader,root,names+['alias'])
                with self.assertRaises(ValueError):hashes(reader,root,['file0','file0'])
    suite=unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromTestCase(Controls))
    # The fourth original control pins an older historical Root lock.
    for name in ('test_fresh_bytes_order_missing_and_replacement','test_no_parent_leaf_alias_escape_directory_or_fifo',
                 'test_same_held_root_bytes_without_a_compiler_or_live_round'):
        suite.addTest(reader.Controls(name))
    result=unittest.TextTestRunner(verbosity=2).run(suite)
    raise SystemExit(0 if result.wasSuccessful() and result.testsRun==5 else 1)
