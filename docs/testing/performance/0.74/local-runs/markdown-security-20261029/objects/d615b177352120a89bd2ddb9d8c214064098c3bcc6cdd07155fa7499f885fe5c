import copy
import json
from pathlib import Path
import tarfile
import tomllib
import unittest

import feature_review as f

ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/'target/dbus-native-20261028'


def inputs():
    manifests=[]
    for name in ('zbus-5.19.0','tokio-1.53.3'):
        with tarfile.open(BASE/'input/archives'/(name+'.crate'),'r:gz') as tar:
            manifests.append(tar.extractfile(name+'/Cargo.toml').read())
    manifests += [(ROOT/'target/dbus-legacy-integration-20261028/tests/input/fd/source/zbus/Cargo.toml').read_bytes(),
                  (BASE/'input/fd/source/zbus/Cargo.toml').read_bytes()]
    return (BASE/'input/fd/Cargo.lock').read_bytes(),(BASE/'session/Cargo.lock').read_bytes(),manifests


def encode(data):
    text='version = 4\n'
    for row in data['package']:
        text+='\n[[package]]\n'+''.join(k+' = '+json.dumps(v)+'\n' for k,v in row.items())
    return text.encode()


class FeatureReview(unittest.TestCase):
    def test_actual_saved_graph_changes_only_two_source_proven_edges(self):
        before,after,manifests=inputs()
        result=f.review(before,after,*manifests)
        self.assertEqual((result['old_packages'],result['new_packages']),(156,135))
        self.assertEqual(len(result['removed']),21)
        self.assertFalse(result['new_cargo_run'])

    def test_whole_source_or_candidate_feature_mutation_refuses(self):
        before,after,manifests=inputs()
        for i in range(4):
            changed=list(manifests)
            changed[i]+=b'\n'
            with self.subTest(manifest=i), self.assertRaises(ValueError):
                f.review(before,after,*changed)

    def test_unrelated_edge_checksum_old_backend_or_added_package_cannot_hide(self):
        before,after,manifests=inputs()
        parsed=tomllib.loads(after.decode())
        for case in ('edge','checksum','old','added'):
            data=copy.deepcopy(parsed)
            if case=='edge': next(r for r in data['package'] if r['name']=='tokio')['dependencies'].pop()
            if case=='checksum': next(r for r in data['package'] if 'checksum' in r)['checksum']='0'*64
            if case=='old': data['package'].append(next(r for r in tomllib.loads(before.decode())['package'] if r['name']=='zbus' and r['version']=='5.19.0'))
            if case=='added': data['package'].append(dict(name='foreign',version='1.0.0'))
            with self.subTest(case=case), self.assertRaises(ValueError):
                f.review(before,encode(data),*manifests)


if __name__ == '__main__':
    unittest.main()
