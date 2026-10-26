"""Check sealed final receipts without running retained code."""
import hashlib
import json
from pathlib import Path

INDEX = '1295bed7516ff198b4cf4eb85cc435acef99e203ed0d57700624fbd2056eeebb'
MAIN = '70ba30460b8e4c995c23d3f37090b0ef849bc2915e6507e62dfc5b23a7837417'


def verify(folder):
    def require(value, reason):
        if not value:
            raise ValueError(reason)
    sha = lambda raw: hashlib.sha256(raw).hexdigest()
    raw = (folder/'index.json').read_bytes()
    require(sha(raw) == INDEX, 'index differs')
    index = json.loads(raw)
    require(index['final_checks'] is True and index['checks'] == 3 and len(index['members']) == 17 and
            index['new_workload_calls'] == index['new_native_attempts'] == 0 and
            all(index[k] is False for k in ('workload_replayed', 'overhead_accepted', 'package_pass', 'server')), 'scope differs')
    members = {}
    for name, entry in index['members'].items():
        raw = (folder/'objects'/entry['sha256']).read_bytes()
        require(len(raw) == entry['bytes'] and sha(raw) == entry['sha256'], 'raw member differs')
        members[name] = raw
    require(sha(members['main/index.json']) == MAIN, 'main packet differs')
    for name in ('verify.py', 'test_verify.py'):
        require(members['main/'+name] == (folder.parent/name).read_bytes(), 'current verifier differs')
    require(sha(members['qualification-manifest.toml']) ==
            '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc', 'qualification differs')
    for label in ('leaf-attribution-integrity-first', 'leaf-attribution-doc', 'leaf-attribution-contract'):
        before, after = (json.loads(members['checks/'+label+'/'+name]) for name in ('intent.json', 'result.json'))
        require(before['head'] == index['source_commit'] and after['exit'] == 0 and after['outer_refusal'] is None and
                after['head_unchanged'] is after['source_unchanged'] is True and before['source'] == after['source_after'],
                'check receipt differs')
    return dict(checks=3, passed=True, main_index_sha256=MAIN, new_workload_calls=0, package_pass=False)


if __name__ == '__main__':
    print(json.dumps(verify(Path(__file__).resolve().parent), sort_keys=True))
