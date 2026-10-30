"""Pure exact-manifest/caller checks, never Cargo/WSL/Git installation effects."""
import json,tomllib,unittest
import root_context as s
c=s.c

class Controls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):cls.raw=(s.ROOT/'Cargo.toml').read_bytes();cls.proposal=json.loads(s.PROPOSAL.read_bytes())
    def test_baseline_is_whole_actual_root(self):self.assertEqual(s.variant(self.raw,self.proposal,'B'),self.raw)
    def test_owned_manifest_reconstructs_shipping_proposal(self):
        raw=s.variant(self.raw,self.proposal,'O');shipping=raw.replace(b', registry = "hc074-draft"',b'')
        self.assertEqual(c.sha(shipping),self.proposal['after_sha256'])
    def test_only_chitchat_dependency_changes(self):
        old=tomllib.loads(self.raw.decode());new=tomllib.loads(s.variant(self.raw,self.proposal,'O').decode())
        before=old['workspace']['dependencies'].pop('chitchat');after=new['workspace']['dependencies'].pop('chitchat')
        self.assertEqual(old,new);self.assertEqual(before,'0.11.0');self.assertEqual(after,dict(package='hydra-chitchat',version='=0.11.1-hydra.1',registry='hc074-draft'))
    def test_original_manifest_cannot_drift(self):
        with self.assertRaises(ValueError):s.variant(self.raw+b'\n',self.proposal,'O')
    def test_wrong_shipping_digest_refuses(self):
        proposal=dict(self.proposal,after_sha256='0'*64)
        with self.assertRaises(ValueError):s.variant(self.raw,proposal,'O')
    def test_unknown_variant_refuses(self):
        with self.assertRaises(ValueError):s.variant(self.raw,self.proposal,'publish')
    def test_exact_four_caller_packages(self):
        self.assertEqual(set(s.PACKAGES),{'hydracache-cluster-chitchat','hydracache-cluster-testkit','hydracache-cluster-raft','hydracache-sandbox'})
    def test_all_actual_direct_users_still_inherit_alias(self):
        audit=s.abi.audit(s.ROOT);self.assertEqual(len(audit['direct']),4);self.assertEqual(len(audit['exposed']),3)
    def runner(self):return c.load(s.HERE/'runner.py','copied_root_pure_runner_control')
    def test_fixed_phase_parser_refuses_workspace_or_publish(self):
        r=self.runner();self.assertEqual(r.parse('check-O-188-adapter'),('check','O','188','adapter'))
        for value in ('workspace','publish','check-O-188-all','test-O-188-adapter-extra'):
            with self.assertRaises(ValueError):r.parse(value)
    def test_whole_default_catalog_accounts_ignored_explicitly(self):
        r=self.runner();value=r.test_outcome(b'running 3 tests\ntest result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out;\n')
        self.assertEqual((value['passed'],value['ignored'],value['targets']),(2,1,1))
    def test_filtered_or_partial_test_result_refuses(self):
        r=self.runner()
        for raw in (b'running 3 tests\n',b'running 3 tests\ntest result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out;\n'):
            with self.assertRaises(ValueError):r.test_outcome(raw)
    def test_lock_label_normalization_preserves_semantic_edges(self):
        prefix=[dict(name='zstd',version='0.14.3',source='registry+test',dependencies=['safe 8.2.0']),dict(name='safe',version='8.2.0',source='registry+test')]
        after=[dict(prefix[0],dependencies=['safe']),prefix[1]]
        self.assertEqual(s.lock_edges(prefix),s.lock_edges(after))
    def test_ambiguous_source_edge_refuses(self):
        rows=[dict(name='root',version='1',dependencies=['dep 1']),dict(name='dep',version='1',source='registry+a'),dict(name='dep',version='1',source='registry+b')]
        with self.assertRaises(ValueError):s.lock_edges(rows)

if __name__=='__main__':unittest.main(verbosity=2)
