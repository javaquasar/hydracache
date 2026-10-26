"""Verify retained final check bytes only, never analysis/collector execution."""
import hashlib
import json
from pathlib import Path

INDEX = 'ae6f7b3dc03c8df53f4f60a669829173a5d0bc4c1806eda4c331c02acfb3cbbb'
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def verify(folder):
    raw = (folder/'index.json').read_bytes()
    require(sha(raw) == INDEX, 'index differs')
    index = json.loads(raw)
    require(index['checks'] == 5 and index['new_workload_calls'] == 0 and all(index[k] is False for k in
            ('workload_replayed', 'overhead_accepted', 'package_pass')) and
            sha((folder.parent/'index.json').read_bytes()) == index['analysis_index_sha256'] ==
            '055c7f306ad9dafb55b85a578ec788231a11f504e2117830991cdc0c5436a5b8', 'scope/parent differs')
    members = {}
    for name, entry in index['members'].items():
        body = (folder/'objects'/entry['sha256']).read_bytes()
        require(len(body) == entry['bytes'] and sha(body) == entry['sha256'], 'member differs')
        members[name] = body
    require(len(members) == 26, 'member count differs')
    for name in ('verify.py', 'test_verify.py'):
        require(members[name] == (folder.parent/name).read_bytes(), 'verifier source differs')
    require(sha(members['qualification-manifest.toml']) ==
            '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc', 'qualification changed')
    for name in members:
        if name.startswith('checks/') and name.endswith('/intent.json'):
            stem = name.removesuffix('intent.json')
            a, b = (json.loads(members[stem+part]) for part in ('intent.json', 'result.json'))
            require(b['exit'] == 0 and b['outer_refusal'] is None and b['source_unchanged'] is b['head_unchanged'] is True and
                    a['source'] == b['source_after'] and a['head'] == 'fa9b22330289ad0dacd05faf97310fa0b3b96336',
                    'receipt/source differs')
    for label in ('integrity-first', 'integrity-precommit'):
        log = members[f'checks/capture-attribution-{label}/stderr.log']
        require(b'Ran 5 tests' in log and log.rstrip().endswith(b'OK'), 'integrity verdict differs')
    require(b'doc-check: OK' in members['checks/capture-attribution-doc-first/stdout.log'] and
            b'OK (local, non-promotable)' in members['checks/capture-attribution-contract-first/stdout.log'], 'gate verdict differs')
    return dict(checks=5, raw_members=26, new_workload_calls=0, overhead_accepted=False, package_pass=False)


if __name__ == '__main__':
    print(verify(Path(__file__).resolve().parent))
