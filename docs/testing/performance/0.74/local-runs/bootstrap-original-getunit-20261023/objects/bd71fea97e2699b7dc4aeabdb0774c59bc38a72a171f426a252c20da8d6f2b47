import tempfile
from pathlib import Path
import unittest
import seal
class SealControls(unittest.TestCase):
    def test_paths_refuse_escape(self):
        for name in ('../secret','C:/secret','/etc/shadow','a\\b'):
            with self.assertRaises(ValueError): seal.relative(name)
    def test_objects_are_exact_and_cannot_be_relabelled_as_admission(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);(root/'objects').mkdir()
            raw=b'exact\r\n';sha=seal.digest(raw)
            (root/'objects'/sha).write_bytes(raw)
            index={'files':{'fixture/stdout.log':{'sha256':sha,'bytes':len(raw)}},'admission':False,'server':False,'product':False}
            seal.publish(root/'index.json',index)
            seal.verify(root)
            (root/'objects'/sha).write_bytes(b'drift\n')
            with self.assertRaises(ValueError): seal.verify(root)
    def test_first_archive_cannot_overwrite(self):
        with tempfile.TemporaryDirectory() as tmp:
            target=Path(tmp)/'result.json';seal.publish(target,{'first':True})
            with self.assertRaises(FileExistsError): seal.publish(target,{'first':False})
if __name__=='__main__': unittest.main()
