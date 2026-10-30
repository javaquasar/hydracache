"""Data corruption controls; no Cargo, service, clock or native session."""
import gzip,io,json,tarfile,unittest
from unittest.mock import patch
import packet as p

class DataControls(unittest.TestCase):
    def archive(self,names):
        out=io.BytesIO()
        with tarfile.open(fileobj=out,mode='w') as archive:
            for name,kind in names:
                info=tarfile.TarInfo(name);info.type=kind;info.size=1 if kind==tarfile.REGTYPE else 0
                archive.addfile(info,io.BytesIO(b'x') if info.size else None)
        return gzip.compress(out.getvalue(),mtime=0)
    def test_contained_names(self):
        self.assertEqual(str(p.safe('source/src/lib.rs')),'source/src/lib.rs')
        for name in ('../escape','/absolute','C:/root','a\\b','a//b','a/../b'):
            with self.assertRaises(ValueError):p.safe(name)
    def test_regular_archive_restores_bytes_only(self):
        raw=self.archive([('hc074-copied-root/src/lib.rs',tarfile.REGTYPE)])
        self.assertEqual(p.source_files(raw),{'src/lib.rs':b'x'})
    def test_tar_traversal_refuses(self):
        with self.assertRaises(ValueError):p.source_files(self.archive([('hc074-copied-root/../escape',tarfile.REGTYPE)]))
    def test_tar_link_refuses(self):
        with self.assertRaises(ValueError):p.source_files(self.archive([('hc074-copied-root/link',tarfile.SYMTYPE)]))
    def test_duplicate_tar_refuses(self):
        entry=('hc074-copied-root/src/lib.rs',tarfile.REGTYPE)
        with self.assertRaises(ValueError):p.source_files(self.archive([entry,entry]))
    def test_wrong_source_prefix_refuses(self):
        with self.assertRaises(ValueError):p.source_files(self.archive([('elsewhere/src/lib.rs',tarfile.REGTYPE)]))
    def index(self):
        b=b'original';return dict(schema='hc074-chitchat-copied-root-v1',members={'data.json':dict(sha256=p.sha(b),bytes=len(b))},result={})
    def test_changed_whole_object_refuses_before_accounting(self):
        with self.assertRaisesRegex(ValueError,'whole exact'):p.validate(self.index(),lambda _:b'changed!')
    def test_changed_size_refuses(self):
        index=self.index();index['members']['data.json']['bytes']+=1
        with self.assertRaisesRegex(ValueError,'whole exact'):p.validate(index,lambda _:b'original')
    def test_bad_descriptor_refuses(self):
        index=self.index();index['members']['data.json']['bytes']=True
        with self.assertRaisesRegex(ValueError,'descriptor'):p.validate(index,lambda _:b'original')
    def test_unsafe_object_name_refuses(self):
        index=self.index();index['members']['../escape']=index['members'].pop('data.json')
        with self.assertRaisesRegex(ValueError,'contained'):p.validate(index,lambda _:b'original')
    def test_forged_summary_refuses(self):
        index=self.index();index['result']={'root_adopted':True}
        with patch.object(p,'reconcile',return_value={}):
            with self.assertRaisesRegex(ValueError,'accounting'):p.validate(index,lambda _:b'original')
    def test_success_requires_inner_result_and_exact_limits(self):
        prefix='second/native/stage';owner=dict(retired=True,exit=0,errors=[],classification='success')
        intent=dict(bound=1200,session_bound=1100,floor=4*1024**3,poll=5,cleanup=5,retry=False)
        data={prefix+'/'+n:p.encoded(v) for n,v in [('result.json',dict(complete=True)),('execution.json',owner),('intent.json',intent)]}
        with self.assertRaises(KeyError):p.phase(data,prefix)
        data[prefix+'/session-result.json']=p.encoded(dict(complete=True))
        self.assertTrue(p.phase(data,prefix)['complete'])
        data[prefix+'/intent.json']=p.encoded(dict(intent,session_bound=1101))
        with self.assertRaisesRegex(ValueError,'admission'):p.phase(data,prefix)

if __name__=='__main__':unittest.main(verbosity=2)
