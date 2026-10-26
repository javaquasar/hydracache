"""Read sealed final receipts only; do not run retained source."""
import hashlib
import json
from pathlib import Path

INDEX = 'f422d27cc85068fe9e7556d24f06100c9e448bb25228b23020722edc5e7c7d28'
MAIN = 'a518934e04231820823c0535c5865c4fe5e19794694fc2b787ea6dc102a7de88'


def verify(folder):
    def require(value, reason):
        if not value:
            raise ValueError(reason)
    sha = lambda raw: hashlib.sha256(raw).hexdigest()
    raw = (folder/'index.json').read_bytes()
    require(sha(raw) == INDEX, 'index differs')
    index = json.loads(raw)
    require(index['final_checks'] is True and index['checks'] == 4 and len(index['members']) == 21 and
            index['native_attempts'] == index['new_workload_calls'] == 0 and all(index[k] is False for k in
            ('caller_connected', 'native_control_executed', 'overhead_accepted', 'package_pass', 'server')), 'scope differs')
    members = {}
    for name, entry in index['members'].items():
        body = (folder/'objects'/entry['sha256']).read_bytes()
        require(len(body) == entry['bytes'] and sha(body) == entry['sha256'], 'raw member differs')
        members[name] = body
    require(sha(members['main/index.json']) == MAIN, 'main index differs')
    for name in ('verify.py', 'test_verify.py'):
        require(members['main/'+name] == (folder.parent/name).read_bytes(), 'current verifier differs')
    require(sha(members['qualification-manifest.toml']) ==
            '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc', 'qualification differs')
    for label in ('spawn-frames-integrity-first', 'spawn-frames-integrity-final', 'spawn-frames-doc', 'spawn-frames-contract'):
        before, after = (json.loads(members['checks/'+label+'/'+name]) for name in ('intent.json', 'result.json'))
        require(before['head'] == index['source_commit'] and after['exit'] == 0 and after['outer_refusal'] is None and
                after['source_unchanged'] is after['head_unchanged'] is True and before['source'] == after['source_after'],
                'check receipt differs')
    return dict(checks=4, passed=True, main_index_sha256=MAIN, native_attempts=0, new_workload_calls=0, package_pass=False)


if __name__ == '__main__':
    print(json.dumps(verify(Path(__file__).resolve().parent), sort_keys=True))
