"""Generated source/evidence view controls; no model or live process execution."""
from copy import deepcopy
import io
import json
from pathlib import Path
import stat
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import run as runner
import view_inputs as subject


class HistoryViewTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.m=subject.manifest();parent=(runner.ROOT/'target').resolve()
        cls.temporary=tempfile.TemporaryDirectory(dir=parent,prefix='ahv-test-')
        cls.generated=Path(cls.temporary.name).resolve()
        subject.need(cls.generated.parent==parent and cls.generated.name.startswith('ahv-test-'),'Unsafe test cleanup target')
        cls.root,cls.archive=runner.materialize(runner.ROOT,cls.generated,cls.m)
        cls.result,cls.stdout,cls.stderr=runner.worker(cls.root,cls.m)

    @classmethod
    def tearDownClass(cls):cls.temporary.cleanup()

    def test_actual_frozen_replay_uses_separate_modules_and_equal_facts(self):
        self.assertEqual(self.result['tree']['files'],1576)
        self.assertEqual((self.result['tree']['sources'],self.result['tree']['packets'],self.result['tree']['packet_members']),(129,24,1423))
        self.assertIs(self.result['historical_facts_equal'],True)
        self.assertIs(self.result['old_readers_unchanged'],True)
        self.assertEqual(self.stderr,b'')

    def test_text_image_adapter_preserves_qualification_raw_bytes(self):
        name='docs/testing/performance/0.74/qualification-manifest.toml'
        data=(self.root/name).read_bytes();pin='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'
        self.assertEqual(subject.sha(data),pin);self.assertEqual(data.count(b'\r'),0)
        archived=data.replace(b'\n',b'\r\n');entry=('source',pin,None)
        subject.check_bytes(archived,entry,archived=True)
        with self.assertRaises(ValueError):subject.check_bytes(archived,entry)
        self.assertEqual(subject.canonical(archived),data)
        with self.assertRaises(ValueError):subject.check_bytes(archived,('data',pin,len(data)))

    def test_history_success_does_not_admit_changed_current_copy(self):
        root=self.generated/'isolated-current';file=root/subject.FIXTURE;file.parent.mkdir(parents=True)
        file.write_bytes((self.root/subject.FIXTURE).read_bytes())
        self.assertIs(subject.current_fixture_identity(root)['current_fixture_baseline_identity'],True)
        file.write_bytes(file.read_bytes()+b'\n// hypothetical current patch\n')
        with self.assertRaises(ValueError):subject.current_fixture_identity(root)
        self.assertIs(self.result['history_only'],True)
        self.assertIs(self.result['runtime_candidate_guard_implemented'],False)
        self.assertEqual(subject.verify_tree(self.root,self.m),self.archive['tree'])

    def test_manifest_source_membership_modules_and_bounds_are_fixed(self):
        for key,value in [('source_commit','0'*40),('packet_members',1424),('reader','other.py'),('source_representation','crlf')]:
            altered=deepcopy(self.m);altered[key]=value
            with self.assertRaises(ValueError):subject.validate_manifest(altered)
        for key in ('source_files','reader_modules','packets'):
            altered=deepcopy(self.m)
            if type(altered[key]) is dict:altered[key].pop(next(iter(altered[key])))
            else:altered[key].pop()
            with self.assertRaises(ValueError):subject.validate_manifest(altered)
        altered=deepcopy(self.m);altered['bounds']['files']+=1
        with self.assertRaises(ValueError):subject.validate_manifest(altered)

    def test_path_traversal_aliases_and_devices_refuse(self):
        for name in ('.','..','/a','a/../b','a//b','a/./b','C:/a','a\\b','a.','NUL','a/COM1.txt','a/LPT0'):
            with self.assertRaises(ValueError,msg=name):subject.safe_name(name)

    def test_duplicate_json_and_nonfinite_values_refuse(self):
        for text in ('{"x":1,"x":2}','{"x":NaN}','{"x":Infinity}'):
            with self.assertRaises(ValueError):subject.decode(text)

    def fake_extract(self,names,mutate=None):
        stream=io.BytesIO()
        with zipfile.ZipFile(stream,'w') as z:
            for name,data,mode in names:
                info=zipfile.ZipInfo(name)
                if mode is not None:info.external_attr=mode<<16
                z.writestr(info,data)
        stream.seek(0);m=deepcopy(self.m)
        if mutate:mutate(m)
        expected={'one.rs':('source',subject.sha(b'one\n'),None)}
        with tempfile.TemporaryDirectory(dir=self.generated,prefix='extract-') as directory:
            with patch.object(subject,'expected_members',return_value=expected):return subject.extract(stream,Path(directory),m)

    def test_archive_exact_membership_hashes_and_case_refuse(self):
        for names in ([('other.rs',b'one\n',None)], [('one.rs',b'changed\n',None)],
            [('one.rs',b'one\n',None),('One.rs',b'one\n',None)],
            [('one.rs',b'one\n',None),('unknown/',b'',stat.S_IFDIR)]):
            with self.assertRaises(ValueError):self.fake_extract(names)
        self.assertEqual(self.fake_extract([('one.rs',b'one\r\n',None)])['files'],1)

    def test_archive_links_traversal_and_size_refuse(self):
        for names in ([('../escape',b'x',None)], [('one.rs',b'one\n',stat.S_IFLNK|0o777)],
            [('one.rs',b'one\n',stat.S_IFIFO|0o600)], [('one.rs',b'one\n',stat.S_IFCHR|0o600)]):
            with self.assertRaises(ValueError):self.fake_extract(names)
        with self.assertRaises(ValueError):self.fake_extract([('one.rs',b'one\n',None)],lambda m:m['bounds'].update(file_bytes=3))
        with self.assertRaises(ValueError):self.fake_extract([('one.rs',b'one\n',None)],lambda m:m['bounds'].update(total_bytes=3))

    def test_inventory_digest_and_member_shape_refuse(self):
        data=json.dumps(dict(files=[dict(path='one',bytes=1,sha256='0'*64)])).encode()
        toy=dict(source_files={},packets={'packet':subject.sha(data)},packet_members=1,bounds=self.m['bounds'])
        self.assertEqual(len(subject.expected_members(toy,lambda _:data)),2)
        with self.assertRaises(ValueError):subject.expected_members(toy,lambda _:data+b' ')
        for entry in (dict(path='../one',bytes=1,sha256='0'*64),dict(path='one',bytes=True,sha256='0'*64)):
            changed=json.dumps(dict(files=[entry])).encode();altered=dict(toy,packets={'packet':subject.sha(changed)})
            with self.assertRaises(ValueError):subject.expected_members(altered,lambda _:changed)

    def test_read_scope_denies_outside_and_writes_and_restores_method(self):
        original=Path.open;inside=self.root/subject.FIXTURE;outside=runner.ROOT/subject.FIXTURE
        with subject.read_only_scope(self.root):
            self.assertTrue(inside.read_bytes())
            with self.assertRaises(ValueError):outside.read_bytes()
            for mode in ('w','x','a','r+','wb'):
                with self.assertRaises(ValueError):inside.open(mode)
        self.assertIs(Path.open,original)

    def test_postflight_tree_refuses_content_extra_files_and_directories(self):
        path=self.root/subject.FIXTURE;original=path.read_bytes()
        try:
            path.write_bytes(original+b'\n// changed\n')
            with self.assertRaises(ValueError):subject.verify_tree(self.root,self.m)
        finally:path.write_bytes(original)
        extra=self.root/'unexpected';extra.mkdir()
        try:
            with self.assertRaises(ValueError):subject.verify_tree(self.root,self.m)
        finally:extra.rmdir()
        self.assertEqual(subject.verify_tree(self.root,self.m),self.archive['tree'])

    def test_output_no_clobber_deadlines_and_closed_real_gates(self):
        with patch.object(runner,'OUTPUT',self.generated),self.assertRaises(ValueError):runner.first(self.generated)
        self.assertEqual(self.result['original_windows_seconds'],[300,60,10])
        self.assertEqual([r['age_ns'] for r in self.result['first_refusals']],[10042390247,10010835739])
        self.assertIs(self.result['targeted_model_gate'],True);self.assertIs(self.result['prior_broad_isolation_gate'],False)
        for key in ('runtime_candidate_implemented','runtime_candidate_guard_implemented','clocks_changed','services_changed','server','product'):
            self.assertIs(self.result[key],False)
        self.assertEqual(self.result['model_images_executed'],0)


if __name__=='__main__':unittest.main(verbosity=2)
