import copy
import json
import unittest
import verify as v


class Retained(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.index=json.loads((v.OUT/'index.json').read_bytes())
        cls.bodies={r['sha256']:(v.OUT/'objects'/r['sha256']).read_bytes() for r in cls.index['members'].values()}

    def modified(self,name,change):
        index=copy.deepcopy(self.index);bodies=dict(self.bodies);row=index['members'][name]
        raw=change(bodies[row['sha256']]);digest=v.a.hashlib.sha256(raw).hexdigest()
        bodies[digest]=raw;index['members'][name]=dict(sha256=digest,bytes=len(raw))
        return index,bodies

    def test_whole_packet_replays_without_archived_execution(self):
        result=v.validate(self.index,self.bodies.__getitem__)
        self.assertTrue(result['complete'])
        self.assertFalse(result['root_adopted'] or result['release_admitted'])

    def test_raw_corruption_and_scope_promotion_refuse(self):
        bodies=dict(self.bodies);digest=next(iter(bodies));bodies[digest]+=b'corrupt'
        with self.assertRaises(ValueError):v.validate(self.index,bodies.__getitem__)
        for field in ('root_adopted','release_admitted','mtu_repaired','archived_code_executed'):
            index=copy.deepcopy(self.index);index[field]=True
            with self.assertRaises(ValueError):v.validate(index,self.bodies.__getitem__)

    def test_coherently_rehashed_incomplete_owner_refuses(self):
        def change(raw):
            value=json.loads(raw);value['retired']=False
            return json.dumps(value).encode()
        index,bodies=self.modified('native-r5/observe-C1/execution.json',change)
        with self.assertRaises(ValueError):v.validate(index,bodies.__getitem__)

    def test_coherently_rehashed_second_source_hypothesis_refuses(self):
        index,bodies=self.modified('fixtures/C1/src/serialize.rs',lambda b:b.replace(b'Compressor::new(0)',b'Compressor::new(1)'))
        with self.assertRaises(ValueError):v.validate(index,bodies.__getitem__)

    def test_coherently_rehashed_cached_baseline_as_candidate_refuses(self):
        row=self.index['members']['native-r5/image-B0.json']
        raw=self.bodies[row['sha256']]
        index,bodies=self.modified('native-r5/image-C1.json',lambda _:raw)
        with self.assertRaises(ValueError):v.validate(index,bodies.__getitem__)

    def test_negative_result_cannot_be_promoted(self):
        index=copy.deepcopy(self.index);index['prerequisite_passed']=not index['prerequisite_passed']
        with self.assertRaises(ValueError):v.validate(index,self.bodies.__getitem__)


if __name__=='__main__':unittest.main()
