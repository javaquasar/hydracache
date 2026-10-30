"""Pure corruption controls for retained Cargo semantic evidence."""
import copy,json,unittest
import review as r

REG='registry+file:///qualified-private-registry'

def encode(rows):
    text='version = 4\n'
    for row in rows:
        text+='\n[[package]]\n'
        for name,value in row.items():text+=name+' = '+json.dumps(value)+'\n'
    return text.encode()

class Controls(unittest.TestCase):
    def fixture(self):
        callers=[dict(name=n,version='0.73.0',dependencies=['chitchat']) for n in r.PACKAGES.values()]
        old=callers+[dict(name='chitchat',version='0.11.1',source=r.CRATES,checksum='a'*64,dependencies=['lru 0.17.0']),dict(name='lru',version='0.17.0',source=r.CRATES,checksum='b'*64)]
        new=[dict(p,dependencies=[r.OWNED]) for p in callers]+[dict(name=r.OWNED,version=r.VERSION,source=REG,checksum=r.CRATE_SHA,dependencies=['lru 0.18.2']),dict(name='lru',version='0.18.2',source=r.CRATES,checksum='c'*64)]
        return old,new,{'chitchat-0.11.1':'a'*64,'lru-0.17.0':'b'*64,'lru-0.18.2':'c'*64}
    def test_exact_coordinated_lock_substitution(self):
        old,new,ledger=self.fixture();v=r.lock_review(encode(old),encode(new),ledger,REG)
        self.assertEqual({n for n,_,_ in v['added']},{r.OWNED,'lru'})
    def test_wrong_owned_checksum_refuses(self):
        old,new,ledger=self.fixture();new[-2]['checksum']='0'*64
        with self.assertRaises(ValueError):r.lock_review(encode(old),encode(new),ledger,REG)
    def test_wrong_registry_refuses(self):
        old,new,ledger=self.fixture();new[-2]['source']=r.CRATES
        with self.assertRaises(ValueError):r.lock_review(encode(old),encode(new),ledger,REG)
    def test_unrelated_node_change_refuses(self):
        old,new,ledger=self.fixture();old.append(dict(name='other',version='1'));new.append(dict(name='other',version='2'))
        with self.assertRaises(ValueError):r.lock_review(encode(old),encode(new),ledger,REG)
    def test_unrelated_retained_edge_change_refuses(self):
        old,new,ledger=self.fixture();old.append(dict(name='other',version='1',dependencies=['chitchat']));new.append(dict(name='other',version='1',dependencies=[r.OWNED]))
        with self.assertRaises(ValueError):r.lock_review(encode(old),encode(new),ledger,REG)
    def test_retained_whole_node_metadata_change_refuses(self):
        old,new,ledger=self.fixture();new[0]['version']='0.74.0'
        with self.assertRaises(ValueError):r.lock_review(encode(old),encode(new),ledger,REG)
    def test_ambiguous_edge_refuses(self):
        with self.assertRaises(ValueError):r.edges([dict(name='root',version='1',dependencies=['dep']),dict(name='dep',version='1'),dict(name='dep',version='2')])
    def test_duplicate_node_refuses(self):
        with self.assertRaises(ValueError):r.edges([dict(name='root',version='1'),dict(name='root',version='1')])
    def test_label_normalization_preserves_edges(self):
        rows=[dict(name='root',version='1',dependencies=['dep 2']),dict(name='dep',version='2')]
        after=copy.deepcopy(rows);after[0]['dependencies']=['dep'];self.assertEqual(r.edges(rows),r.edges(after))
    def meta(self,v):
        selected=dict(name='chitchat' if v=='B' else r.OWNED,version='0.11.1' if v=='B' else r.VERSION,source=r.CRATES if v=='B' else REG,id='selected-normal-package')
        names=[dict(name=n,id=n,manifest_path=r.ROOT+'/copies/'+v+'/crates/'+n+'/Cargo.toml') for n in r.PACKAGES.values()]
        return dict(workspace_root=r.ROOT+'/copies/'+v,packages=names+[selected],resolve=dict(nodes=[dict(id=p['id'],deps=[dict(name='chitchat',pkg=selected['id'])]) for p in names]))
    def test_both_actual_alias_shapes_are_supported(self):
        for v in ('B','O'):self.assertEqual(len(r.aliases(self.meta(v),v,REG)['callers']),4)
    def test_one_mixed_alias_refuses(self):
        d=self.meta('O');d['resolve']['nodes'][0]['deps'][0]['pkg']='legacy'
        with self.assertRaises(ValueError):r.aliases(d,'O',REG)
    def test_dual_legacy_graph_refuses(self):
        d=self.meta('O');d['packages'].append(dict(name='chitchat',id='legacy'))
        with self.assertRaises(ValueError):r.aliases(d,'O',REG)
    def test_missing_direct_caller_refuses(self):
        d=self.meta('O');d['packages'].pop(0)
        with self.assertRaises(ValueError):r.aliases(d,'O',REG)
    def test_mock_manifest_path_refuses(self):
        d=self.meta('O');d['packages'][0]['manifest_path']='/tmp/mock/Cargo.toml'
        with self.assertRaises(ValueError):r.aliases(d,'O',REG)
    def test_wrong_version_refuses(self):
        d=self.meta('O');d['packages'][-1]['version']='next'
        with self.assertRaises(ValueError):r.aliases(d,'O',REG)
    def test_empty_data_cannot_admit_matrix_or_product(self):
        d=r.reconcile({});self.assertFalse(d['metadata_complete']);self.assertFalse(d['full_focused_matrix_complete']);self.assertFalse(d['root_adopted']);self.assertFalse(d['release_admitted'])

if __name__=='__main__':unittest.main(verbosity=2)
