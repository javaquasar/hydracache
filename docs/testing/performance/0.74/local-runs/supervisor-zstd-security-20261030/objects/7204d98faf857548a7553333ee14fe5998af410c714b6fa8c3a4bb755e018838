import copy
import json
import unittest
import verify as v

class StoredRefusals(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.index=json.loads((v.OUT/'index.json').read_bytes())
        cls.raw={r['sha256']:(v.OUT/'objects'/r['sha256']).read_bytes() for r in cls.index['members'].values()}

    def test_complete_packet(self):v.validate(self.index,self.raw.__getitem__)

    def test_body_and_length_tampering_refuse(self):
        for name in ('candidate/Cargo.lock','capture/policy/root/stderr.log','capture/gnu/candidate-archive-0-188/stdout.log'):
            raw=dict(self.raw);digest=self.index['members'][name]['sha256'];raw[digest]+=b'\n'
            with self.subTest(name=name),self.assertRaises(ValueError):v.validate(self.index,raw.__getitem__)

    def test_claim_promotions_refuse_even_with_rehashed_analysis(self):
        for field in ('workspace_gate_passed','release_admitted','q_adopted','q_cpu_gate_passed','standalone_wire_is_exact_root_graph','windows_full_formatter_passed'):
            index=copy.deepcopy(self.index);raw=dict(self.raw);index['result'][field]=True
            body=(json.dumps(index['result'],indent=2,sort_keys=True)+'\n').encode();digest=v.c.sha(body)
            raw[digest]=body;index['members']['analysis.json']=dict(sha256=digest,bytes=len(body))
            with self.subTest(field=field),self.assertRaises(ValueError):v.validate(index,raw.__getitem__)

if __name__=='__main__':unittest.main()
