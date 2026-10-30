import copy
import json
import unittest
import verify as v

class DataOnlyRefusals(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.index=json.loads((v.OUT/'index.json').read_bytes())
        cls.raw={r['sha256']:(v.OUT/'objects'/r['sha256']).read_bytes() for r in cls.index['members'].values()}

    def test_complete_source_policy_and_case_replay(self):v.validate(self.index,self.raw.__getitem__)

    def test_corrupt_archive_and_log_bytes_refuse(self):
        for name in ('root/Cargo.lock','capture/r4/input/archives/chitchat-0.11.1.crate','capture/r4/policy/U/stderr.log'):
            data=dict(self.raw);digest=self.index['members'][name]['sha256'];data[digest]+=b'\n'
            with self.subTest(name=name),self.assertRaises(ValueError):v.validate(self.index,data.__getitem__)

    def test_coherently_rehashed_owner_and_selection_tampering_refuse(self):
        base='capture/r4/peers-r4/windows-194-U/'
        for name,mutate in ((base+'execution.json',lambda x:x.update(retired=False)),
           (base+'catalog.json',lambda x:x['selected'].remove('serialize::tests::test_proptest_compressed_stream')),
           (base+'catalog.json',lambda x:x.update(negative_contracts=[]))):
            index=copy.deepcopy(self.index);data=dict(self.raw);row=index['members'][name]
            value=json.loads(data[row['sha256']]);mutate(value);body=(json.dumps(value,sort_keys=True)+'\n').encode();digest=v.c.sha(body)
            data[digest]=body;index['members'][name]=dict(sha256=digest,bytes=len(body))
            with self.subTest(name=name),self.assertRaises(ValueError):v.validate(index,data.__getitem__)

    def test_no_root_or_wide_gate_promotion(self):
        for field in ('root_adopted','release_admitted'):
            index=copy.deepcopy(self.index);index[field]=True
            with self.subTest(field=field),self.assertRaises(ValueError):v.validate(index,self.raw.__getitem__)
        index=copy.deepcopy(self.index);index['result']['wide_gate_passed']=True
        with self.assertRaises(ValueError):v.validate(index,self.raw.__getitem__)

if __name__=='__main__':unittest.main()
