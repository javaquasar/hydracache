"""Check final receipts and first reader failure without retained execution."""
import hashlib
import json
from pathlib import Path

INDEX = 'ed9d039acce32b7d9b93dfbf8d4a208ebd039193871381ba26fbdce5dba8c062'
MAIN = '4e83b67104ea30709616d7a9f65055c8b1c293900861f5c874796dd3166fe62f'


def verify(folder):
    def require(value, reason):
        if not value:
            raise ValueError(reason)
    sha = lambda raw: hashlib.sha256(raw).hexdigest()
    raw = (folder/'index.json').read_bytes()
    require(sha(raw) == INDEX, 'index differs')
    index = json.loads(raw)
    require(index['final_checks'] is True and index['checks'] == 3 and len(index['members']) == 19 and
            index['native_attempts'] == index['new_workload_calls'] == 0 and index['native_control_executed'] is False and
            all(index[k] is False for k in ('retry', 'overhead_accepted', 'package_pass', 'server')), 'scope differs')
    members = {}
    for name, entry in index['members'].items():
        raw = (folder/'objects'/entry['sha256']).read_bytes()
        require(len(raw) == entry['bytes'] and sha(raw) == entry['sha256'], 'raw member differs')
        members[name] = raw
    require(sha(members['main/index.json']) == MAIN, 'main index differs')
    for name in ('verify.py', 'test_verify.py'):
        require(members['main/'+name] == (folder.parent/name).read_bytes(), 'current verifier differs')
    require(sha(members['first-verifier.py']) == 'f7a55ed1fb2953ae4338b3f48508a5c415674317d76988292fb86e0f08f7b264',
            'first reader source differs')
    refusal = json.loads(members['first-verifier-refusal.json'])
    require(refusal['exit'] == 1 and refusal['error'] == "KeyError: 'git_calls'" and
            refusal['native_result_changed'] is refusal['native_retry'] is False, 'first reader failure differs')
    require(sha(members['qualification-manifest.toml']) ==
            '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc', 'qualification differs')
    for label in ('spawn-capture-integrity-first', 'spawn-capture-doc', 'spawn-capture-contract'):
        before, after = (json.loads(members['checks/'+label+'/'+name]) for name in ('intent.json', 'result.json'))
        require(before['head'] == index['source_commit'] and after['exit'] == 0 and after['outer_refusal'] is None and
                after['source_unchanged'] is after['head_unchanged'] is True and before['source'] == after['source_after'],
                'receipt differs')
    return dict(checks=3, passed=True, main_index_sha256=MAIN, native_attempts=0, new_workload_calls=0,
                retained_direct_reader_refusals=1, package_pass=False)


if __name__ == '__main__':
    print(json.dumps(verify(Path(__file__).resolve().parent), sort_keys=True))
