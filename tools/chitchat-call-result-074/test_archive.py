import copy
import json
import unittest
import verify as v


class Archive(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.index=json.loads((v.OUT/'index.json').read_bytes())
        cls.bodies={row['sha256']:(v.OUT/'objects'/row['sha256']).read_bytes() for row in cls.index['members'].values()}

    def mutate(self,name,fn):
        index=copy.deepcopy(self.index);bodies=dict(self.bodies);row=index['members'][name]
        raw=fn(bodies[row['sha256']]);digest=v.r.sha(raw);bodies[digest]=raw
        index['members'][name]=dict(sha256=digest,bytes=len(raw));return index,bodies

    def test_whole_packet_and_negative_old_contracts_are_not_a_release(self):
        result=v.validate(self.index,self.bodies.__getitem__)
        self.assertTrue(result['prerequisite_passed'])
        self.assertFalse(result['compatibility']['mtu_gate_passed'] or result['release_admitted'])

    def test_corrupted_bytes_and_scope_promotion_refuse(self):
        bodies=dict(self.bodies);key=next(iter(bodies));bodies[key]+=b'corruption'
        with self.assertRaises(ValueError):v.validate(self.index,bodies.__getitem__)
        for name in ('root_adopted','release_admitted','mtu_repaired','full_resource_contract_passed'):
            index=copy.deepcopy(self.index);index[name]=True
            with self.assertRaises(ValueError):v.validate(index,self.bodies.__getitem__)

    def test_coherent_incomplete_owner_and_cached_baseline_refuse(self):
        def incomplete(raw):
            value=json.loads(raw);value['retired']=False;return json.dumps(value).encode()
        index,bodies=self.mutate('native-r1/observe-C2/execution.json',incomplete)
        with self.assertRaises(ValueError):v.validate(index,bodies.__getitem__)
        row=self.index['members']['native-r1/image-B0.json'];raw=self.bodies[row['sha256']]
        index,bodies=self.mutate('native-r1/image-C2.json',lambda _:raw)
        with self.assertRaises(ValueError):v.validate(index,bodies.__getitem__)

    def test_coherent_dropped_negative_or_property_case_refuses(self):
        def without_negative(raw):
            value=json.loads(raw);value['negative']=value['negative'][1:];return json.dumps(value).encode()
        index,bodies=self.mutate('semantic/gnu-C2-188/catalog.json',without_negative)
        with self.assertRaises(ValueError):v.validate(index,bodies.__getitem__)
        def without_property(raw):
            value=json.loads(raw);value['selected']=[n for n in value['selected'] if 'proptest' not in n];return json.dumps(value).encode()
        index,bodies=self.mutate('semantic/gnu-C2-194/catalog.json',without_property)
        with self.assertRaises(ValueError):v.validate(index,bodies.__getitem__)

    def test_coherent_second_functional_source_change_refuses(self):
        index,bodies=self.mutate('fixtures/C2/src/serialize.rs',lambda b:b.replace(b'Compressor::new(0)',b'Compressor::new(1)'))
        with self.assertRaises(ValueError):v.validate(index,bodies.__getitem__)


if __name__=='__main__':unittest.main()
